use crate::{
    delta_state::dot::{Dot, DotCtx},
    ReplicaId,
};

/// A state-based CRDT list is not part of Bartosz Sypytkowski's blog series, this
/// is just an exercise for myself to see how it could be done.
///
/// It works nicely if you only ever push/pop from the end of the list, perhaps stack is
/// a better name
#[derive(Clone, Debug)]
pub struct List<V> {
    ctx: DotCtx,
    values: Vec<(Dot, V)>,
    /// this isn't used anymore can probably get rid of it
    tombstone: (Dot, usize),
}

impl<V> Default for List<V> {
    fn default() -> Self {
        let mut ctx = DotCtx::default();
        let starting_dot = Dot(0.into(), 0);
        ctx.add(starting_dot);

        Self {
            ctx,
            values: Default::default(),
            tombstone: (starting_dot, 0),
        }
    }
}

impl<V: Clone + std::fmt::Debug> List<V> {
    pub fn values_iter(&self) -> impl Iterator<Item = &V> {
        self.values.iter().map(|(_, v)| v)
    }

    pub fn update(&mut self, replica: ReplicaId, value: V, index: usize) {
        let dot = self.ctx.next_dot(replica);
        self.values[index] = (dot, value);
    }

    pub fn push(&mut self, replica: ReplicaId, value: V) {
        let dot = self.ctx.next_dot(replica);
        self.values.push((dot, value));
        self.tombstone = (dot, self.values.len());
    }

    pub fn insert(&mut self, replica: ReplicaId, value: V, index: usize) {
        let dot = self.ctx.next_dot(replica);
        self.values.insert(index, (dot, value));
        self.values.iter_mut().skip(index + 1).for_each(|(d, _)| {
            *d = dot;
        });
        self.tombstone = (dot, self.values.len());
    }

    pub fn pop(&mut self, replica: ReplicaId) -> Option<V> {
        let dot = self.ctx.next_dot(replica);
        match self.values.pop() {
            Some(val) => {
                self.tombstone = (dot, self.values.len());
                Some(val.1)
            }
            None => None,
        }
    }

    pub fn merge(&self, other: &Self) -> Self {
        let mut values: Vec<(Dot, V)> = vec![];
        let mut self_iter = self.internal_iter();
        let mut other_iter = other.internal_iter();
        let mut tombstone = self.tombstone;

        loop {
            let (a, b) = (self_iter.next(), other_iter.next());
            match (a, b) {
                (Some((_, None)), Some((_, _))) => {
                    values.extend(other_iter.filter_map(|(dot, val)| match val {
                        Some(val) if !self.ctx.contains(dot) => Some((dot, val.clone())),
                        _ => None,
                    }));
                    // if other.ctx.contains(self_dot) {
                    //     values.extend(other_iter.map(|(dot, val)| (dot, val.unwrap().clone())));
                    //     tombstone = other.tombstone;
                    //     break;
                    // }
                    break;
                }
                (Some((_, _)), Some((_, None))) => {
                    values.extend(self_iter.filter_map(|(dot, val)| match val {
                        Some(val) if !other.ctx.contains(dot) => Some((dot, val.clone())),
                        _ => None,
                    }));
                    // if self.ctx.contains(other_dot) {
                    //     values.extend(self_iter.map(|(dot, val)| (dot, val.unwrap().clone())));
                    //     break;
                    // }
                    tombstone = other.tombstone;
                    break;
                }
                (Some((self_dot, Some(self_val))), Some((other_dot, Some(other_val)))) => {
                    if self_dot == other_dot {
                        values.push((self_dot, self_val.clone()));
                    } else if self.ctx.contains(other_dot) {
                        values.push((self_dot, self_val.clone()));
                    } else {
                        values.push((other_dot, other_val.clone()));
                    }
                }
                (_, _) => unreachable!(),
            }
        }

        Self {
            ctx: self.ctx.merge(&other.ctx),
            values,
            tombstone,
        }
    }

    fn internal_iter(&self) -> InternalListIter<V> {
        InternalListIter { idx: 0, list: self }
    }
}

struct InternalListIter<'a, V> {
    idx: u32,
    list: &'a List<V>,
}

impl<'a, V> Iterator for InternalListIter<'a, V> {
    type Item = (Dot, Option<&'a V>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.list.values.len() as u32 {
            let val = &self.list.values[self.idx as usize];
            self.idx += 1;
            Some((val.0, Some(&val.1)))
        } else if self.idx == self.list.values.len() as u32 {
            self.idx += 1;
            let tombstone = &self.list.tombstone;
            Some((tombstone.0, None))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{delta_state::dot::Dot, ReplicaGenerator};

    use super::List;

    #[test]
    fn basic() {
        let mut gen = ReplicaGenerator::new();
        let a_id = gen.gen();
        let b_id = gen.gen();
        let mut a: List<&str> = List::default();

        a.push(a_id, "apple");
        a.push(a_id, "orange");
        a.push(a_id, "lime");

        let mut b = a.clone();
        let lime = b.pop(b_id);
        assert_eq!(lime, Some("lime"));

        let c = a.merge(&b);
        let values = c.values.iter().collect::<Vec<_>>();

        assert_eq!(
            values,
            vec![&(Dot(a_id, 1), "apple"), &(Dot(a_id, 2), "orange"),]
        );
    }

    /// Actually starts at 1 not 0:
    /// A: (a, 0, apple) (a, 1, orange) (a, 2, lime) (a, 2, tombstone)
    /// B: (a, 0, apple) (a, 1, orange) (a, 2, lime) (a, 2, tombstone)
    /// B: (a, 0, apple) (b, 0, lime) (b, 0, tombstone)
    ///
    /// MERGE:
    /// (a, 0, apple) (a, 1, orange) (a, 2, lime) (a, 2, tombstone) :: a
    /// (a, 0, apple) (b, 0, lime) (b, 0, tombstone) :: b
    /// (a, 0, apple) (b, 0, lime) (b, 0, tombstone) :: new
    ///
    /// MERGE:
    /// (a, 0, apple) (a, 1, orange) (a, 2, lime) (a, 3, strawberry) (a, 3, tombstone) :: (a, 3)
    /// (a, 0, apple) (b, 0, lime) (b, 0, tombstone) :: (a, 3) (b, 0)
    /// (a, 0, apple) (b, 0, lime) (a, 3, strawberry) (b, 0, tombstone) :: new
    ///
    /// MERGE:
    /// (a, 0, apple) (a, 1, orange) (a, 2, lime) (a, 3, strawberry) (a, 3, tombstone) :: (a, 3)
    /// (a, 0, apple) (b, 0, lime) (b, 0, tombstone) :: (a, 2) (b, 0)
    /// (a, 0, apple) (b, 0, lime) (a, 3, strawberry) (b, 0, tombstone) :: new
    #[test]
    fn basic2() {
        let mut gen = ReplicaGenerator::new();
        let a_id = gen.gen();
        let b_id = gen.gen();
        let mut a: List<&str> = List::default();

        a.push(a_id, "apple");
        a.push(a_id, "orange");
        a.push(a_id, "lime");

        let mut b = a.clone();
        let lime = b.pop(b_id);
        assert_eq!(lime, Some("lime"));
        a.push(a_id, "strawberry");

        let c = a.merge(&b);
        let values = c.values.iter().collect::<Vec<_>>();

        assert_eq!(
            values,
            vec![
                &(Dot(a_id, 1), "apple"),
                &(Dot(a_id, 2), "orange"),
                &(Dot(a_id, 4), "strawberry")
            ]
        );
    }
}
