use fp_bindgen::prelude::*;
use fp_bindgen::types::CargoDependency;
use fp_bindgen::{prelude::Serializable, TsExtendedRuntimeConfig};
use serde::{Deserialize, Serialize};
use sypytkowski_blog::delta_state::dot::Dot;

// #[derive(Serializable)]
// #[fp(rust_plugin_module = "ligma")]
// struct Dot {
//     replica: u32,
//     counter: u32,
// }

// impl Dot {
//     pub fn new() -> Self {
//         Self {
//             replica: 0,
//             counter: 0,
//         }
//     }
// }

fp_import! {}

fp_export! {
    fn test(dot: Dot) -> Dot;
}

fn main() {
    let bindings = [
        BindingsType::RustPlugin(RustPluginConfig {
            name: "ligma",
            authors: "[\"zackoverflow\"]",
            version: "0.0.1",
            dependencies: [(
                "ligma",
                CargoDependency {
                    path: Some("../ligma"),
                    ..Default::default()
                },
            )]
            .into(),
        }),
        BindingsType::TsRuntimeWithExtendedConfig(TsExtendedRuntimeConfig::default()),
    ];

    for bindings_type in bindings {
        let path = format!("ligma-protocol/bindings/{}", bindings_type);

        fp_bindgen!(fp_bindgen::BindingConfig {
            bindings_type,
            path: &path
        });
    }
}
