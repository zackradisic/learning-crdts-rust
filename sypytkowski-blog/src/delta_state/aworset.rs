use std::collections::{BTreeSet, HashSet};

use crate::{ReplicaId, Value};

use super::dot::DotKernel;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "wasm",
    derive(
        fp_bindgen::prelude::Serializable,
        serde_derive::Serialize,
        serde_derive::Deserialize
    )
)]
pub struct AWORSet<V: Clone + PartialEq + Default + Value> {
    pub(crate) kernel: DotKernel<V>,
    pub(crate) delta: Option<DotKernel<V>>,
}

impl<V> Default for AWORSet<V>
where
    V: Clone + PartialEq + Default + Value,
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
    V: Clone + PartialEq + Default + std::fmt::Debug + Value,
{
    pub fn new(kernel: DotKernel<V>) -> Self {
        Self {
            kernel,
            delta: None,
        }
    }

    pub fn len(&self) -> usize {
        self.kernel.entries.len()
    }

    pub fn add(&mut self, replica: ReplicaId, value: V) {
        let deltas = self.delta.get_or_insert_default();
        // Remove duplicates
        self.kernel.remove(&value, deltas);
        self.kernel.add(replica, value, deltas);
    }

    pub fn remove(&mut self, value: &V) {
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

    pub fn split_mut(&mut self) -> Option<DotKernel<V>> {
        let delta = self.delta.take();
        delta
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
    V: Clone + PartialEq + Default + Ord + std::fmt::Debug + Value,
{
    pub fn value(&self) -> BTreeSet<V> {
        self.kernel.values().cloned().collect()
    }
    pub fn values_ref(&self) -> BTreeSet<&V> {
        self.kernel.values().collect()
    }
}

impl<V> AWORSet<V>
where
    V: Clone + PartialEq + Default + std::fmt::Debug + Value,
{
    pub fn values_iter(&self) -> std::collections::btree_map::Values<super::dot::Dot, V> {
        self.kernel.values()
    }
}

impl<V> AWORSet<V>
where
    V: Clone + PartialEq + Default + Eq + std::hash::Hash + std::fmt::Debug + Value,
{
    pub fn value_hashset(&self) -> HashSet<V> {
        self.kernel.values().cloned().collect()
    }
}

#[cfg(test)]
pub mod test {
    use crate::ReplicaGenerator;

    use super::AWORSet;

    #[test]
    fn basic() {
        let mut gen = ReplicaGenerator::new();
        let a_id = gen.gen();
        let b_id = gen.gen();
        let mut a = AWORSet::<String>::default();
        let mut b = AWORSet::<String>::default();

        a.add(a_id, "noice".into());
        let (a, a_deltas) = a.split_expect_deltas();
        b.merge_delta(a_deltas);
        let (b, _) = b.split();

        assert_eq!(a, b)
    }

    pub mod properties {
        use std::fmt::Debug;

        use crate::{
            delta_state::{
                aworset::AWORSet,
                dot::{
                    test::{dotkernel_strategy, patch_kernels},
                    DotKernel,
                },
            },
            Value,
        };
        use proptest::prelude::*;

        pub fn aworset_strategy_impl<V: Debug + Clone + Value + Default + PartialEq>(
            value_strat: impl Strategy<Value = V> + 'static,
        ) -> impl Strategy<Value = AWORSet<V>> {
            dotkernel_strategy(value_strat).prop_map(|kernel| AWORSet {
                kernel,
                delta: None,
            })
        }
        pub fn aworset_strategy() -> impl Strategy<Value = AWORSet<u16>> {
            aworset_strategy_impl(any::<u16>())
        }
        pub fn patch<V: Clone + PartialEq + Default + Value>(aworsets: &mut [&mut AWORSet<V>]) {
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
