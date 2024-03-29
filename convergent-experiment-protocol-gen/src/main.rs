use fp_bindgen::prelude::*;
use fp_bindgen::types::CargoDependency;
use fp_bindgen::{prelude::Serializable, TsExtendedRuntimeConfig};
use serde::{Deserialize, Serialize};
use sypytkowski_convergent::delta_state::awormap::{AWORMap, Deltas};
use sypytkowski_convergent::{ReplicaId, Value};

#[derive(Debug, Clone, PartialEq, Default, Serializable, Serialize, Deserialize)]
pub struct Square {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
#[derive(
    Debug, Clone, PartialEq, Default, Serializable, Serialize, Deserialize, Eq, PartialOrd, Ord,
)]
pub struct SquareId(pub u32);

impl Value for Square {}
impl Value for SquareId {}

fp_import! {
    fn log(str: String);
}

fp_export! {
    fn get() -> AWORMap<SquareId, Square>;
    fn set(replica: ReplicaId, id: SquareId, square: Square);
    fn del(replica: ReplicaId, id: SquareId);
    fn merge_deltas(delta: Deltas<SquareId, Square>);
    fn merge(other: AWORMap<SquareId, Square>) -> AWORMap<SquareId, Square>;
    fn deltas() -> Deltas<SquareId, Square>;
    fn replace(map: AWORMap<SquareId, Square>);
}

fn main() {
    let bindings = [
        (
            BindingsType::RustPlugin(RustPluginConfig {
                name: "convergent-experiment-protocol",
                authors: "[\"zackoverflow\"]",
                version: "0.0.1",
                dependencies: [(
                    "sypytkowski-convergent",
                    CargoDependency {
                        path: Some("../sypytkowski-convergent"),
                        features: ["wasm"].into(),
                        ..Default::default()
                    },
                )]
                .into(),
            }),
            "convergent-experiment-protocol",
        ),
        (
            BindingsType::TsRuntimeWithExtendedConfig(TsExtendedRuntimeConfig::default()),
            "convergent-experiment/frontend/src/lib/proto",
        ),
    ];

    for (bindings_type, path) in bindings.into_iter().skip(0) {
        fp_bindgen!(fp_bindgen::BindingConfig {
            bindings_type,
            path: &path
        });
    }
}
