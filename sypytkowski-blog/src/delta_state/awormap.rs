use std::fmt::Debug;
use std::hash::Hash;
use std::{cmp::Ord, collections::HashMap};

use serde::de::Visitor;
use serde::ser::SerializeTuple;
use serde::{Deserialize, Serialize};

use crate::{ReplicaId, Value};

use super::aworset::AWORSet;
use super::dot::DotKernel;

pub type Deltas<K, V> = DotKernel<KeyVal<K, V>>;

#[derive(Default, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "wasm",
    derive(
        serde_derive::Serialize,
        fp_bindgen::prelude::Serializable,
        serde_derive::Deserialize
    )
)]
#[cfg_attr(
    feature = "wasm",
    fp(rust_plugin_module = "sypytkowski_blog::delta_state::awormap")
)]
pub struct AWORMap<
    K: Clone + PartialEq + Default + Debug + Ord + Value,
    V: Value + Clone + Default + Debug,
> {
    pub(crate) keys: AWORSet<KeyVal<K, V>>,
}

impl<K, V> AWORMap<K, V>
where
    K: Clone + PartialEq + Default + Debug + Ord + Value + Hash,
    V: Value + Clone + Default + Debug + Hash,
{
    pub fn values_owned(&self) -> HashMap<K, V> {
        self.keys
            .values_iter()
            .map(|kv| (kv.key.clone(), kv.val.clone()))
            .collect()
    }
    pub fn values(&self) -> HashMap<&K, &V> {
        self.keys
            .values_iter()
            .map(|kv| (&kv.key, &kv.val))
            .collect()
    }
}

impl<K, V> AWORMap<K, V>
where
    K: Clone + PartialEq + Default + Debug + Ord + Value,
    V: Value + Clone + Default + Debug,
{
    pub fn insert(&mut self, replica: ReplicaId, key: K, value: V) {
        self.keys.add(replica, KeyVal { key, val: value });
    }

    pub fn remove(&mut self, replica: ReplicaId, key: K) {
        self.keys.remove(&KeyVal {
            key,
            val: Default::default(),
        });
    }

    pub fn merge_delta(&mut self, delta: Deltas<K, V>) {
        self.keys.merge_delta(delta);
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self {
            keys: self.keys.merge(&other.keys),
        }
    }

    pub fn split_mut(&mut self) -> Option<Deltas<K, V>> {
        self.keys.split_mut()
    }

    pub fn split(self) -> (Self, Option<Deltas<K, V>>) {
        let (keys, delta) = self.keys.split();
        (Self { keys }, delta)
    }

    pub fn split_expect_deltas(self) -> (Self, Deltas<K, V>) {
        let (keys, delta) = self.keys.split_expect_deltas();
        (Self { keys }, delta)
    }
}

/// Key-value pair so it can implement Serializable, note that
/// it also implements PartialEq but only compares keys
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "wasm", derive(fp_bindgen::prelude::Serializable,))]
#[cfg_attr(
    feature = "wasm",
    fp(rust_plugin_module = "sypytkowski_blog::delta_state::awormap")
)]
pub struct KeyVal<K: Clone + PartialEq + Default + Debug + Ord + Value, V: Value + Default + Debug>
{
    key: K,
    val: V,
}

impl<K, V> Eq for KeyVal<K, V>
where
    K: Clone + PartialEq + Default + Debug + Ord + Value,
    V: Value + Default + Debug,
{
    fn assert_receiver_is_total_eq(&self) {}
}
impl<K, V> PartialOrd for KeyVal<K, V>
where
    K: Clone + PartialEq + Default + Debug + Ord + Value,
    V: Value + Default + Debug,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.key.cmp(&other.key))
    }
}

impl<K, V> Ord for KeyVal<K, V>
where
    K: Clone + PartialEq + Default + Debug + Ord + Value,
    V: Value + Default + Debug,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        unsafe { self.partial_cmp(&other).unwrap_unchecked() }
    }
}

impl<K, V> Hash for KeyVal<K, V>
where
    K: Clone + PartialEq + Default + Debug + Ord + Value + Hash,
    V: Value + Clone + Default + Debug + Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.val.hash(state);
    }
}

impl<K, V> PartialEq for KeyVal<K, V>
where
    K: Clone + PartialEq + Default + Debug + Ord + Value,
    V: Value + Default + Debug,
{
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<K, V> Value for KeyVal<K, V>
where
    K: Clone + PartialEq + Default + Debug + Ord + Value + Value,
    V: Value + Default + Debug,
{
}

// impl<K, V> fp_bindgen::prelude::Serializable for KeyVal<K, V>
// where
//     K: Clone + PartialEq + Default + Debug + Ord + Value + Value,
//     V: Value + Default + Debug,
// {
//     fn ident() -> fp_bindgen::types::TypeIdent {
//         fp_bindgen::types::TypeIdent::new(
//             "KeyVal",
//             vec![(K::ident(), vec![]), (V::ident(), vec![])],
//         )
//     }

//     fn ty() -> fp_bindgen::types::Type {
//         use fp_bindgen::types::Type;
//         Type::Tuple(vec![K::ident(), V::ident()])
//     }
// }

impl<K, V> Serialize for KeyVal<K, V>
where
    K: Clone + PartialEq + Default + Debug + Ord + Value + Value + serde::Serialize,
    V: Value + Default + Debug + serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&self.key)?;
        tup.serialize_element(&self.val)?;
        tup.end()
    }
}

impl<'de, K, V> Deserialize<'de> for KeyVal<K, V>
where
    K: Clone + PartialEq + Default + Debug + Ord + Value + Value + serde::Deserialize<'de>,
    V: Value + Default + Debug + serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        KeyValVisitor::<K, V>::deserialize(deserializer).map(|kv| KeyVal {
            key: kv.0,
            val: kv.1,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct KeyValVisitor<K, V>(K, V)
where
    K: Clone + PartialEq + Default + Debug + Ord + Value + Value,
    V: Value + Default + Debug;

#[cfg(test)]
mod test {
    use crate::ReplicaGenerator;

    use super::AWORMap;

    #[test]
    fn works() {
        let mut gen = ReplicaGenerator::new();
        let a_id = gen.gen();
        let b_id = gen.gen();

        let mut a = AWORMap::<String, String>::default();
        let mut b = AWORMap::<String, String>::default();

        a.insert(a_id, "fruit".into(), "apple".into());
        b.insert(b_id, "fruit".into(), "orange".into());
        let (mut a, a_deltas) = a.split_expect_deltas();
        let (mut b, b_deltas) = b.split_expect_deltas();

        a.merge_delta(b_deltas);
        b.merge_delta(a_deltas);

        // let c = a.merge(&b);
        // println!("C: {:#?}", c.values());

        println!("A: {:#?}\n\nB: {:#?}", a.values(), b.values());
    }

    mod properties {
        use proptest::prelude::*;
        use std::fmt::Debug;

        use crate::{
            delta_state::{
                awormap::{AWORMap, KeyVal},
                aworset::{self, test::properties::aworset_strategy_impl},
            },
            Value,
        };

        fn keyval_strategy<
            K: Clone + PartialEq + Default + Debug + Value + Ord,
            V: Value + Default + Debug + Clone,
        >(
            key_strat: impl Strategy<Value = K>,
            value_strat: impl Strategy<Value = V>,
        ) -> impl Strategy<Value = KeyVal<K, V>> {
            (key_strat, value_strat).prop_map(|(key, val)| KeyVal { key, val })
        }

        fn awormap_strategy_impl<
            K: Clone + PartialEq + Default + Debug + Value + Ord,
            V: Value + Default + Debug + Clone,
        >(
            key_strat: impl Strategy<Value = K> + 'static,
            value_strat: impl Strategy<Value = V> + 'static,
        ) -> impl Strategy<Value = AWORMap<K, V>> {
            aworset_strategy_impl(keyval_strategy(key_strat, value_strat))
                .prop_map(|keys| AWORMap { keys })
        }
        fn awormap_strategy() -> impl Strategy<Value = AWORMap<u16, u16>> {
            awormap_strategy_impl(any::<u16>(), any::<u16>())
        }

        fn patch<
            K: Clone + PartialEq + Default + Debug + Value + Ord,
            V: Value + Default + Debug + Clone,
        >(
            awormaps: &mut [&mut AWORMap<K, V>],
        ) {
            let mut aworsets = awormaps
                .iter_mut()
                .map(|awormap| &mut awormap.keys)
                .collect::<Vec<_>>();
            aworset::test::properties::patch(&mut aworsets);
        }

        proptest! {
            // #![proptest_config(ProptestConfig{ cases: 1, ..Default::default()})]
            #![proptest_config(ProptestConfig{ ..Default::default()})]


            #[test]
            fn commutativity(mut a in awormap_strategy(), mut b in awormap_strategy()) {
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

        // TODO: finish
        // mod delta {
        //     use proptest::prelude::*;

        //     use crate::delta_state::awormap::test::properties::{awormap_strategy, patch};

        //     proptest! {
        //         // #![proptest_config(ProptestConfig{ cases: 1, ..Default::default()})]
        //         #![proptest_config(ProptestConfig{ ..Default::default()})]

        //         #[test]
        //         fn commutativity(mut a in awormap_strategy(), mut b in awormap_strategy()) {
        //             patch(&mut [&mut a, &mut b]);

        //             let (mut a, a_deltas) = a.split_expect_deltas();
        //             let (mut b, b_deltas) = b.split_expect_deltas();

        //             let a = a.merge_delta(b_deltas);
        //             let b = b.merge_delta(a_deltas);

        //             assert_eq!(a, b);
        //         }

        //         #[test]
        //         fn associativity(mut a in awormap_strategy(), mut b in awormap_strategy(), mut c in awormap_strategy()) {
        //             patch(&mut [&mut a, &mut b, &mut c]);
        //             let ab_c = a.merge(&b).merge(&c);
        //             let a_bc = a.merge(&b.merge(&c));

        //             assert_eq!(ab_c, a_bc);
        //         }

        //         #[test]
        //         fn idempotency(a in awormap_strategy()) {
        //             let aa = a.merge(&a);

        //             assert_eq!(aa, a);
        //         }
        //     }
        // }
    }
}
