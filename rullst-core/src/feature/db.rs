use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::driver::FeatureDriver;
use super::resolvers::{calculate_hash_bucket, parse_variants, resolve_variant};

// ─── Database Driver (with local TTL caching) ───────────────────────────────

static DB_FEATURE_CACHE_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Longest time one flag lookup may wait for a connection and its query.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);
/// Most flag names cached by one driver, including negative entries.
const MAX_CACHED_FLAGS: usize = 4_096;

/// `enabled`, `rollout_percentage` and `variants` of one stored flag.
type FlagRow = (bool, Option<u32>, Option<String>);

struct DbCacheValue {
    /// `None` caches a missing flag (or a failed lookup with no known value).
    flag: Option<FlagRow>,
    expires_at: Instant,
    epoch: u64,
}

enum Lookup {
    /// No database pool is initialized yet; nothing was queried.
    Unavailable,
    Found(FlagRow),
    Missing,
    /// The query failed or exceeded [`LOOKUP_TIMEOUT`].
    Failed,
}

/// Feature flag driver backed by a database table `rullst_feature_flags`.
///
/// Uses a concurrent process-local cache with a configurable TTL. Lookup
/// latency depends on contention, key/value size, hardware, and build profile.
/// A flag without a row, and a failed or timed-out lookup (for example a
/// missing table or an unavailable database), is cached for the same TTL, so
/// an undefined flag does not cost a query on every evaluation. After a failed
/// lookup the last value read for that flag keeps being served until a later
/// lookup succeeds. One lookup waits at most two seconds, and at most 4,096
/// flag names are cached per driver.
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

    /// Invalidates cached database flags in every `DbFeatureDriver` in this process.
    ///
    /// Call this only after the corresponding database transaction commits. Other processes and
    /// direct database writers remain visible through the configured TTL unless the application
    /// distributes this signal itself.
    pub fn invalidate_process_cache() {
        DB_FEATURE_CACHE_EPOCH.fetch_add(1, Ordering::AcqRel);
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn fetch_flag_from_db(&self, flag: &str) -> Lookup {
        use sqlx::Row;

        let Some(pool) = crate::db::safe_pool() else {
            return Lookup::Unavailable;
        };
        let sql = if crate::db::safe_driver() == Some("postgres") {
            "SELECT enabled, rollout_percentage, variants FROM rullst_feature_flags WHERE name = $1"
        } else {
            "SELECT enabled, rollout_percentage, variants FROM rullst_feature_flags WHERE name = ?"
        };
        let query = sqlx::query(sql).bind(flag).fetch_optional(pool);
        let row = match tokio::time::timeout(LOOKUP_TIMEOUT, query).await {
            Ok(Ok(Some(row))) => row,
            Ok(Ok(None)) => return Lookup::Missing,
            Ok(Err(_)) | Err(_) => return Lookup::Failed,
        };

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

        Lookup::Found((enabled, rollout_percentage, variants))
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn resolve_flag(&self, flag: &str) -> Option<FlagRow> {
        self.resolve_with(flag, || self.fetch_flag_from_db(flag))
            .await
    }

    /// Applies the cache policy around one `lookup` of `flag`.
    async fn resolve_with<F, Fut>(&self, flag: &str, lookup: F) -> Option<FlagRow>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Lookup>,
    {
        let epoch = DB_FEATURE_CACHE_EPOCH.load(Ordering::Acquire);
        let previous = self.cache.get(flag).map(|entry| {
            let fresh = Instant::now() < entry.expires_at && entry.epoch == epoch;
            (entry.flag.clone(), fresh)
        });
        if let Some((cached, true)) = previous {
            return cached;
        }

        let resolved = match lookup().await {
            Lookup::Unavailable => return None,
            Lookup::Found(row) => Some(row),
            Lookup::Missing => None,
            // Keep serving the last value read (stale-if-error) until the TTL
            // allows another attempt.
            Lookup::Failed => previous.and_then(|(cached, _)| cached),
        };
        self.remember(flag, resolved.clone(), epoch);
        resolved
    }

    fn remember(&self, flag: &str, value: Option<FlagRow>, epoch: u64) {
        let now = Instant::now();
        if !self.cache.contains_key(flag) && self.cache.len() >= MAX_CACHED_FLAGS {
            self.cache
                .retain(|_, entry| now < entry.expires_at && entry.epoch == epoch);
            if self.cache.len() >= MAX_CACHED_FLAGS {
                return;
            }
        }
        self.cache.insert(
            flag.to_string(),
            DbCacheValue {
                flag: value,
                expires_at: now + self.ttl,
                epoch,
            },
        );
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

#[cfg(test)]
#[path = "db_tests.rs"]
mod tests;
