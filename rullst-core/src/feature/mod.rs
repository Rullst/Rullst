//! Feature flags system for `rullst-core`.
//!
//! Provides a composable driver pipeline for feature flag evaluation:
//! memory overrides → env vars → TOML config → database.

mod db;
mod driver;
mod env;
mod manager;
mod memory;
mod resolvers;
mod toml;

#[cfg(test)]
mod tests;

// ─── Public Re-exports ──────────────────────────────────────────────────────

pub use db::DbFeatureDriver;
pub use driver::FeatureDriver;
pub use env::EnvFeatureDriver;
pub use manager::FeatureManager;
pub use memory::MemoryFeatureDriver;
pub use resolvers::{calculate_hash_bucket, parse_rollout, parse_variants, resolve_variant};
pub use toml::TomlFeatureDriver;

// ─── Static Facade Interface ────────────────────────────────────────────────

use std::sync::OnceLock;

static FEATURE_CELL: OnceLock<FeatureManager> = OnceLock::new();

/// Globally sets the framework's `FeatureManager` instance.
pub fn init(manager: FeatureManager) -> Result<(), FeatureManager> {
    FEATURE_CELL.set(manager)
}

/// Retrieves the static `FeatureManager` instance, lazy-initializing it if necessary.
#[cfg_attr(mutants, mutants::skip)]
pub fn manager() -> &'static FeatureManager {
    FEATURE_CELL.get_or_init(FeatureManager::default)
}

/// Checks if a feature flag is globally enabled.
#[cfg_attr(mutants, mutants::skip)]
pub async fn enabled(flag: &str) -> bool {
    manager().enabled(flag).await
}

/// Checks if a feature flag is enabled for a specific identifier (progressive rollout).
#[cfg_attr(mutants, mutants::skip)]
pub async fn enabled_for(flag: &str, identifier: &str) -> bool {
    manager().enabled_for(flag, identifier).await
}

/// Evaluates A/B split variations for a specific identifier.
#[cfg_attr(mutants, mutants::skip)]
pub async fn variant(flag: &str, identifier: &str) -> Option<String> {
    manager().variant(flag, identifier).await
}
