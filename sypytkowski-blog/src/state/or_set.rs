use std::collections::BTreeMap;

use crate::ReplicaId;

use super::vector_clock::VectorClock;

/// # Observed Remove Set
/// allows insertion/deletion of a set while still being able
/// to converge when merging replicas from different locations
///
/// this is achieved by maintaining an `add` and `remove` map where the keys
/// is the actual value, and the values of the map are the vector clock signifying
/// the time the value was added/remove.
pub struct ORSet<T: Ord + Clone> {
    add: BTreeMap<T, VectorClock>,
    rem: BTreeMap<T, VectorClock>,
}

impl<T: Ord + Clone> ORSet<T> {
    pub fn new(add: BTreeMap<T, VectorClock>, rem: BTreeMap<T, VectorClock>) -> Self {
        Self { add, rem }
    }

    pub fn value(&self) -> BTreeMap<T, VectorClock> {
        // Remove every key of `rem` from `add` if the
        // deletion time happens after the insertion time
        self.rem
            .iter()
            .fold(self.add.clone(), |mut acc, (key, del_time)| {
                match acc.get(key) {
                    Some(a_time) if a_time < del_time => {
                        acc.remove(key);
                    }
                    _ => (),
                };
                acc
            })
    }

    pub fn add(&mut self, replica: ReplicaId, val: T) {
        match (self.add.get_mut(&val), self.rem.get_mut(&val)) {
            (Some(clock), None) => {
                clock.increment(replica);
                self.rem.remove(&val);
            }
            (None, Some(clock)) => {
                clock.increment(replica);
                let clock_clone = clock.clone();
                self.rem.remove(&val);
                self.add.insert(val, clock_clone);
            }
            (_, _) => {
                let mut clock = VectorClock::default();
                clock.increment(replica);
                self.add.insert(val, clock);
            }
        }
    }

    pub fn remove(&mut self, replica: ReplicaId, val: T) {
        match (self.add.get_mut(&val), self.rem.get_mut(&val)) {
            (Some(_), None) => {
                let mut clock = self.add.remove(&val).unwrap();
                clock.increment(replica);
                self.rem.insert(val, clock);
            }
            (None, Some(rem_clock)) => {
                self.add.remove(&val);
                rem_clock.increment(replica);
            }
            _ => {
                let mut clock = VectorClock::default();
                clock.increment(replica);
                self.rem.insert(val, clock);
            }
        }
    }

    pub fn merge(&self, other: &Self) -> Self {
        let add_keys_merged = Self::merge_keys(&self.add, &other.add);
        let rem_keys_merged = Self::merge_keys(&self.rem, &other.rem);

        let add =
            rem_keys_merged
                .iter()
                .fold(add_keys_merged.clone(), |mut acc, (value, clock)| {
                    match acc.get(value) {
                        Some(add_clock) if add_clock < clock => {
                            acc.remove(value);
                        }
                        _ => (),
                    };
                    acc
                });

        let rem =
            add_keys_merged
                .iter()
                .fold(rem_keys_merged.clone(), |mut acc, (value, add_clock)| {
                    match acc.get(value) {
                        Some(clock) if add_clock < clock => {
                            acc.remove(value);
                        }
                        _ => (),
                    };
                    acc
                });

        Self { add, rem }
    }

    fn merge_keys(
        a: &BTreeMap<T, VectorClock>,
        b: &BTreeMap<T, VectorClock>,
    ) -> BTreeMap<T, VectorClock> {
        b.iter().fold(a.clone(), |mut acc, (value, clock)| {
            match acc.get_mut(&value) {
                Some(a_clock) => {
                    let merged = a_clock.merge(clock);
                    *a_clock = merged;
                }
                None => {
                    acc.insert(value.clone(), clock.clone());
                }
            };
            acc
        })
    }
}

impl<T: Ord + Clone> Default for ORSet<T> {
    fn default() -> Self {
        Self {
            add: Default::default(),
            rem: Default::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ReplicaGenerator;

    use super::ORSet;

    #[test]
    fn add() {
        let mut gen = ReplicaGenerator::new();

        let alice = gen.gen();
        let bob = gen.gen();

        let mut orset = ORSet::<u64>::default();
        orset.add(alice, 420);

        let val = orset.value();
        assert!(val.contains_key(&420));

        orset.add(bob, 420);
        let val = orset.value();
        assert!(val.contains_key(&420));
        let clock = val.get(&420).expect("Clock should be defined");
        assert_eq!(clock.get(&alice), Some(&0));
        assert_eq!(clock.get(&bob), Some(&0));
    }

    #[test]
    fn add_merge() {
        let mut gen = ReplicaGenerator::new();

        let alice = gen.gen();
        let bob = gen.gen();

        let mut orset_alice = ORSet::<u64>::default();
        let mut orset_bob = ORSet::<u64>::default();
        orset_alice.add(alice, 420);
        orset_bob.add(bob, 69);

        let merged_orset = orset_alice.merge(&orset_bob);

        let val = merged_orset.value();
        assert!(val.contains_key(&420));
        assert!(val.contains_key(&69));
    }

    #[test]
    fn remove() {
        let mut gen = ReplicaGenerator::new();

        let alice = gen.gen();
        let bob = gen.gen();

        let mut orset = ORSet::<u64>::default();
        orset.add(alice, 420);
        orset.add(alice, 69);

        let val = orset.value();
        assert!(val.contains_key(&420));

        orset.remove(bob, 420);

        let val = orset.value();
        assert!(val.contains_key(&69));
        assert!(!val.contains_key(&420));

        let clock = val.get(&69).expect("Clock should be defined");
        assert_eq!(clock.get(&alice), Some(&0));
    }

    #[test]
    fn remove_add_concurrent() {
        let mut gen = ReplicaGenerator::new();

        let alice = gen.gen();
        let bob = gen.gen();

        let mut orset_alice = ORSet::<u64>::default();
        let mut orset_bob = ORSet::<u64>::default();

        orset_bob.remove(bob, 420);
        orset_alice.add(alice, 420);

        let merged_orset = orset_alice.merge(&orset_bob);
        let val = merged_orset.value();
        assert!(val.contains_key(&420));
    }
}
