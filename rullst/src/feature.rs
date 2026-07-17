use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

// ─── Deterministic Hashing & Resolvers ──────────────────────────────────────────

/// Deterministically calculates a bucket index from 0 to 99 for a given flag and identifier.
/// This ensures a stable user-to-flag assignment without persistent storage.
pub fn calculate_hash_bucket(flag: &str, identifier: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    hasher.write(flag.as_bytes());
    hasher.write(identifier.as_bytes());
    let hash_val = hasher.finish();
    (hash_val % 100) as u32
}

/// Parses a rollout percentage string (e.g. "30%") into a number.
pub fn parse_rollout(s: &str) -> Option<u32> {
    let cleaned = s.trim().trim_end_matches('%');
    cleaned.parse::<u32>().ok()
}

/// Parses an A/B split configuration string (e.g. "variant-a:50,variant-b:50")
/// into a vector of variant names and their percentage weights.
pub fn parse_variants(s: &str) -> Vec<(String, u32)> {
    let mut parsed = Vec::new();
    for part in s.split(',') {
        let mut split = part.split(':');
        if let (Some(name), Some(pct_str)) = (split.next(), split.next())
            && let Ok(pct) = pct_str.trim().parse::<u32>()
        {
            parsed.push((name.trim().to_string(), pct));
        }
    }
    parsed
}

/// Evaluates a hash bucket index against a list of variants and returns the matching name.
pub fn resolve_variant(variants: &[(String, u32)], bucket: u32) -> Option<String> {
    let mut accumulator = 0;
    for (name, pct) in variants {
        accumulator += pct;
        if bucket < accumulator {
            return Some(name.clone());
        }
    }
    None
}

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
            .map(|r| r.enabled && r.rollout_percentage.is_none())
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn enabled_for(&self, flag: &str, identifier: &str) -> Option<bool> {
        let rule = self.rules.get(flag)?;
        if !rule.enabled {
            return Some(false);
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
            let bucket = calculate_hash_bucket(flag, identifier);
            return resolve_variant(variants, bucket);
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

/// Helper function to parse feature toggles string formats uniformly
fn parse_feature_string_value(value: &str, flag: &str, identifier: Option<&str>) -> Option<String> {
    let cleaned = value.trim();
    if cleaned.is_empty() {
        return None;
    }

    // 1. Check if simple boolean
    #[cfg_attr(mutants, mutants::skip)]
    if cleaned == "true" || cleaned == "1" || cleaned == "yes" {
        return Some("enabled".to_string());
    }
    if cleaned == "false" || cleaned == "0" || cleaned == "no" {
        return Some("disabled".to_string());
    }

    // 2. Check if percentage rollout (e.g., "30%")
    if cleaned.ends_with('%')
        && let Some(pct) = parse_rollout(cleaned)
    {
        if let Some(ident) = identifier {
            let bucket = calculate_hash_bucket(flag, ident);
            return Some(if bucket < pct {
                "enabled".to_string()
            } else {
                "disabled".to_string()
            });
        }
        return Some("disabled".to_string());
    }

    // 3. Check if A/B splits (e.g., "variant-a:50,variant-b:50")
    if cleaned.contains(':') {
        let variants = parse_variants(cleaned);
        if !variants.is_empty()
            && let Some(ident) = identifier
        {
            let bucket = calculate_hash_bucket(flag, ident);
            return resolve_variant(&variants, bucket);
        }
    }

    Some(cleaned.to_string())
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
    config: DashMap<String, String>,
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

    fn load_from_str(&self, content: &str) {
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

    async fn variant(&self, flag: &str, identifier: &str) -> Option<String> {
        let val = self.config.get(flag)?;
        self.evaluate(val.value(), flag, Some(identifier))
    }
}

// ─── Database Driver (with local TTL caching) ───────────────────────────────

struct DbCacheValue {
    enabled: bool,
    rollout_percentage: Option<u32>,
    variants: Option<String>,
    expires_at: Instant,
}

/// Feature flag driver backed by a database table `rullst_feature_flags`.
///
/// Features a high-performance concurrent local cache with custom TTL to ensure sub-millisecond lookups.
///
/// # Note on Database Pool Initialization
/// This driver requires a live database pool to function. If feature flags are evaluated before the
/// database connection pool has been initialized (e.g., in early application startup or static constructors),
/// this driver will gracefully return `None` (falling through to subsequent drivers in the chain)
/// rather than blocking or panicking.
#[non_exhaustive]
pub struct DbFeatureDriver {
    cache: DashMap<String, DbCacheValue>,
    ttl: Duration,
}

impl DbFeatureDriver {
    /// Creates a new `DbFeatureDriver` with a default cache TTL of 5 seconds.
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            ttl: Duration::from_secs(5),
        }
    }

    /// Creates a new `DbFeatureDriver` with a custom cache TTL duration.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            ttl,
        }
    }

    async fn fetch_flag_from_db(&self, flag: &str) -> Option<(bool, Option<u32>, Option<String>)> {
        use sqlx::Row;

        let pool = crate::db::safe_pool()?;
        let row = sqlx::query(
            "SELECT enabled, rollout_percentage, variants FROM rullst_feature_flags WHERE name = ?",
        )
        .bind(flag)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()?;

        // Resolve enabled column safely (support int 0/1 or boolean)
        let enabled = row
            .try_get::<i32, _>("enabled")
            .map(|v| v != 0)
            .or_else(|_| row.try_get::<bool, _>("enabled"))
            .unwrap_or(false);

        let rollout_percentage = row
            .try_get::<i32, _>("rollout_percentage")
            .map(|v| Some(v as u32))
            .unwrap_or(None);

        let variants = row.try_get::<String, _>("variants").ok();

        Some((enabled, rollout_percentage, variants))
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn resolve_flag(&self, flag: &str) -> Option<(bool, Option<u32>, Option<String>)> {
        if let Some(entry) = self.cache.get(flag)
            && Instant::now() < entry.expires_at
        {
            return Some((
                entry.enabled,
                entry.rollout_percentage,
                entry.variants.clone(),
            ));
        }

        // Cache miss or expired — fetch fresh from DB
        let (enabled, rollout, variants) = self.fetch_flag_from_db(flag).await?;
        self.cache.insert(
            flag.to_string(),
            DbCacheValue {
                enabled,
                rollout_percentage: rollout,
                variants: variants.clone(),
                expires_at: Instant::now() + self.ttl,
            },
        );

        Some((enabled, rollout, variants))
    }

    #[cfg_attr(mutants, mutants::skip)]
    fn evaluate(
        &self,
        enabled: bool,
        rollout: Option<u32>,
        variants: Option<String>,
        flag: &str,
        identifier: Option<&str>,
    ) -> Option<String> {
        if !enabled {
            return Some("disabled".to_string());
        }

        if let Some(vars_str) = variants {
            let vars = parse_variants(&vars_str);
            if !vars.is_empty()
                && let Some(ident) = identifier
            {
                let bucket = calculate_hash_bucket(flag, ident);
                return resolve_variant(&vars, bucket);
            }
        }

        if let Some(pct) = rollout {
            if let Some(ident) = identifier {
                let bucket = calculate_hash_bucket(flag, ident);
                return Some(if bucket < pct {
                    "enabled".to_string()
                } else {
                    "disabled".to_string()
                });
            }
            return Some("disabled".to_string());
        }

        Some(if enabled {
            "enabled".to_string()
        } else {
            "disabled".to_string()
        })
    }
}

impl Default for DbFeatureDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FeatureDriver for DbFeatureDriver {
    #[cfg_attr(mutants, mutants::skip)]
    async fn enabled(&self, flag: &str) -> Option<bool> {
        let (enabled, rollout, variants) = self.resolve_flag(flag).await?;
        let evaluated = self.evaluate(enabled, rollout, variants, flag, None)?;
        Some(evaluated == "enabled")
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn enabled_for(&self, flag: &str, identifier: &str) -> Option<bool> {
        let (enabled, rollout, variants) = self.resolve_flag(flag).await?;
        let evaluated = self.evaluate(enabled, rollout, variants, flag, Some(identifier))?;
        Some(evaluated == "enabled")
    }

    async fn variant(&self, flag: &str, identifier: &str) -> Option<String> {
        let (enabled, rollout, variants) = self.resolve_flag(flag).await?;
        self.evaluate(enabled, rollout, variants, flag, Some(identifier))
    }
}

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
    /// 4. `DbFeatureDriver` (database-backed flags, requires initialized database pool)
    fn default() -> Self {
        Self::new()
            .add_driver(Box::new(MemoryFeatureDriver::new()))
            .add_driver(Box::new(EnvFeatureDriver::new()))
            .add_driver(Box::new(TomlFeatureDriver::new()))
            .add_driver(Box::new(DbFeatureDriver::new()))
    }
}

// ─── Static Facade Interface ────────────────────────────────────────────────

static FEATURE_CELL: OnceLock<FeatureManager> = OnceLock::new();

/// Globally sets the framework's `FeatureManager` instance.
pub fn init(manager: FeatureManager) -> Result<(), FeatureManager> {
    FEATURE_CELL.set(manager)
}

/// Retrieves the static `FeatureManager` instance, lazy-initializing it if necessary.
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_hash_bucket() {
        let b1 = calculate_hash_bucket("flag-a", "user-1");
        let b2 = calculate_hash_bucket("flag-a", "user-1");
        let b3 = calculate_hash_bucket("flag-a", "user-2");
        assert_eq!(b1, b2);
        assert!(b1 < 100);
        assert!(b3 < 100);
    }

    #[test]
    fn test_parse_rollout() {
        assert_eq!(parse_rollout("30%"), Some(30));
        assert_eq!(parse_rollout("  100% "), Some(100));
        assert_eq!(parse_rollout("0"), Some(0));
        assert_eq!(parse_rollout("abc"), None);
    }

    #[test]
    fn test_parse_variants() {
        let parsed = parse_variants("control:50,treatment:50");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "control");
        assert_eq!(parsed[0].1, 50);
        assert_eq!(parsed[1].0, "treatment");
        assert_eq!(parsed[1].1, 50);

        let parsed_empty = parse_variants("invalid");
        assert!(parsed_empty.is_empty());
    }

    #[test]
    fn test_resolve_variant() {
        let variants = vec![("control".to_string(), 30), ("treatment".to_string(), 70)];
        // bucket 10 should fall into control (0-29)
        assert_eq!(resolve_variant(&variants, 10), Some("control".to_string()));
        // bucket 50 should fall into treatment (30-99)
        assert_eq!(
            resolve_variant(&variants, 50),
            Some("treatment".to_string())
        );
        // bucket 101 is out of bounds
        assert_eq!(resolve_variant(&variants, 101), None);
    }

    #[tokio::test]
    async fn test_memory_driver_override_enabled() {
        let driver = MemoryFeatureDriver::new();
        assert_eq!(driver.enabled("test-flag").await, None);

        driver.override_enabled("test-flag", true);
        assert_eq!(driver.enabled("test-flag").await, Some(true));
        assert_eq!(driver.enabled_for("test-flag", "user1").await, Some(true));
        assert_eq!(
            driver.variant("test-flag", "user1").await,
            Some("enabled".to_string())
        );

        driver.override_enabled("test-flag-2", false);
        assert_eq!(driver.enabled("test-flag-2").await, Some(false));
        assert_eq!(
            driver.enabled_for("test-flag-2", "user1").await,
            Some(false)
        );
        assert_eq!(
            driver.variant("test-flag-2", "user1").await,
            Some("disabled".to_string())
        );
    }

    #[tokio::test]
    async fn test_memory_driver_rollout() {
        let driver = MemoryFeatureDriver::new();
        driver.override_rollout("rollout-flag", 50); // 50%

        let bucket = calculate_hash_bucket("rollout-flag", "user-in");
        // We just verify it doesn't crash and returns boolean based on bucket
        let res = driver.enabled_for("rollout-flag", "user-in").await.unwrap();
        assert_eq!(res, bucket < 50);

        let variant = driver.variant("rollout-flag", "user-in").await.unwrap();
        assert_eq!(variant, if bucket < 50 { "enabled" } else { "disabled" });
    }

    #[tokio::test]
    async fn test_memory_driver_variants() {
        let driver = MemoryFeatureDriver::new();
        driver.override_variants("variant-flag", vec![("a".to_string(), 100)]);
        assert_eq!(
            driver.variant("variant-flag", "user1").await,
            Some("a".to_string())
        );
    }

    #[test]
    fn test_parse_feature_string_value() {
        assert_eq!(
            parse_feature_string_value(" true ", "f", None),
            Some("enabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value(" 0 ", "f", None),
            Some("disabled".to_string())
        );
        assert_eq!(parse_feature_string_value("", "f", None), None);
        assert_eq!(
            parse_feature_string_value("100%", "f", Some("u")),
            Some("enabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value("0%", "f", Some("u")),
            Some("disabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value("0%", "f", None),
            Some("disabled".to_string())
        );
        assert_eq!(
            parse_feature_string_value("a:100", "f", Some("u")),
            Some("a".to_string())
        );
        assert_eq!(
            parse_feature_string_value("custom-string", "f", None),
            Some("custom-string".to_string())
        );
    }

    #[tokio::test]
    async fn test_env_driver() {
        let driver = EnvFeatureDriver::new();
        unsafe {
            std::env::set_var("FEATURE_MY_FLAG", "true");
        }
        assert_eq!(driver.enabled("my-flag").await, Some(true));
        assert_eq!(driver.enabled_for("my-flag", "u").await, Some(true));
        assert_eq!(
            driver.variant("my-flag", "u").await,
            Some("enabled".to_string())
        );
    }

    #[tokio::test]
    async fn test_feature_manager() {
        let driver = MemoryFeatureDriver::new();
        driver.override_enabled("global-flag", true);

        let manager = FeatureManager::new().add_driver(Box::new(driver));
        assert!(manager.enabled("global-flag").await);
        assert!(manager.enabled_for("global-flag", "u").await);
        assert_eq!(
            manager.variant("global-flag", "u").await.unwrap(),
            "enabled"
        );

        assert!(!manager.enabled("unknown").await);
        assert!(!manager.enabled_for("unknown", "u").await);
        assert_eq!(manager.variant("unknown", "u").await, None);
    }

    #[test]
    fn test_feature_init() {
        let manager1 = FeatureManager::new();
        let _ = super::init(manager1);
        let manager2 = FeatureManager::new();
        assert!(super::init(manager2).is_err());
        let _m = super::manager();
    }

    #[test]
    fn test_resolve_variant_boundary() {
        let variants = vec![("a".to_string(), 50), ("b".to_string(), 50)];
        // If bucket is exactly 50, it should hit the second variant because accumulator for 'a' is 50,
        // and 50 < 50 is false. So it moves to 'b'.
        let v = resolve_variant(&variants, 50);
        assert_eq!(v, Some("b".to_string()));
    }

    #[tokio::test]
    async fn test_toml_driver_empty_lines() {
        let driver = TomlFeatureDriver::new();
        driver.load_from_str("\n\n[features]\nflag = true\n# comment\n");
        assert_eq!(driver.enabled("flag").await, Some(true));
    }
}
