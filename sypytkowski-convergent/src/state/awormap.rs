use std::collections::BTreeMap;

use crate::{
    delta_state::{aworset::AWORSet, convergent::Convergent},
    ReplicaId, Value,
};

#[derive(Clone, Debug, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct AWORMap<
    K: Clone + PartialEq + Default + std::fmt::Debug + std::cmp::Ord + Value,
    V: Value,
> {
    pub(crate) keys: AWORSet<K>,
    pub(crate) entries: BTreeMap<K, V>,
}

impl<K: Clone + PartialEq + Default + std::fmt::Debug + std::cmp::Ord + Value, V: Value> Default
    for AWORMap<K, V>
{
    fn default() -> Self {
        Self {
            keys: Default::default(),
            entries: Default::default(),
        }
    }
}

impl<
        K: Clone + PartialEq + Default + std::fmt::Debug + std::cmp::Ord + Value,
        V: Convergent + Clone + std::fmt::Debug + Value,
    > AWORMap<K, V>
{
    pub fn value(&self) -> &BTreeMap<K, V> {
        &self.entries
    }

    pub fn add(&mut self, replica: ReplicaId, key: K, value: V) {
        self.keys.add(replica, key.clone());
        self.entries.insert(key, value);
    }

    pub fn rem(&mut self, key: &K) {
        self.keys.remove(&key);
        self.entries.remove(&key);
    }

    pub fn merge(&self, other: &Self) -> Self {
        let keys = self.keys.merge(&other.keys);
        let mut entries = BTreeMap::<K, V>::default();

        for key in keys.values_iter() {
            if let Some(_) = entries.get(key) {
                continue;
            }
            match (self.entries.get(key), other.entries.get(key)) {
                (Some(a), Some(b)) => {
                    let merged = a.merge(b);
                    entries.insert(key.clone(), merged);
                }
                (Some(a), None) => {
                    entries.insert(key.clone(), a.clone());
                }
                (None, Some(b)) => {
                    entries.insert(key.clone(), b.clone());
                }
                (None, None) => (),
            }
        }

        Self { keys, entries }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        delta_state::{
            aworset::AWORSet,
            dot::{Dot, DotCtx, DotKernel, VectorClock},
        },
        state::awormap::AWORMap,
        ReplicaGenerator, ReplicaId,
    };

    mod properties {

        use crate::{
            delta_state::aworset::{self, test::properties::aworset_strategy},
            state::awormap::AWORMap,
            Value,
        };
        use proptest::prelude::*;

        fn awormap_strategy() -> impl Strategy<Value = AWORMap<u16, u16>> {
            aworset_strategy()
                .prop_flat_map(|keys| {
                    let values = if keys.len() == 0 {
                        proptest::collection::vec(any::<u16>(), 0..=0)
                    } else {
                        proptest::collection::vec(any::<u16>(), 0..keys.len())
                    };
                    (Just(keys), values)
                })
                .prop_map(|(keys, values)| AWORMap {
                    entries: keys.kernel.entries.values().copied().zip(values).collect(),
                    keys,
                })
        }

        fn patch<
            K: std::clone::Clone
                + std::cmp::PartialEq
                + std::default::Default
                + std::fmt::Debug
                + Ord
                + Value,
            V: Clone + PartialEq + Default + Value,
        >(
            awormaps: &mut [&mut AWORMap<K, V>],
        ) {
            let mut aworsets = awormaps
                .iter_mut()
                .map(|awormap| &mut awormap.keys)
                .collect::<Vec<_>>();
            aworset::test::properties::patch(&mut aworsets);

            for map in awormaps.iter_mut() {
                // Above will delete keys so prune them from entries as well
                let keys = map.keys.value();
                map.entries.drain_filter(|k, _| !keys.contains(k));
            }
        }

        proptest! {
            // #![proptest_config(ProptestConfig{ cases: 1, ..Default::default()})]
            #![proptest_config(ProptestConfig{ ..Default::default()})]

            #[test]
            fn commutativity(mut a in awormap_strategy(), mut b in awormap_strategy()) {
                patch(&mut [&mut a, &mut b]);

                let ab = a.merge(&b);
                let ba = b.merge(&a);

                assert_eq!(ab, ba);
            }

            // TODO: Broke these tests, see `test` and `test2` for inputs to reproduce.
            // Problem is that the strategy for `AWORMap` is not correct and can create
            // entries that exist in the underlying AWORSet kernel but not in the `entries` map
            #[test]
            fn associativity(mut a in awormap_strategy(), mut b in awormap_strategy(), mut c in awormap_strategy()) {
                patch(&mut [&mut a, &mut b, &mut c]);

                let ab_c = a.merge(&b).merge(&c);
                let a_bc = a.merge(&b.merge(&c));

                assert_eq!(ab_c, a_bc);
            }

            #[test]
            fn idempotency(a in awormap_strategy()) {
                let aa = a.merge(&a);

                assert_eq!(aa, a);
            }
        }
    }
}
