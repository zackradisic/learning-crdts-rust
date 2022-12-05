use crate::{grow_counter::GrowCounter, ReplicaId};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct PNCounter {
    incr: GrowCounter,
    decr: GrowCounter,
}

impl PNCounter {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_from_incr_decr(incr: GrowCounter, decr: GrowCounter) -> Self {
        Self { incr, decr }
    }

    pub fn value(&self) -> i64 {
        self.incr.value() - self.decr.value()
    }

    pub fn increment(&mut self, replica: ReplicaId) {
        self.incr.increment(replica);
    }

    pub fn decrement(&mut self, replica: ReplicaId) {
        self.decr.increment(replica);
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self {
            incr: self.incr.merge(&other.incr),
            decr: self.decr.merge(&other.decr),
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use proptest::prelude::*;

    use crate::{grow_counter::GrowCounter, pn_counter::PNCounter};

    proptest! {
        #[test]
        fn commutativity(mut a_incr: BTreeMap<u64, i64>, mut a_decr: BTreeMap<u64, i64>, mut b_incr: BTreeMap<u64, i64>, mut b_decr: BTreeMap<u64, i64>) {
            let a_incr = GrowCounter::from_u64_map(a_incr);
            let a_decr = GrowCounter::from_u64_map(a_decr);
            let b_incr = GrowCounter::from_u64_map(b_incr);
            let b_decr = GrowCounter::from_u64_map(b_decr);
            let a = PNCounter::new_from_incr_decr(a_incr, a_decr);
            let b = PNCounter::new_from_incr_decr(b_incr, b_decr);

            let ab = a.merge(&b);
            let ba = b.merge(&a);

            assert_eq!(ab, ba)
        }

        #[test]
        fn associativity(mut a_incr: BTreeMap<u64, i64>, mut a_decr: BTreeMap<u64, i64>, mut b_incr: BTreeMap<u64, i64>, mut b_decr: BTreeMap<u64, i64>, mut c_incr: BTreeMap<u64, i64>, mut c_decr: BTreeMap<u64, i64>) {
            let a_incr = GrowCounter::from_u64_map(a_incr);
            let a_decr = GrowCounter::from_u64_map(a_decr);
            let b_incr = GrowCounter::from_u64_map(b_incr);
            let b_decr = GrowCounter::from_u64_map(b_decr);
            let c_incr = GrowCounter::from_u64_map(c_incr);
            let c_decr = GrowCounter::from_u64_map(c_decr);
            let a = PNCounter::new_from_incr_decr(a_incr, a_decr);
            let b = PNCounter::new_from_incr_decr(b_incr, b_decr);
            let c = PNCounter::new_from_incr_decr(c_incr, c_decr);

            let ab_c = a.merge(&b).merge(&c);
            let a_bc = a.merge(&b.merge(&c));

            assert_eq!(ab_c, a_bc)
        }

        #[test]
        fn idempotency(mut a_incr: BTreeMap<u64, i64>, mut a_decr: BTreeMap<u64, i64>,) {
            let a_incr = GrowCounter::from_u64_map(a_incr);
            let a_decr = GrowCounter::from_u64_map(a_decr);
            let a = PNCounter::new_from_incr_decr(a_incr, a_decr);

            let result = a.merge(&a);
            assert_eq!(a, result)
        }
    }
}
