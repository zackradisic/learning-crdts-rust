pub trait Convergent {
    fn merge(&self, other: &Self) -> Self;
}

impl Convergent for u16 {
    fn merge(&self, other: &Self) -> Self {
        (*self).max(*other)
    }
}
