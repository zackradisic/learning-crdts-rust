#![allow(unused_imports)]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Dot {
    pub replica: u32,
    pub counter: u32,
}
