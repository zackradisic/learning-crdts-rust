//! Implementation of Dotted Version Vectors (DVV) from blog series part 3
//!
//! I think the blog post omits some details about DVVs so I recommend reading this paper
//! "Dotted Version Vectors: Efficient Causality Tracking for Distributed Key-Value Stores" (https://gsd.di.uminho.pt/members/vff/dotted-version-vectors-2012.pdf)
//!
//! A dot is tuple of (replica, sequence number)
//! A DVV is a tuple of (dot, version vector)
//! Note the version vector is represented as a vector clock and a dot cloud
//!
//! Read the first four sections of the linked paper which explains in the detail the need for DVVs through examples.
//!
//! The main innovation of DVV is that comparison is O(1) instead of O(n) where n is the number of replicas.
//!
//! For two DVVs: ((i, n), u) and ((j, m), v) if n <= v(i) then it follows that u <= v
//! basicaly you just need to check if the sequence number of the dot is less than or equal to the version vector at the replica id of the dot.
//! If not then the two DVVs are concurrent.
//!
//! In addition, DVVs are also more space efficient. For example in the original ORSet implementation (see or_set.rs) we store a
//! vector clock for each element, we need to track every client that has added or removed the element. This is not necessary with DVVs.
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    ops::{Deref, DerefMut},
};

use serde::{de::Visitor, Deserialize, Serialize};

use crate::{ReplicaId, Value};

#[derive(Deserialize)]
pub struct VectorClockDeserializer(BTreeMap<String, u64>);

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "wasm", derive(fp_bindgen::prelude::Serializable))]
#[cfg_attr(
    feature = "wasm",
    fp(rust_plugin_module = "sypytkowski_blog::delta_state::dot")
)]
pub struct VectorClock(pub BTreeMap<ReplicaId, u64>);

impl Default for VectorClock {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl Deref for VectorClock {
    type Target = BTreeMap<ReplicaId, u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VectorClock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Serialize for VectorClock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        let mut str_buf = String::new();
        for (&replica_id, &value) in self.iter() {
            let start = str_buf.len();
            write!(str_buf, "{:?}:{:?}", replica_id.0, value).unwrap();
            let end = str_buf.len();
            map.serialize_entry(&str_buf.as_str()[start..end], &value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for VectorClock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut str_map = VectorClockDeserializer::deserialize(deserializer)?.0;
        let mut map = BTreeMap::new();
        for (k, v) in str_map {
            let id = ReplicaId(k.parse().map_err(|e| serde::de::Error::custom(e))?);
            map.insert(id, v);
        }
        Ok(VectorClock(map))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "wasm", derive(fp_bindgen::prelude::Serializable))]
#[cfg_attr(
    feature = "wasm",
    fp(rust_plugin_module = "sypytkowski_blog::delta_state::dot")
)]
pub struct Dot(pub ReplicaId, pub u64);

impl serde::Serialize for Dot {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:?}:{:?}", self.0 .0, self.1))
    }
}
struct DotDeserializer;
impl<'de> Visitor<'de> for DotDeserializer {
    type Value = Dot;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "Dot")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut split = v.split(':');
        let replica = split
            .next()
            .ok_or(serde::de::Error::missing_field("replica"))?;
        let sequence_number = split
            .next()
            .ok_or(serde::de::Error::missing_field("sequence number"))?;

        Ok(Dot(
            ReplicaId(
                replica
                    .parse::<u64>()
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?,
            ),
            sequence_number
                .parse::<u64>()
                .map_err(|e| serde::de::Error::custom(e.to_string()))?,
        ))
    }
}
impl<'de> serde::Deserialize<'de> for Dot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(DotDeserializer)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "wasm",
    derive(
        fp_bindgen::prelude::Serializable,
        serde_derive::Serialize,
        serde_derive::Deserialize
    )
)]
pub struct DotKernel<V: Clone + Value> {
    pub(crate) ctx: DotCtx,
    pub(crate) entries: BTreeMap<Dot, V>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "wasm",
    derive(
        fp_bindgen::prelude::Serializable,
        serde_derive::Serialize,
        serde_derive::Deserialize
    )
)]
pub struct DotCtx {
    pub(crate) clock: VectorClock,
    pub(crate) dot_cloud: BTreeSet<Dot>,
}

impl<V: Clone + PartialEq + Default + Value> Default for DotKernel<V> {
    fn default() -> Self {
        Self {
            ctx: Default::default(),
            entries: Default::default(),
        }
    }
}

impl<V: Clone + PartialEq + std::fmt::Debug + Value> DotKernel<V> {
    pub fn values(&self) -> std::collections::btree_map::Values<Dot, V> {
        self.entries.values()
    }

    pub fn merge(&self, other: &Self) -> Self {
        // Initialize entries from `self`
        let mut entries = self.entries.clone();

        // Add unseen items from `other`
        for (dot, val) in other.entries.iter() {
            if !(self.entries.contains_key(dot) || self.ctx.contains(*dot)) {
                entries.insert(*dot, val.clone());
            }
        }

        // If `other`'s dot context has the dot Dot(i, n) but its entries do not, it means `other`
        // saw it and deleted it from its own entries.
        //
        // Here is an example to illustrate this:
        // A's state:
        //   Entries: {Dot(A, 2) -> "lmao", Dot(B, 1) -> "lol"}
        //   Ctx:     {Dot(A, 2), Dot(B, 1)}
        // B's state:
        //   Entries: {Dot(B, 2) -> "norb", Dot(B, 1) -> "lol"}
        //   Ctx:     {Dot(A, 1), Dot(B, 2)}
        //
        // If we merge A and B, we see that B does not have Dot(A, 2) in its ctx, so we don't remove "lmao".
        // But if it did have Dot(A, 2) then it means A <= B.
        for dot in self.entries.keys() {
            if other.ctx.contains(*dot) && !other.entries.contains_key(dot) {
                entries.remove(dot);
            }
        }

        Self {
            entries,
            ctx: self.ctx.merge(&other.ctx),
        }
    }

    pub fn add(&mut self, replica: ReplicaId, value: V, delta: &mut Self) {
        let dot = self.ctx.next_dot(replica);
        self.entries.insert(dot, value.clone());
        delta.entries.insert(dot, value);
        delta.ctx.add(dot);
        delta.ctx.compact();
    }

    pub fn remove(&mut self, value: &V, delta: &mut Self) {
        let dot = self
            .entries
            .iter()
            .find(|(_, val)| val == &value)
            .map(|(k, _)| *k);

        if let Some(dot) = dot {
            self.entries.remove(&dot);
            delta.ctx.add(dot);
        }
    }

    pub fn remove_all(&mut self) {
        for (k, _) in self.entries.drain_filter(|_, _| true) {
            self.ctx.add(k);
        }
        self.ctx.compact();
    }
}

impl Default for DotCtx {
    fn default() -> Self {
        Self {
            clock: Default::default(),
            dot_cloud: Default::default(),
        }
    }
}

impl DotCtx {
    pub fn add(&mut self, dot: Dot) {
        self.dot_cloud.insert(dot);
    }

    pub fn contains(&self, dot @ Dot(id, n): Dot) -> bool {
        match self.clock.get(&id) {
            Some(found) if *found >= n => true,
            _ => self.dot_cloud.contains(&dot),
        }
    }

    pub fn next_dot(&mut self, replica: ReplicaId) -> Dot {
        let val = self
            .clock
            .entry(replica)
            .and_modify(|val| {
                *val += 1;
            })
            .or_insert(1);

        Dot(replica, *val)
    }

    pub fn merge(&self, other: &Self) -> Self {
        let clock = self
            .clock
            .iter()
            .fold(other.clock.clone(), |mut acc, (&key, &new_val)| {
                acc.entry(key)
                    .and_modify(|val| {
                        *val = new_val.max(*val);
                    })
                    .or_insert(new_val);
                acc
            });

        let mut dot_cloud = self.dot_cloud.clone();
        dot_cloud.extend(other.dot_cloud.iter());

        let mut ret = Self { clock, dot_cloud };
        ret.compact();
        ret
    }

    /// - If dot for replica in the cloud is 1 greater than the value in clock, update the clock with it and remove from cloud
    /// - If dot for replica in the cloud is less than or equal to the value in clock, remove from cloud
    /// - Otherwise do nothing
    pub fn compact(&mut self) {
        drop(self.dot_cloud.drain_filter(|&Dot(id, n)| {
            let n2 = self.clock.get(&id).copied().unwrap_or(0);

            if n == n2 + 1 {
                self.clock.insert(id, n);
                true
            } else if n <= n2 {
                true
            } else {
                false
            }
        }));
    }
}

#[cfg(test)]
pub mod test {
    use std::collections::{BTreeMap, BTreeSet};

    use proptest::{
        collection::{btree_map, btree_set},
        prelude::*,
        strategy::ValueTree,
    };

    use crate::{ReplicaId, Value};

    use super::{Dot, DotCtx, DotKernel, VectorClock};

    const MAX_VALUES: u64 = 100;
    pub fn dot_strategy() -> impl Strategy<Value = Dot> {
        ((5..MAX_VALUES), (0..MAX_VALUES)).prop_map(|(id, n)| Dot(id.into(), n))
    }
    fn dotcloud_strategy() -> impl Strategy<Value = BTreeSet<Dot>> {
        btree_set(dot_strategy(), 0..(MAX_VALUES as usize))
    }
    fn vector_clock_strategy() -> impl Strategy<Value = VectorClock> {
        btree_map(
            (5..MAX_VALUES).prop_map(ReplicaId),
            0..MAX_VALUES,
            0..(MAX_VALUES as usize),
        )
        .prop_map(VectorClock)
    }
    fn dotctx_strategy() -> impl Strategy<Value = DotCtx> {
        (dotcloud_strategy(), vector_clock_strategy()).prop_map(|(dot_cloud, clock)| {
            let mut ctx = DotCtx { dot_cloud, clock };
            ctx.compact();
            ctx
        })
    }

    pub fn dotkernel_strategy<V: Clone + std::fmt::Debug + Value>(
        value_strategy: impl Strategy<Value = V>,
    ) -> impl Strategy<Value = DotKernel<V>> {
        (
            dotctx_strategy(),
            btree_map(dot_strategy(), value_strategy, 0..(MAX_VALUES as usize)),
        )
            .prop_map(|(mut ctx, entries)| {
                // Make sure that entries dots are in the context
                // It is invalid for a dot in an entry to not be in the context
                for dot in entries.keys() {
                    if !ctx.contains(*dot) {
                        ctx.add(*dot);
                    }
                }
                ctx.compact();

                (ctx, entries)
            })
            .prop_map(|(ctx, entries)| DotKernel { ctx, entries })
    }

    // pub fn dotkernel_strategy<V: Clone + std::fmt::Debug>(
    //     value_strategy: impl Strategy<Value = V>,
    // ) -> impl Strategy<Value = DotKernel<V>> {
    //     (
    //         dotctx_strategy(),
    //         btree_map(dot_strategy(), value_strategy, 0..(MAX_VALUES as usize)),
    //     )
    //         .prop_map(|(ctx, entries)| DotKernel { ctx, entries })
    // }

    // #[derive(Debug)]
    // pub struct DotKernelState<K, V> {
    //     pub map: BTreeMap<K, V>,
    // }
    // #[derive(Debug)]
    // pub struct DotKernelStrategy<K: std::fmt::Debug, V: Clone + std::fmt::Debug, S: Strategy> {
    //     pub value_strategy: S,
    //     state: DotKernelState<K, V>,
    // }
    // impl<K: std::fmt::Debug, V: Clone + std::fmt::Debug, S: Strategy> DotKernelStrategy<K, V, S> {
    //     pub fn new(value_strategy: S, state: Option<DotKernelState<K, V>>) -> Self {
    //         Self {
    //             value_strategy: value_strategy,
    //             state: state.unwrap_or_else(|| DotKernelState {
    //                 map: BTreeMap::new(),
    //             }),
    //         }
    //     }
    // }
    // impl<K: std::fmt::Debug, V: Clone + std::fmt::Debug, S: Strategy> Strategy
    //     for DotKernelStrategy<K, V, S>
    // {
    //     type Tree = ValueTree<Value = V>;
    //     type Value = V;

    //     fn new_tree(
    //         &self,
    //         runner: &mut proptest::test_runner::TestRunner,
    //     ) -> proptest::strategy::NewTree<Self> {
    //         todo!()
    //     }
    // }

    /// We can't have kernels that have different dot for the same value
    /// in the entries map, for example:
    ///
    /// Kernel 1 has (8, 9): 420
    /// Kernel 2 has (8, 9): 69
    ///
    /// This is an illegal state because it means the replica has two different values
    /// at the exact same time, which is impossible because adding to a replica
    /// increments the dot.
    pub fn patch_kernels<V: Clone + PartialEq + Value>(kernels: &mut [&mut DotKernel<V>]) {
        let mut deletions: Vec<(usize, Dot)> = vec![];
        for i in 0..kernels.len() {
            for (dot, value) in kernels[i].entries.iter() {
                for (i, other) in kernels.iter().enumerate().skip(i + 1) {
                    if let Some(other_value) = other.entries.get(dot) {
                        if other_value != value {
                            deletions.push((i, *dot))
                        }
                    }
                }
            }
        }
        for deletion in &deletions {
            kernels[deletion.0].entries.remove(&deletion.1);
        }
    }

    mod kernel {
        use proptest::prelude::*;

        use crate::delta_state::dot::test::{
            dotkernel_strategy as dotkernel_strategy_impl, patch_kernels,
        };

        fn dotkernel_strategy() -> impl Strategy<Value = super::DotKernel<u16>> {
            // change as needed
            dotkernel_strategy_impl(any::<u16>())
        }

        proptest! {
            // #![proptest_config(ProptestConfig{ cases: 1, ..Default::default()})]
            #![proptest_config(ProptestConfig{ ..Default::default()})]

            #[test]
            fn commutativity(mut a in dotkernel_strategy(), mut b in dotkernel_strategy()) {
                patch_kernels(&mut [&mut a, &mut b]);

                let ab = a.merge(&b);
                let ba = b.merge(&a);

                // if ab != ba {
                //     println!("THE A: {:?}", a);
                //     println!("THE B: {:?}", b);
                // }
                assert_eq!(ab, ba);

            }

            #[test]
            fn associativity(mut a in dotkernel_strategy(), mut b in dotkernel_strategy(), mut c in dotkernel_strategy()) {
                patch_kernels(&mut [&mut a, &mut b, &mut c]);
                let ab_c = a.merge(&b).merge(&c);
                let a_bc = a.merge(&b.merge(&c));

                assert_eq!(ab_c, a_bc);
            }

            #[test]
            fn idempotency(a in dotkernel_strategy()) {
                let aa = a.merge(&a);

                assert_eq!(aa, a);
            }
        }
    }

    mod ctx {
        use proptest::prelude::*;

        use crate::delta_state::dot::test::dotctx_strategy;

        proptest! {
            // #![proptest_config(ProptestConfig{ cases: 5, ..Default::default()})]
            #![proptest_config(ProptestConfig{ ..Default::default()})]

            #[test]
            fn commutativity(a in dotctx_strategy(), b in dotctx_strategy()) {
                let ab = a.merge(&b);
                let ba = b.merge(&a);

                assert_eq!(ab, ba);
            }

            #[test]
            fn associativity(a in dotctx_strategy(), b in dotctx_strategy(), c in dotctx_strategy()) {
                let ab_c = a.merge(&b).merge(&c);
                let a_bc = a.merge(&b.merge(&c));

                assert_eq!(ab_c, a_bc);
            }

            #[test]
            fn idempotency(a in dotctx_strategy()) {
                let aa = a.merge(&a);

                assert_eq!(aa, a);
            }
        }
    }
}
