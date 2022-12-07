use std::collections::BTreeSet;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct GSet<T: Debug + Clone + Ord> {
    values: BTreeSet<T>,
    delta: Option<Box<GSet<T>>>,
}

impl<T: Debug + Clone + Ord> GSet<T> {
    pub fn value(&self) -> &BTreeSet<T> {
        &self.values
    }

    pub fn add(&mut self, val: T) {
        self.values.insert(val.clone());
        let deltas = self.delta.get_or_insert_default();
        deltas.values.insert(val);
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self::merge_impl(self, other)
    }

    fn merge_impl(a: &Self, b: &Self) -> Self {
        let mut values = a.values.clone();
        values.extend(b.values.iter().cloned());

        let delta = match (&a.delta, &b.delta) {
            (Some(x), Some(y)) => Some(Box::new(Self::merge_impl(&x, &y))),
            (Some(x), None) => Some(x.clone()),
            (None, Some(y)) => Some(y.clone()),
            (None, None) => None,
        };

        Self { values, delta }
    }

    fn split(&self) -> (Self, Option<GSet<T>>) {
        (
            Self {
                values: self.values.clone(),
                delta: None,
            },
            self.delta.clone().map(|d| *d),
        )
    }

    fn expect_split(&self) -> (Self, GSet<T>) {
        let (val, delta) = self.split();
        (val, delta.expect("Expected deltas"))
    }
}

impl<T: Debug + Clone + Ord> Default for GSet<T> {
    fn default() -> Self {
        Self {
            values: Default::default(),
            delta: Default::default(),
        }
    }
}
