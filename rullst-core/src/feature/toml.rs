use async_trait::async_trait;
use dashmap::DashMap;

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
#[non_exhaustive]
pub struct TomlFeatureDriver {
    pub(crate) config: DashMap<String, String>,
    config_path: std::path::PathBuf,
}

impl TomlFeatureDriver {
    /// Creates a new `TomlFeatureDriver` and parses `Rullst.toml` if present.
    pub fn new() -> Self {
        let config_path = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("Rullst.toml");
        let driver = Self {
            config: DashMap::new(),
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
        self.config.clear();
        let mut in_features = false;
        for line in content.lines() {
            let trimmed = line.trim();
            #[cfg_attr(mutants, mutants::skip)]
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed == "[features]" {
                in_features = true;
                continue;
            }
            if trimmed.starts_with('[') {
                in_features = false;
                continue;
            }

            if in_features {
                let mut parts = trimmed.splitn(2, '=');
                if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                    let k = key.trim().to_string();
                    let clean_val = val.split('#').next().unwrap_or(val).trim();
                    let v = clean_val.trim_matches('"').trim_matches('\'').to_string();
                    self.config.insert(k, v);
                }
            }
        }
    }

    fn evaluate(&self, value: &str, flag: &str, identifier: Option<&str>) -> Option<String> {
        parse_feature_string_value(value, flag, identifier)
    }
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
        let val = self.config.get(flag)?;
        let evaluated = self.evaluate(val.value(), flag, None)?;
        Some(evaluated == "enabled")
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn enabled_for(&self, flag: &str, identifier: &str) -> Option<bool> {
        let val = self.config.get(flag)?;
        let evaluated = self.evaluate(val.value(), flag, Some(identifier))?;
        Some(evaluated == "enabled")
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn variant(&self, flag: &str, identifier: &str) -> Option<String> {
        let val = self.config.get(flag)?;
        self.evaluate(val.value(), flag, Some(identifier))
    }
}
