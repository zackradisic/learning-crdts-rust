use sypytkowski_blog::delta_state::awormap::Deltas;

use crate::types::*;

#[fp_bindgen_support::fp_export_signature]
pub fn deltas() -> Deltas<SquareId, Square>;

#[fp_bindgen_support::fp_export_signature]
pub fn get() -> AWORMap<SquareId, Square>;

#[fp_bindgen_support::fp_export_signature]
pub fn merge(other: AWORMap<SquareId, Square>) -> AWORMap<SquareId, Square>;

#[fp_bindgen_support::fp_export_signature]
pub fn merge_deltas(delta: Deltas<SquareId, Square>);

#[fp_bindgen_support::fp_export_signature]
pub fn replace(map: AWORMap<SquareId, Square>);

#[fp_bindgen_support::fp_export_signature]
pub fn set(replica: ReplicaId, id: SquareId, square: Square);
