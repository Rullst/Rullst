use async_trait::async_trait;

// ─── Feature Driver Trait ───────────────────────────────────────────────────

/// Abstraction over feature flag and A/B split configurations.
#[async_trait]
pub trait FeatureDriver: Send + Sync {
    /// Check if a feature flag is enabled.
    async fn enabled(&self, flag: &str) -> Option<bool>;

    /// Check if a feature flag is enabled for a specific target identifier.
    async fn enabled_for(&self, flag: &str, identifier: &str) -> Option<bool>;

    /// Retrieve the variation name assigned to a specific target identifier.
    async fn variant(&self, flag: &str, identifier: &str) -> Option<String>;
}
