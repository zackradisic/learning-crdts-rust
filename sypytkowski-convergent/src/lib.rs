#![feature(option_get_or_insert_default)]
#![feature(btree_drain_filter)]

use std::str::FromStr;

use fp_bindgen::prelude::Serializable;

pub mod delta_state;
pub mod state;

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    fp_bindgen::prelude::Serializable,
    serde_derive::Serialize,
    serde_derive::Deserialize,
)]
#[fp(rust_plugin_module = "sypytkowski_convergent")]
pub struct ReplicaId(u64);

impl From<u64> for ReplicaId {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

pub struct ReplicaGenerator {
    count: u64,
}

impl ReplicaGenerator {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn gen(&mut self) -> ReplicaId {
        let ret = self.count;
        self.count += 1;
        ReplicaId(ret)
    }
}

pub trait Value: fp_bindgen::prelude::Serializable {}

macro_rules! impl_value {
    ($($t:ty),*) => {
        $(
            impl Value for $t {}
        )*
    };
}

impl_value!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, String, bool);
