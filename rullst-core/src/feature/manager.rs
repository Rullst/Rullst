#[cfg(feature = "orm")]
use super::db::DbFeatureDriver;
use super::driver::FeatureDriver;
use super::env::EnvFeatureDriver;
use super::memory::MemoryFeatureDriver;
use super::toml::TomlFeatureDriver;
use async_trait::async_trait;
use std::sync::Arc;

// ─── Feature Manager & Facade ────────────────────────────────────────────────

/// The primary feature flags manager coordinating the driver pipeline.
#[non_exhaustive]
pub struct FeatureManager {
    drivers: Vec<Box<dyn FeatureDriver>>,
    overrides: Option<Arc<MemoryFeatureDriver>>,
}

/// The default pipeline's override layer, shared with [`FeatureManager::overrides`].
struct SharedOverrides(Arc<MemoryFeatureDriver>);

#[async_trait]
impl FeatureDriver for SharedOverrides {
    async fn enabled(&self, flag: &str) -> Option<bool> {
        self.0.enabled(flag).await
    }

    async fn enabled_for(&self, flag: &str, identifier: &str) -> Option<bool> {
        self.0.enabled_for(flag, identifier).await
    }

    async fn variant(&self, flag: &str, identifier: &str) -> Option<String> {
        self.0.variant(flag, identifier).await
    }
}

impl FeatureManager {
    /// Creates a new `FeatureManager` with empty drivers.
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
            overrides: None,
        }
    }

    /// The first-priority `MemoryFeatureDriver` of the [`Default`] pipeline,
    /// for programmatic and test overrides through this manager (including the
    /// global [`crate::feature::manager`]). `None` for a manager built with
    /// [`FeatureManager::new`], whose drivers are all supplied by the caller.
    ///
    /// Unpublished v13 API.
    pub fn overrides(&self) -> Option<&MemoryFeatureDriver> {
        self.overrides.as_deref()
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
    ///
    /// The memory layer is reachable through [`FeatureManager::overrides`].
    fn default() -> Self {
        let overrides = Arc::new(MemoryFeatureDriver::new());
        let mut manager = Self::new()
            .add_driver(Box::new(SharedOverrides(Arc::clone(&overrides))))
            .add_driver(Box::new(EnvFeatureDriver::new()))
            .add_driver(Box::new(TomlFeatureDriver::new()));
        manager.overrides = Some(overrides);

        #[cfg(feature = "orm")]
        let manager = manager.add_driver(Box::new(DbFeatureDriver::new()));

        manager
    }
}
