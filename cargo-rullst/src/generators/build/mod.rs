// src/generators/build/mod.rs — Build pipeline: upgrade, Wasm Islands, and production binary.

pub(crate) mod precompressed;
mod production;
mod upgrade;
mod wasm;

pub use production::run_production_build;
pub(crate) use upgrade::{
    AssistFinding, AssistPlan, assist_plan, prepare_manifests, validate_prepared_manifests,
    validate_prepared_resolution,
};
pub use upgrade::{UpgradeOptions, run_upgrade};
pub use wasm::run_build_client;
