#[path = "attributes/common.rs"]
mod common;
#[path = "attributes/field.rs"]
mod field;
#[path = "attributes/model.rs"]
mod model;

#[cfg(test)]
pub(super) use common::{split_top_level, strip_outer_call, validate_relation_attribute};
pub(super) use field::FieldAttributes;
pub(super) use model::ModelAttributes;
