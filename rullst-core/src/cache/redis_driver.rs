//! Redis-backed cache driver. Requires the `cache-redis` feature.

use super::*;
use std::collections::BTreeSet;

const MAX_SCAN_ROUNDS: usize = 64;

/// Cache driver backed by Redis.
///
/// Uses `SET`/`GET` with `EX` for TTL support. Ideal for distributed
/// multi-instance deployments where cache must be shared.
pub struct RedisDriver {
    client: redis::Client,
    prefix: String,
}

impl RedisDriver {
    /// Creates a Redis cache driver with the fixed `rullst:cache:` namespace.
    pub fn new(redis_url: impl Into<String>) -> Result<Self, CacheError> {
        let redis_url = redis_url.into();
        let client = redis::Client::open(redis_url)
            .map_err(|error| CacheError::Driver(format!("Failed to connect to Redis: {error}")))?;
        Ok(Self {
            client,
            prefix: "rullst:cache:".to_string(),
        })
    }

    #[cfg_attr(mutants, mutants::skip)]
    fn prefixed_key(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }

    async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, CacheError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| CacheError::Driver(format!("Redis connection failed: {error}")))
    }
}

#[async_trait]
impl CacheDriver for RedisDriver {
    #[cfg_attr(mutants, mutants::skip)]
    async fn get(&self, key: &str) -> Result<Option<Arc<String>>, CacheError> {
        let mut connection = self.connection().await?;
        let result: Option<String> = redis::cmd("GET")
            .arg(self.prefixed_key(key))
            .query_async(&mut connection)
            .await
            .map_err(|error| CacheError::Driver(format!("Redis GET failed: {error}")))?;
        Ok(result.map(Arc::new))
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn put(&self, key: &str, value: &str, ttl_secs: Option<u64>) -> Result<(), CacheError> {
        let mut connection = self.connection().await?;
        let prefixed_key = self.prefixed_key(key);
        if let Some(ttl) = ttl_secs {
            redis::cmd("SETEX")
                .arg(&prefixed_key)
                .arg(ttl)
                .arg(value)
                .query_async::<()>(&mut connection)
                .await
                .map_err(|error| CacheError::Driver(format!("Redis SETEX failed: {error}")))?;
        } else {
            redis::cmd("SET")
                .arg(&prefixed_key)
                .arg(value)
                .query_async::<()>(&mut connection)
                .await
                .map_err(|error| CacheError::Driver(format!("Redis SET failed: {error}")))?;
        }
        Ok(())
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn forget(&self, key: &str) -> Result<(), CacheError> {
        let mut connection = self.connection().await?;
        redis::cmd("UNLINK")
            .arg(self.prefixed_key(key))
            .query_async::<i64>(&mut connection)
            .await
            .map_err(|error| CacheError::Driver(format!("Redis UNLINK failed: {error}")))?;
        Ok(())
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn flush(&self) -> Result<(), CacheError> {
        let mut connection = self.connection().await?;
        let pattern = format!("{}*", self.prefix);
        let mut cursor = 0_u64;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut connection)
                .await
                .map_err(|error| CacheError::Driver(format!("Redis SCAN failed: {error}")))?;
            if !keys.is_empty() {
                redis::cmd("UNLINK")
                    .arg(&keys)
                    .query_async::<i64>(&mut connection)
                    .await
                    .map_err(|error| CacheError::Driver(format!("Redis UNLINK failed: {error}")))?;
            }
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }
        Ok(())
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn has(&self, key: &str) -> Result<bool, CacheError> {
        let mut connection = self.connection().await?;
        redis::cmd("EXISTS")
            .arg(self.prefixed_key(key))
            .query_async(&mut connection)
            .await
            .map_err(|error| CacheError::Driver(format!("Redis EXISTS failed: {error}")))
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn inspect(&self, limit: usize) -> Result<CacheInspection, CacheError> {
        inspection::validate_inspection_limit(limit)?;
        let mut connection = self.connection().await?;
        let pattern = format!("{}*", self.prefix);
        let mut cursor = 0_u64;
        let mut keys = BTreeSet::new();
        let mut rounds = 0_usize;
        loop {
            let (next_cursor, page): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(limit.min(100))
                .query_async(&mut connection)
                .await
                .map_err(|error| CacheError::Driver(format!("Redis SCAN failed: {error}")))?;
            keys.extend(page);
            cursor = next_cursor;
            rounds += 1;
            if cursor == 0 || keys.len() > limit || rounds >= MAX_SCAN_ROUNDS {
                break;
            }
        }
        let truncated = cursor != 0 || keys.len() > limit;
        let mut entries = Vec::with_capacity(keys.len().min(limit));
        for prefixed_key in keys.into_iter().take(limit) {
            let value_bytes: u64 = redis::cmd("STRLEN")
                .arg(&prefixed_key)
                .query_async(&mut connection)
                .await
                .map_err(|error| CacheError::Driver(format!("Redis STRLEN failed: {error}")))?;
            let ttl_ms: i64 = redis::cmd("PTTL")
                .arg(&prefixed_key)
                .query_async(&mut connection)
                .await
                .map_err(|error| CacheError::Driver(format!("Redis PTTL failed: {error}")))?;
            if ttl_ms == -2 {
                continue;
            }
            let logical_key = prefixed_key
                .strip_prefix(&self.prefix)
                .ok_or_else(|| CacheError::Driver("Redis cache namespace mismatch".to_string()))?;
            entries.push(CacheEntryMetadata::new(
                logical_key.to_string(),
                usize::try_from(value_bytes).unwrap_or(usize::MAX),
                u64::try_from(ttl_ms).ok(),
            ));
        }
        Ok(CacheInspection::new(entries, truncated))
    }
}
