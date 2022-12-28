pub trait Convergent {
    fn merge(&self, other: &Self) -> Self;
}

macro_rules! impl_convergent_num {
    ($($t:ty),*) => ($(
        impl Convergent for $t {
            fn merge(&self, other: &Self) -> Self {
                (*self).max(*other)
            }
        }
    )*)
}

impl_convergent_num!(u16, u32, u64, i16, i32, i64, f32, f64);
