use std::collections::{BTreeSet, HashSet};

use crate::ReplicaId;

use super::dot::DotKernel;

#[derive(Debug, Clone, PartialEq)]
pub struct AWORSet<V>
where
    V: Clone + PartialEq + Default,
{
    pub(crate) kernel: DotKernel<V>,
    delta: Option<DotKernel<V>>,
}

impl<V> Default for AWORSet<V>
where
    V: Clone + PartialEq + Default,
{
    fn default() -> Self {
        Self {
            kernel: Default::default(),
            delta: Default::default(),
        }
    }
}

impl<V> AWORSet<V>
where
    V: Clone + PartialEq + Default + std::fmt::Debug,
{
    pub fn new(kernel: DotKernel<V>) -> Self {
        Self {
            kernel,
            delta: None,
        }
    }

    pub fn add(&mut self, replica: ReplicaId, value: V) {
        let deltas = self.delta.get_or_insert_default();
        // Remove duplicates
        self.kernel.remove(value.clone(), deltas);
        self.kernel.add(replica, value, deltas);
    }

    pub fn remove(&mut self, value: V) {
        self.kernel
            .remove(value, self.delta.get_or_insert_default());
    }

    pub fn merge(&self, other: &Self) -> Self {
        let delta = match (&self.delta, &other.delta) {
            (Some(a), Some(b)) => Some(a.merge(b)),
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };

        let kernel = self.kernel.merge(&other.kernel);

        Self { kernel, delta }
    }

    pub fn merge_delta(&mut self, delta: DotKernel<V>) {
        let new_deltas = match &self.delta {
            Some(a) => a.merge(&delta),
            None => delta,
        };

        self.kernel = self.kernel.merge(&new_deltas);
        self.delta = Some(new_deltas);
    }

    pub fn split(self) -> (AWORSet<V>, Option<DotKernel<V>>) {
        (AWORSet::new(self.kernel), self.delta)
    }
    pub fn split_expect_deltas(self) -> (AWORSet<V>, DotKernel<V>) {
        let (kernel, maybe_deltas) = self.split();
        (kernel, maybe_deltas.expect("Deltas should be defined."))
    }
}

impl<V> AWORSet<V>
where
    V: Clone + PartialEq + Default + Ord + std::fmt::Debug,
{
    pub fn value(&self) -> BTreeSet<V> {
        self.kernel.values().cloned().collect()
    }
}

impl<V> AWORSet<V>
where
    V: Clone + PartialEq + Default + Eq + std::hash::Hash + std::fmt::Debug,
{
    pub fn value_hashset(&self) -> HashSet<V> {
        self.kernel.values().cloned().collect()
    }
}

#[cfg(test)]
mod test {
    use crate::ReplicaGenerator;

    use super::AWORSet;

    #[test]
    fn basic() {
        let mut gen = ReplicaGenerator::new();
        let a_id = gen.gen();
        let b_id = gen.gen();
        let mut a = AWORSet::<&str>::default();
        let mut b = AWORSet::<&str>::default();

        a.add(a_id, "noice");
        let (a, a_deltas) = a.split_expect_deltas();
        b.merge_delta(a_deltas);
        let (b, _) = b.split();

        assert_eq!(a, b)
    }

    mod properties {
        use crate::delta_state::{
            aworset::AWORSet,
            dot::{
                test::{dotkernel_strategy, patch_kernels},
                DotKernel,
            },
        };
        use proptest::prelude::*;

        fn aworset_strategy() -> impl Strategy<Value = AWORSet<u16>> {
            dotkernel_strategy(any::<u16>()).prop_map(|kernel| AWORSet {
                kernel,
                delta: None,
            })
        }
        fn patch<V: Clone + PartialEq + Default>(aworsets: &mut [&mut AWORSet<V>]) {
            let mut kernels: Vec<&mut DotKernel<V>> =
                aworsets.iter_mut().map(|set| &mut set.kernel).collect();
            patch_kernels(kernels.as_mut_slice())
        }

        proptest! {
            // #![proptest_config(ProptestConfig{ cases: 1, ..Default::default()})]
            #![proptest_config(ProptestConfig{ ..Default::default()})]

            #[test]
            fn commutativity(mut a in aworset_strategy(), mut b in aworset_strategy()) {
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
            fn associativity(mut a in aworset_strategy(), mut b in aworset_strategy(), mut c in aworset_strategy()) {
                patch(&mut [&mut a, &mut b, &mut c]);
                let ab_c = a.merge(&b).merge(&c);
                let a_bc = a.merge(&b.merge(&c));

                assert_eq!(ab_c, a_bc);
            }

            #[test]
            fn idempotency(a in aworset_strategy()) {
                let aa = a.merge(&a);

                assert_eq!(aa, a);
            }
        }
    }
}