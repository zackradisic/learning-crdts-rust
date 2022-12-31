use crate::ReplicaId;

use super::gcounter::GCounter;

type Deltas = PNCounter;

#[derive(Debug, Clone, PartialEq)]
pub struct PNCounter {
    inc: GCounter,
    dec: GCounter,
}

impl PNCounter {
    pub fn value(&self) -> i64 {
        self.inc.value() - self.dec.value()
    }

    pub fn increment(&mut self, replica: ReplicaId) {
        self.inc.increment(replica)
    }

    pub fn decrement(&mut self, replica: ReplicaId) {
        self.dec.increment(replica)
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self {
            inc: self.inc.merge(&other.inc),
            dec: self.dec.merge(&other.dec),
        }
    }

    pub fn split(&self) -> (Self, Option<Deltas>) {
        let (inc, inc_deltas) = self.inc.split();
        let (dec, dec_deltas) = self.inc.split();
        let deltas = match (inc_deltas, dec_deltas) {
            (None, None) => None,
            (a, b) => Some(PNCounter {
                inc: *a.unwrap_or_default(),
                dec: *b.unwrap_or_default(),
            }),
        };

        (Self { inc, dec }, deltas)
    }

    pub fn split_expect(&self) -> (Self, Deltas) {
        let (counter, deltas) = self.split();
        (counter, deltas.expect("Expected deltas."))
    }

    pub fn new(inc: GCounter, dec: GCounter) -> Self {
        Self { inc, dec }
    }
}

impl Default for PNCounter {
    fn default() -> Self {
        Self {
            inc: Default::default(),
            dec: Default::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use proptest::prelude::*;

    use crate::delta_state::gcounter::test::gcounter_strategy;

    use super::PNCounter;

    pub fn pncounter_strategy() -> impl Strategy<Value = PNCounter> {
        (gcounter_strategy(), gcounter_strategy()).prop_map(|(inc, dec)| PNCounter::new(inc, dec))
    }

    proptest! {
        // #![proptest_config(ProptestConfig{ cases: 5, ..Default::default()})]
        #![proptest_config(ProptestConfig{ ..Default::default()})]

        #[test]
        fn commutativity(a in pncounter_strategy(), b in pncounter_strategy()) {

            let ab = a.merge(&b);
            let ba = b.merge(&a);


            assert_eq!(ab, ba)
        }

        #[test]
        fn associativity(a in pncounter_strategy(), b in pncounter_strategy(), c in pncounter_strategy()) {
            let ab_c = a.merge(&b).merge(&c);
            let bc = b.merge(&c);
            let a_bc = a.merge(&bc);

            assert_eq!(ab_c, a_bc)
        }

        #[test]
        fn idempotency(a in pncounter_strategy()) {
            assert_eq!(a, a.merge(&a))
        }
    }

    mod deltas {
        use proptest::prelude::*;

        use crate::delta_state::pncounter::{test::pncounter_strategy, PNCounter};

        proptest! {
            // #![proptest_config(ProptestConfig{ cases: 5, ..Default::default()})]
            #![proptest_config(ProptestConfig{ ..Default::default()})]

            #[test]
            fn commutativity(a in pncounter_strategy(), b in pncounter_strategy()) {
                let (a, a_deltas) = a.split_expect();
                let (b, b_deltas) = b.split_expect();

                let ab = a.merge(&b_deltas);
                let ba = b.merge(&a_deltas);

                let result_ab = PNCounter::default().merge(&ab);
                let result_ba = PNCounter::default().merge(&ba);

                assert_eq!(result_ab, result_ba)
            }

            #[test]
            fn associativity(a in pncounter_strategy(), b in pncounter_strategy(), c in pncounter_strategy()) {
                let (_, a_deltas) = a.split_expect();
                let (_, b_deltas) = b.split_expect();
                let (_, c_deltas) = c.split_expect();

                let ab_c = a_deltas.merge(&b_deltas).merge(&c_deltas);
                let bc = b_deltas.merge(&c_deltas);
                let a_bc = a_deltas.merge(&bc);

                let result_ab_c = PNCounter::default().merge(&ab_c);
                let result_a_bc = PNCounter::default().merge(&a_bc);

                assert_eq!(result_ab_c, result_a_bc)
            }

            #[test]
            fn idempotency(a in pncounter_strategy()) {
                let (_, a_deltas) = a.split_expect();
                let result = PNCounter::default().merge(&a_deltas);
                let result_idempotent = PNCounter::default().merge(&a_deltas.merge(&a_deltas));
                assert_eq!(result, result_idempotent)
            }
        }
    }
}
