use async_trait::async_trait;
use dashmap::DashMap;

use super::driver::FeatureDriver;
use super::resolvers::{calculate_hash_bucket, split_variant};

// ─── Memory Driver ──────────────────────────────────────────────────────────

struct MemoryFlagRule {
    enabled: bool,
    rollout_percentage: Option<u32>,
    variants: Option<Vec<(String, u32)>>,
}

/// Memory-backed feature flag driver. Perfect for programmatic overrides and tests.
#[non_exhaustive]
pub struct MemoryFeatureDriver {
    rules: DashMap<String, MemoryFlagRule>,
}

impl MemoryFeatureDriver {
    /// Creates a new `MemoryFeatureDriver`.
    pub fn new() -> Self {
        Self {
            rules: DashMap::new(),
        }
    }

    /// Explicitly override a flag state.
    pub fn override_enabled(&self, flag: &str, enabled: bool) {
        self.rules.insert(
            flag.to_string(),
            MemoryFlagRule {
                enabled,
                rollout_percentage: None,
                variants: None,
            },
        );
    }

    /// Explicitly override a percentage rollout rule (e.g. 30%).
    pub fn override_rollout(&self, flag: &str, percentage: u32) {
        self.rules.insert(
            flag.to_string(),
            MemoryFlagRule {
                enabled: true,
                rollout_percentage: Some(percentage),
                variants: None,
            },
        );
    }

    /// Explicitly override an A/B split configuration (e.g. [("a", 50), ("b", 50)]).
    ///
    /// Read the assignment with `variant`. As in the Env, TOML and DB drivers,
    /// `enabled_for` is true only for identifiers assigned a variant named
    /// `"enabled"`; as in the Env and TOML drivers, `enabled` (no identifier)
    /// is false.
    pub fn override_variants(&self, flag: &str, variants: Vec<(String, u32)>) {
        self.rules.insert(
            flag.to_string(),
            MemoryFlagRule {
                enabled: true,
                rollout_percentage: None,
                variants: Some(variants),
            },
        );
    }
}

impl Default for MemoryFeatureDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FeatureDriver for MemoryFeatureDriver {
    async fn enabled(&self, flag: &str) -> Option<bool> {
        self.rules
            .get(flag)
            .map(|r| r.enabled && r.rollout_percentage.is_none() && r.variants.is_none())
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn enabled_for(&self, flag: &str, identifier: &str) -> Option<bool> {
        let rule = self.rules.get(flag)?;
        if !rule.enabled {
            return Some(false);
        }
        // An A/B split evaluates to a variant name, as in the string drivers.
        if let Some(ref variants) = rule.variants {
            return Some(split_variant(variants, flag, identifier) == "enabled");
        }
        if let Some(pct) = rule.rollout_percentage {
            let bucket = calculate_hash_bucket(flag, identifier);
            return Some(bucket < pct);
        }
        Some(rule.enabled)
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn variant(&self, flag: &str, identifier: &str) -> Option<String> {
        let rule = self.rules.get(flag)?;
        if !rule.enabled {
            return Some("disabled".to_string());
        }
        if let Some(ref variants) = rule.variants {
            return Some(split_variant(variants, flag, identifier));
        }
        if let Some(pct) = rule.rollout_percentage {
            let bucket = calculate_hash_bucket(flag, identifier);
            return Some(if bucket < pct {
                "enabled".to_string()
            } else {
                "disabled".to_string()
            });
        }
        Some(if rule.enabled {
            "enabled".to_string()
        } else {
            "disabled".to_string()
        })
    }
}
