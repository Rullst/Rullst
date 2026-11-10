// src/generators/build/mod.rs — Build pipeline: upgrade and production binary.

pub(crate) mod precompressed;
mod production;
mod upgrade;

pub use production::run_production_build;
pub use upgrade::{UpgradeOptions, run_upgrade};
pub(crate) use upgrade::{
    prepare_manifests, validate_prepared_manifests, validate_prepared_resolution,
};
