#![allow(unused_imports)]
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, collections::BTreeSet};
use sypytkowski_convergent::Value;

pub use sypytkowski_convergent::delta_state::awormap::AWORMap;
pub use sypytkowski_convergent::delta_state::awormap::KeyVal;
pub use sypytkowski_convergent::delta_state::dot::Dot;
pub use sypytkowski_convergent::delta_state::dot::VectorClock;
pub use sypytkowski_convergent::ReplicaId;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AWORSet<V: Clone + PartialEq + Default + Value> {
    pub kernel: DotKernel<V>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<DotKernel<V>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DotCtx {
    pub clock: VectorClock,
    pub dot_cloud: BTreeSet<Dot>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DotKernel<V: Clone + Value> {
    pub ctx: DotCtx,
    pub entries: BTreeMap<Dot, V>,
}

// stupid patch
#[derive(
    Clone,
    Debug,
    Deserialize,
    PartialEq,
    Serialize,
    Default,
    PartialOrd,
    fp_bindgen::prelude::Serializable,
)]
pub struct Square {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(
    Clone,
    Debug,
    Deserialize,
    PartialEq,
    Serialize,
    Default,
    Eq,
    PartialOrd,
    Ord,
    fp_bindgen::prelude::Serializable,
)]
pub struct SquareId(pub u32);

impl Value for Square {}
impl Value for SquareId {}
