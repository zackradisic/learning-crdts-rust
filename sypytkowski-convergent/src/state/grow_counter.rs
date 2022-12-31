use std::{
    collections::{btree_map::Entry, BTreeMap},
    ops::Deref,
};

use crate::ReplicaId;

#[derive(Clone, Debug, PartialEq)]
pub struct GrowCounter {
    map: BTreeMap<ReplicaId, i64>,
}

impl GrowCounter {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    pub fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (ReplicaId, i64)>,
    {
        Self {
            map: BTreeMap::<ReplicaId, i64>::from_iter(iter),
        }
    }

    pub fn value(&self) -> i64 {
        self.map.values().fold(0, |acc, val| acc + val)
    }

    pub fn increment(&mut self, replica: ReplicaId) -> i64 {
        *self.map.entry(replica).or_insert(0)
    }

    pub fn merge(&self, other: &Self) -> GrowCounter {
        self.map
            .iter()
            .fold(other.clone(), |mut acc, (&key, &value)| {
                match acc.map.entry(key) {
                    Entry::Vacant(entry) => {
                        entry.insert(value);
                    }
                    Entry::Occupied(mut entry) => {
                        let new = entry.get().max(&value);
                        entry.insert(*new);
                    }
                };
                acc
            })
    }

    pub fn from_u64_map(map: BTreeMap<u64, i64>) -> Self {
        Self {
            map: map.into_iter().map(|(k, v)| (ReplicaId(k), v)).collect(),
        }
    }
}

impl Deref for GrowCounter {
    type Target = BTreeMap<ReplicaId, i64>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl Default for GrowCounter {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use proptest::prelude::*;

    use crate::state::grow_counter::GrowCounter;

    proptest! {
        #[test]
        fn commutativity(mut a: BTreeMap<u64, i64>, mut b: BTreeMap<u64, i64>) {
            let a = GrowCounter::from_u64_map(a);
            let b = GrowCounter::from_u64_map(b);

            let left_to_right = a.merge(&b);
            let right_to_left = b.merge(&a);

            assert_eq!(left_to_right, right_to_left)
        }

        #[test]
        fn associativity(mut a: BTreeMap<u64, i64>, mut b: BTreeMap<u64, i64>, mut c: BTreeMap<u64, i64>) {
            let a = GrowCounter::from_u64_map(a);
            let b = GrowCounter::from_u64_map(b);
            let c = GrowCounter::from_u64_map(c);

            let ab_c = a.merge(&b).merge(&c);
            let a_bc = a.merge(&b.merge(&c));

            assert_eq!(ab_c, a_bc)
        }

        #[test]
        fn idempotency(mut a: BTreeMap<u64, i64>) {
            let a = GrowCounter::from_u64_map(a);
            let result = a.merge(&a);
            assert_eq!(a, result)
        }
    }
}
