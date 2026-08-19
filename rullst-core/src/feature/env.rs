use async_trait::async_trait;

use super::driver::FeatureDriver;
use super::resolvers::parse_feature_string_value;

// ─── Env Driver ─────────────────────────────────────────────────────────────

/// Driver that parses feature flags defined in environment variables.
///
/// Prefix variable names with `FEATURE_` (e.g. `FEATURE_NEW_UI=true`).
#[non_exhaustive]
pub struct EnvFeatureDriver;

impl EnvFeatureDriver {
    /// Creates a new `EnvFeatureDriver`.
    pub fn new() -> Self {
        Self
    }

    fn env_key(flag: &str) -> String {
        format!("FEATURE_{}", flag.to_uppercase().replace('-', "_"))
    }

    fn parse_env_value(&self, value: &str, flag: &str, identifier: Option<&str>) -> Option<String> {
        parse_feature_string_value(value, flag, identifier)
    }
}

impl Default for EnvFeatureDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FeatureDriver for EnvFeatureDriver {
    #[cfg_attr(mutants, mutants::skip)]
    async fn enabled(&self, flag: &str) -> Option<bool> {
        let key = Self::env_key(flag);
        let val = std::env::var(key).ok()?;
        let parsed = self.parse_env_value(&val, flag, None)?;
        Some(parsed == "enabled")
    }

    async fn enabled_for(&self, flag: &str, identifier: &str) -> Option<bool> {
        let key = Self::env_key(flag);
        let val = std::env::var(key).ok()?;
        let parsed = self.parse_env_value(&val, flag, Some(identifier))?;
        Some(parsed == "enabled")
    }

    async fn variant(&self, flag: &str, identifier: &str) -> Option<String> {
        let key = Self::env_key(flag);
        let val = std::env::var(key).ok()?;
        self.parse_env_value(&val, flag, Some(identifier))
    }
}
