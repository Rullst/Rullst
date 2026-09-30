use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{PoisonError, RwLock};

use super::driver::FeatureDriver;
use super::resolvers::parse_feature_string_value;

// ─── TOML Driver ────────────────────────────────────────────────────────────

/// Driver that parses feature flags defined in `Rullst.toml`.
///
/// Looks for keys under a `[features]` block:
/// ```toml
/// [features]
/// new-ui = true
/// ab-signup = "30%"
/// pricing-ab = "control:50,treatment:50"
/// ```
///
/// [`reload`](Self::reload) parses the file into a new map and swaps it in at
/// once, so a concurrent evaluation sees either the previous or the new flags,
/// never a partially loaded set. A section header may carry a trailing
/// `# comment`.
#[non_exhaustive]
pub struct TomlFeatureDriver {
    pub(crate) config: RwLock<HashMap<String, String>>,
    config_path: std::path::PathBuf,
}

impl TomlFeatureDriver {
    /// Creates a new `TomlFeatureDriver` and parses `Rullst.toml` if present.
    pub fn new() -> Self {
        let config_path = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("Rullst.toml");
        let driver = Self {
            config: RwLock::new(HashMap::new()),
            config_path,
        };

        if let Ok(content) = std::fs::read_to_string(&driver.config_path) {
            driver.load_from_str(&content);
        }

        driver
    }

    /// Reloads the features section from `Rullst.toml`.
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn reload(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = tokio::fs::read_to_string(&self.config_path).await?;
        self.load_from_str(&content);
        Ok(())
    }

    pub(crate) fn load_from_str(&self, content: &str) {
        let parsed = parse_features(content);
        *self.config.write().unwrap_or_else(PoisonError::into_inner) = parsed;
    }

    pub(crate) fn value(&self, flag: &str) -> Option<String> {
        self.config
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(flag)
            .cloned()
    }

    fn evaluate(&self, value: &str, flag: &str, identifier: Option<&str>) -> Option<String> {
        parse_feature_string_value(value, flag, identifier)
    }
}

/// Reads the `[features]` table of `Rullst.toml` into a new map.
fn parse_features(content: &str) -> HashMap<String, String> {
    let mut config = HashMap::new();
    let mut in_features = false;
    for line in content.lines() {
        let trimmed = line.trim();
        #[cfg_attr(mutants, mutants::skip)]
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') {
            // `[features] # comment` is still the features table.
            let header = trimmed.split('#').next().unwrap_or(trimmed).trim();
            in_features = header == "[features]";
            continue;
        }

        if in_features {
            let mut parts = trimmed.splitn(2, '=');
            if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                let k = key.trim().to_string();
                let clean_val = val.split('#').next().unwrap_or(val).trim();
                let v = clean_val.trim_matches('"').trim_matches('\'').to_string();
                config.insert(k, v);
            }
        }
    }
    config
}

impl Default for TomlFeatureDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FeatureDriver for TomlFeatureDriver {
    #[cfg_attr(mutants, mutants::skip)]
    async fn enabled(&self, flag: &str) -> Option<bool> {
        let val = self.value(flag)?;
        let evaluated = self.evaluate(&val, flag, None)?;
        Some(evaluated == "enabled")
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn enabled_for(&self, flag: &str, identifier: &str) -> Option<bool> {
        let val = self.value(flag)?;
        let evaluated = self.evaluate(&val, flag, Some(identifier))?;
        Some(evaluated == "enabled")
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn variant(&self, flag: &str, identifier: &str) -> Option<String> {
        let val = self.value(flag)?;
        self.evaluate(&val, flag, Some(identifier))
    }
}
