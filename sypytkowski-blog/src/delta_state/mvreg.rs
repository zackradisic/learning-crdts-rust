use std::collections::BTreeSet;

use crate::ReplicaId;

use super::dot::DotKernel;

#[derive(Debug, Clone, PartialEq)]
pub struct MVReg<V: Clone> {
    pub(crate) core: DotKernel<V>,
    pub(crate) delta: Option<DotKernel<V>>,
}

impl<V: Clone + Default + PartialEq> Default for MVReg<V> {
    fn default() -> Self {
        Self {
            core: Default::default(),
            delta: Default::default(),
        }
    }
}

impl<V: Clone + std::fmt::Debug + PartialEq + Ord + Default> MVReg<V> {
    pub fn value(&self) -> BTreeSet<&V> {
        self.core.values().collect()
    }

    pub fn set(&mut self, replica: ReplicaId, value: V) {
        let delta = self.delta.get_or_insert_default();
        self.core.remove_all();
        self.core.add(replica, value, delta);
    }

    pub fn merge(&self, other: &Self) -> Self {
        let delta = match (&self.delta, &other.delta) {
            (Some(a), Some(b)) => Some(a.merge(b)),
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };

        let core = self.core.merge(&other.core);

        Self { core, delta }
    }

    pub fn merge_delta(&mut self, delta: DotKernel<V>) {
        let delta = match &self.delta {
            Some(a) => a.merge(&delta),
            None => delta,
        };

        self.core = self.core.merge(&delta);
        self.delta = Some(delta);
    }

    pub fn split(&self) -> (Self, Option<DotKernel<V>>) {
        (
            Self {
                core: self.core.clone(),
                delta: None,
            },
            self.delta.clone(),
        )
    }

    pub fn split_expect_deltas(&self) -> (Self, DotKernel<V>) {
        let (core, delta) = self.split();
        (core, delta.expect("Deltas should be be defined."))
    }
}

#[cfg(test)]
mod test {
    use crate::ReplicaGenerator;

    use super::MVReg;

    #[test]
    fn basic() {
        let mut gen = ReplicaGenerator::new();
        let a_id = gen.gen();
        let b_id = gen.gen();
        let mut a = MVReg::<&str>::default();
        let mut b = MVReg::<&str>::default();

        a.set(a_id, "noice");
        let (a, a_deltas) = a.split_expect_deltas();
        b.merge_delta(a_deltas);
        let (b, _) = b.split();

        assert_eq!(a, b)
    }

    mod properties {
        use crate::delta_state::{
            dot::{
                test::{dotkernel_strategy, patch_kernels},
                DotKernel,
            },
            mvreg::MVReg,
        };
        use proptest::prelude::*;

        fn mvgreg_strategy() -> impl Strategy<Value = MVReg<u16>> {
            dotkernel_strategy(any::<u16>()).prop_map(|core| MVReg { core, delta: None })
        }
        fn patch<V: Clone + PartialEq + Default>(mvgregs: &mut [&mut MVReg<V>]) {
            let mut kernels: Vec<&mut DotKernel<V>> =
                mvgregs.iter_mut().map(|reg| &mut reg.core).collect();
            patch_kernels(kernels.as_mut_slice())
        }

        proptest! {
            // #![proptest_config(ProptestConfig{ cases: 1, ..Default::default()})]
            #![proptest_config(ProptestConfig{ ..Default::default()})]

            #[test]
            fn commutativity(mut a in mvgreg_strategy(), mut b in mvgreg_strategy()) {
                patch(&mut [&mut a, &mut b]);

                let ab = a.merge(&b);
                let ba = b.merge(&a);

                // if ab != ba {
                //     println!("THE A: {:?}", a);
                //     println!("THE B: {:?}", b);
                // }
                assert_eq!(ab, ba);

            }

            #[test]
            fn associativity(mut a in mvgreg_strategy(), mut b in mvgreg_strategy(), mut c in mvgreg_strategy()) {
                patch(&mut [&mut a, &mut b, &mut c]);
                let ab_c = a.merge(&b).merge(&c);
                let a_bc = a.merge(&b.merge(&c));

                assert_eq!(ab_c, a_bc);
            }

            #[test]
            fn idempotency(a in mvgreg_strategy()) {
                let aa = a.merge(&a);

                assert_eq!(aa, a);
            }
        }
    }
}
