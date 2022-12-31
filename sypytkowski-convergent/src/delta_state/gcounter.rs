use std::collections::{btree_map::Entry, BTreeMap};

use crate::ReplicaId;

/// Note that the deltas are in a GCounter struct for composability reasons
#[derive(Debug, Clone, PartialEq)]
pub struct GCounter {
    values: BTreeMap<ReplicaId, i64>,
    delta: Option<Box<GCounter>>,
}

impl Default for GCounter {
    fn default() -> Self {
        Self {
            values: Default::default(),
            delta: None,
        }
    }
}

impl GCounter {
    pub fn deltas(&self) -> Option<&GCounter> {
        self.delta.as_deref()
    }

    /// Compute value of the G-counter
    pub fn value(&self) -> i64 {
        self.values.values().fold(0, |acc, i| acc + i)
    }

    /// Increment G-counter value for a given replica.
    pub fn increment(&mut self, replica: ReplicaId) {
        *self.values.entry(replica).or_default() += 1;
        *self
            .delta
            .get_or_insert_default()
            .values
            .entry(replica)
            .or_default() += 1;
    }

    /// Merge two G-counters.
    pub fn merge_impl(a: &Self, b: &Self) -> Self {
        let values = a
            .values
            .iter()
            .fold(b.values.clone(), |mut acc, (&replica, &val)| {
                match acc.entry(replica) {
                    Entry::Vacant(entry) => {
                        entry.insert(val);
                    }
                    Entry::Occupied(mut entry) => {
                        entry.insert(val.max(*entry.get()));
                    }
                };
                acc
            });

        let delta = match (&a.delta, &b.delta) {
            (Some(x), Some(y)) => Some(Box::new(Self::merge_impl(&x, &y))),
            (Some(x), None) => Some(x.clone()),
            (None, Some(y)) => Some(y.clone()),
            (None, None) => None,
        };

        Self { values, delta }
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self::merge_impl(&self, other)
    }

    /// Merge full-state G-counter with G-counter delta.
    pub fn merge_deltas(&self, delta: &GCounter) -> Self {
        Self::merge_impl(self, delta)
    }

    /// Split G-counter into full-state G-counter with empty delta, and a delta itself.
    pub fn split(&self) -> (Self, Option<Box<GCounter>>) {
        (
            Self {
                values: self.values.clone(),
                delta: None,
            },
            self.delta.clone(),
        )
    }

    pub fn split_owned(self) -> (Self, Option<Box<GCounter>>) {
        (
            Self {
                values: self.values,
                delta: None,
            },
            self.delta,
        )
    }

    pub fn split_expect(&self) -> (Self, Box<GCounter>) {
        let (map, deltas) = self.split();
        (map, deltas.expect("Expected deltas"))
    }

    pub fn from_u64_map(map: BTreeMap<u64, u8>) -> Self {
        let mut this = Self::default();
        for (k, v) in map {
            let v = v.clamp(1, u8::MAX);
            for _ in 0..v {
                this.increment(ReplicaId(k))
            }
        }
        this
    }
}

#[cfg(test)]
pub mod test {

    use proptest::{collection::btree_map, prelude::*};

    use crate::delta_state::gcounter::GCounter;

    pub fn gcounter_strategy() -> impl Strategy<Value = GCounter> {
        btree_map(any::<u64>(), any::<u8>(), 10).prop_map(GCounter::from_u64_map)
    }

    proptest! {
        // #![proptest_config(ProptestConfig{ cases: 5, ..Default::default()})]
        #![proptest_config(ProptestConfig{ ..Default::default()})]

        #[test]
        fn commutativity(a in gcounter_strategy(), b in gcounter_strategy()) {

            let ab = a.merge(&b);
            let ba = b.merge(&a);


            assert_eq!(ab, ba)
        }

        #[test]
        fn associativity(a in gcounter_strategy(), b in gcounter_strategy(), c in gcounter_strategy()) {
            let ab_c = a.merge(&b).merge(&c);
            let bc = b.merge(&c);
            let a_bc = a.merge(&bc);

            assert_eq!(ab_c, a_bc)
        }

        #[test]
        fn idempotency(a in gcounter_strategy()) {
            assert_eq!(a, a.merge(&a))
        }
    }

    mod deltas {
        use proptest::prelude::*;

        use crate::delta_state::gcounter::{test::gcounter_strategy, GCounter};

        proptest! {
            // #![proptest_config(ProptestConfig{ cases: 5, ..Default::default()})]
            #![proptest_config(ProptestConfig{ ..Default::default()})]

            #[test]
            fn commutativity(a in gcounter_strategy(), b in gcounter_strategy()) {
                let (a, a_deltas) = a.split_expect();
                let (b, b_deltas) = b.split_expect();

                let ab = a.merge_deltas(&b_deltas);
                let ba = b.merge_deltas(&a_deltas);

                let result_ab = GCounter::default().merge(&ab);
                let result_ba = GCounter::default().merge(&ba);

                assert_eq!(result_ab, result_ba)
            }

            #[test]
            fn associativity(a in gcounter_strategy(), b in gcounter_strategy(), c in gcounter_strategy()) {
                let a_deltas = a.deltas().expect("Deltas should be defined");
                let b_deltas = b.deltas().expect("Deltas should be defined");
                let c_deltas = c.deltas().expect("Deltas should be defined");

                let ab_c = a_deltas.merge(&b_deltas).merge(&c_deltas);
                let bc = b_deltas.merge(&c_deltas);
                let a_bc = a_deltas.merge(&bc);

                let result_ab_c = GCounter::default().merge(&ab_c);
                let result_a_bc = GCounter::default().merge(&a_bc);

                assert_eq!(result_ab_c, result_a_bc)
            }

            #[test]
            fn idempotency(a in gcounter_strategy()) {
                let a = a.deltas().expect("Deltas should be defined");
                let result = GCounter::default().merge(a);
                let result_idempotent = GCounter::default().merge(&a.merge(&a));
                assert_eq!(result, result_idempotent)
            }
        }
    }
}
