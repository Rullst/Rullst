use async_trait::async_trait;
use dashmap::DashMap;
use std::time::{Duration, Instant};

use super::driver::FeatureDriver;
use super::resolvers::{calculate_hash_bucket, parse_variants, resolve_variant};

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
    /// Overrides the default TTL.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            ttl,
        }
    }

    #[cfg_attr(mutants, mutants::skip)]
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

    #[cfg_attr(mutants, mutants::skip)]
    async fn variant(&self, flag: &str, identifier: &str) -> Option<String> {
        let (enabled, rollout, variants) = self.resolve_flag(flag).await?;
        self.evaluate(enabled, rollout, variants, flag, Some(identifier))
    }
}
