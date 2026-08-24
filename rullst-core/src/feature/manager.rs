#[cfg(feature = "orm")]
use super::db::DbFeatureDriver;
use super::driver::FeatureDriver;
use super::env::EnvFeatureDriver;
use super::memory::MemoryFeatureDriver;
use super::toml::TomlFeatureDriver;

// ─── Feature Manager & Facade ────────────────────────────────────────────────

/// The primary feature flags manager coordinating the driver pipeline.
#[non_exhaustive]
pub struct FeatureManager {
    drivers: Vec<Box<dyn FeatureDriver>>,
}

impl FeatureManager {
    /// Creates a new `FeatureManager` with empty drivers.
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
        }
    }

    /// Adds a driver to the evaluation pipeline.
    pub fn add_driver(mut self, driver: Box<dyn FeatureDriver>) -> Self {
        self.drivers.push(driver);
        self
    }

    /// Check if a feature flag is enabled.
    pub async fn enabled(&self, flag: &str) -> bool {
        for driver in &self.drivers {
            if let Some(val) = driver.enabled(flag).await {
                return val;
            }
        }
        false
    }

    /// Check if a feature flag is enabled for a target identifier.
    pub async fn enabled_for(&self, flag: &str, identifier: &str) -> bool {
        for driver in &self.drivers {
            if let Some(val) = driver.enabled_for(flag, identifier).await {
                return val;
            }
        }
        false
    }

    /// Retrieve the variation name assigned to a target identifier.
    pub async fn variant(&self, flag: &str, identifier: &str) -> Option<String> {
        for driver in &self.drivers {
            if let Some(val) = driver.variant(flag, identifier).await {
                return Some(val);
            }
        }
        None
    }
}

impl Default for FeatureManager {
    /// Creates a new `FeatureManager` with safe, batteries-included defaults:
    /// 1. `MemoryFeatureDriver` (programmatic/testing overrides)
    /// 2. `EnvFeatureDriver` (environment variable configuration)
    /// 3. `TomlFeatureDriver` (local TOML file configuration via `Rullst.toml`)
    /// 4. `DbFeatureDriver` when the `orm` feature is enabled (database-backed
    ///    flags, requires an initialized database pool)
    fn default() -> Self {
        let manager = Self::new()
            .add_driver(Box::new(MemoryFeatureDriver::new()))
            .add_driver(Box::new(EnvFeatureDriver::new()))
            .add_driver(Box::new(TomlFeatureDriver::new()));

        #[cfg(feature = "orm")]
        let manager = manager.add_driver(Box::new(DbFeatureDriver::new()));

        manager
    }
}
