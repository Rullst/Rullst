// src/generators/build/mod.rs — Build pipeline: upgrade, Wasm Islands, and production binary.

mod production;
mod upgrade;
mod wasm;

pub use production::run_production_build;
pub use upgrade::{UpgradeOptions, run_upgrade};
pub(crate) use upgrade::{
    prepare_manifests, validate_prepared_manifests, validate_prepared_resolution,
};
pub use wasm::run_build_client;
