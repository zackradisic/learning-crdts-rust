use std::{
    cmp::Ordering,
    ops::{Deref, DerefMut},
};

use super::grow_counter::GrowCounter;

#[derive(Clone, Debug)]
pub struct VectorClock(GrowCounter);

impl VectorClock {
    pub fn new(grow: GrowCounter) -> Self {
        Self(grow)
    }

    /// A value of None means the two clocks mark two logical times that are concurrent
    fn partial_ord_impl(a: &Self, b: &Self) -> Option<Ordering> {
        let all_keys = a.0.keys().chain(b.0.keys());
        all_keys.fold(Some(Ordering::Equal), |prev, key| {
            let va = a.0.get(key).copied().unwrap_or_default();
            let vb = b.0.get(key).copied().unwrap_or_default();

            // If all values of corresponding replicas are equal, clocks are equal
            // If all values of a <= all values of b, a is less than b
            // If all values of b >= a, b is greater than a
            // Any other mix is concurrent (returns None)
            match prev {
                Some(Ordering::Equal) if va > vb => Some(Ordering::Greater),
                Some(Ordering::Equal) if va < vb => Some(Ordering::Less),
                Some(Ordering::Less) if va > vb => None,
                Some(Ordering::Greater) if va < vb => None,
                _ => prev,
            }
        })
    }

    pub fn merge(&self, other: &Self) -> Self {
        VectorClock(self.0.merge(&other.0))
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Deref for VectorClock {
    type Target = GrowCounter;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VectorClock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PartialEq for VectorClock {
    fn eq(&self, other: &Self) -> bool {
        matches!(Self::partial_ord_impl(self, other), Some(Ordering::Equal))
    }
}

impl PartialOrd for VectorClock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Self::partial_ord_impl(self, other)
    }
}

#[cfg(test)]
mod test {

    use crate::state::grow_counter::GrowCounter;

    use super::VectorClock;

    #[test]
    fn cmp_less_greater() {
        let a = VectorClock::new(GrowCounter::from_iter([
            (0.into(), 0),
            (1.into(), 1),
            (2.into(), 420),
        ]));

        let b = VectorClock::new(GrowCounter::from_iter([
            (0.into(), 55),
            (1.into(), 69),
            (2.into(), 420),
        ]));

        assert!(a < b);
        assert!(b < a);
    }

    #[test]
    fn cmp_eq() {
        let a = VectorClock::new(GrowCounter::from_iter([
            (0.into(), 0),
            (1.into(), 1),
            (2.into(), 420),
        ]));

        let b = VectorClock::new(GrowCounter::from_iter([
            (0.into(), 0),
            (1.into(), 1),
            (2.into(), 420),
        ]));

        assert!(b == a);
    }

    #[test]
    fn cmp_concurrent() {
        let a = VectorClock::new(GrowCounter::from_iter([
            (0.into(), 55),
            (1.into(), 1),
            (2.into(), 420),
        ]));

        let b = VectorClock::new(GrowCounter::from_iter([
            (0.into(), 0),
            (1.into(), 69),
            (2.into(), 420),
        ]));

        assert!(b != a);
        assert_eq!(a.partial_cmp(&b), None)
    }
}
