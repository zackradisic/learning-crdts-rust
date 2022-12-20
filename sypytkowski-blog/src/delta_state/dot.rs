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
use std::collections::{BTreeMap, BTreeSet};

use crate::ReplicaId;

pub type VectorClock = BTreeMap<ReplicaId, u64>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dot(pub ReplicaId, pub u64);

#[derive(Debug, Clone, PartialEq)]
pub struct DotKernel<V: Clone> {
    ctx: DotCtx,
    pub(crate) entries: BTreeMap<Dot, V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DotCtx {
    clock: VectorClock,
    dot_cloud: BTreeSet<Dot>,
}

impl<V: Clone + PartialEq + Default> Default for DotKernel<V> {
    fn default() -> Self {
        Self {
            ctx: Default::default(),
            entries: Default::default(),
        }
    }
}

impl<V: Clone + PartialEq + std::fmt::Debug> DotKernel<V> {
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

    pub fn remove(&mut self, value: V, delta: &mut Self) {
        let dot = self
            .entries
            .iter()
            .find(|(_, val)| val == &&value)
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
    use std::collections::BTreeSet;

    use proptest::{
        collection::{btree_map, btree_set},
        prelude::*,
    };

    use crate::ReplicaId;

    use super::{Dot, DotCtx, DotKernel, VectorClock};

    const MAX_VALUES: u64 = 1000;
    fn dot_strategy() -> impl Strategy<Value = Dot> {
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
    }
    fn dotctx_strategy() -> impl Strategy<Value = DotCtx> {
        (dotcloud_strategy(), vector_clock_strategy()).prop_map(|(dot_cloud, clock)| {
            let mut ctx = DotCtx { dot_cloud, clock };
            ctx.compact();
            ctx
        })
    }
    pub fn dotkernel_strategy<V: Clone + std::fmt::Debug>(
        value_strategy: impl Strategy<Value = V>,
    ) -> impl Strategy<Value = DotKernel<V>> {
        (
            dotctx_strategy(),
            btree_map(dot_strategy(), value_strategy, 0..(MAX_VALUES as usize)),
        )
            .prop_map(|(ctx, entries)| DotKernel { ctx, entries })
    }
    /// We can't have kernels that have different dot for the same value
    /// in the entries map, for example:
    ///
    /// Kernel 1 has (8, 9): 420
    /// Kernel 2 has (8, 9): 69
    ///
    /// This is an illegal state because it means the replica has two different values
    /// at the exact same time, which is impossible because adding to a replica
    /// increments the dot.
    pub fn patch_kernels<V: Clone + PartialEq>(kernels: &mut [&mut DotKernel<V>]) {
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
        for deletion in deletions {
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
