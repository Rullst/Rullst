//! # Rullst Cache System (`rullst::cache`)
//!
//! Provides a unified caching API with pluggable drivers.
//!
//! ## Drivers
//! - **In-Memory** (default): `DashMap`-based concurrent store with TTL support. Zero config.
//! - **Redis** (optional): Requires the `cache-redis` feature flag.
//!
//! ## Quick Start
//! ```rust,ignore
//! use rullst::cache::Cache;
//!
//! let cache = Cache::memory();
//!
//! // Store a value with 60-second TTL
//! cache.put("user:42:name", "Alice", Some(60)).await?;
//!
//! // Retrieve it
//! let name = cache.get("user:42:name").await?; // Some("Alice")
//!
//! // Cache-aside pattern: fetch from cache or compute + store
//! let value = cache.remember("expensive_key", 300, || async {
//!     Ok("computed_value".to_string())
//! }).await?;
//! ```

use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;

// ─── Error Types ────────────────────────────────────────────────────────────

/// Errors that can occur during cache operations.
#[derive(Debug)]
pub enum CacheError {
    /// The underlying driver encountered an error.
    Driver(String),
    /// Serialization or deserialization failed.
    Serialization(String),
}

impl std::fmt::Display for CacheError {
    #[cfg_attr(mutants, mutants::skip)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::Driver(msg) => write!(f, "Cache driver error: {}", msg),
            CacheError::Serialization(msg) => write!(f, "Cache serialization error: {}", msg),
        }
    }
}

impl std::error::Error for CacheError {}

// ─── Cache Driver Trait ─────────────────────────────────────────────────────

/// Abstraction over cache storage backends.
///
/// Implement this trait to add support for new cache backends.
/// The framework ships with `MemoryDriver` and (optionally) `RedisDriver`.
#[async_trait]
pub trait CacheDriver: Send + Sync {
    /// Retrieve a value by key. Returns `None` if the key doesn't exist or has expired.
    async fn get(&self, key: &str) -> Result<Option<Arc<String>>, CacheError>;
    /// Store a value with an optional TTL in seconds.
    async fn put(&self, key: &str, value: &str, ttl_secs: Option<u64>) -> Result<(), CacheError>;
    /// Remove a key from the cache.
    async fn forget(&self, key: &str) -> Result<(), CacheError>;
    /// Remove all keys from the cache.
    async fn flush(&self) -> Result<(), CacheError>;
    /// Check if a key exists and is not expired.
    async fn has(&self, key: &str) -> Result<bool, CacheError>;
}

// ─── In-Memory Driver ───────────────────────────────────────────────────────

/// Cache entry holding the value and optional expiration time.
#[derive(Clone)]
struct CacheEntry {
    value: Arc<String>,
    expires_at: Option<Instant>,
}

/// In-memory cache driver using `DashMap` for lock-free concurrent access.
///
/// Supports TTL-based expiration. Expired entries are lazily cleaned on access.
/// Perfect for single-instance deployments and development.
pub struct MemoryDriver {
    store: DashMap<String, CacheEntry>,
}

impl MemoryDriver {
    /// Create a new in-memory cache driver.
    pub fn new() -> Self {
        let store: DashMap<String, CacheEntry> = DashMap::new();

        // Spawn active background janitor task to clean up expired cache entries from memory
        if tokio::runtime::Handle::try_current().is_ok() {
            let store_clone = store.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    // Retain only unexpired or eternal entries
                    store_clone.retain(|_, entry| {
                        entry.expires_at.map_or(true, |exp| Instant::now() < exp)
                    });
                }
            });
        }

        Self { store }
    }
}

impl Default for MemoryDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CacheDriver for MemoryDriver {
    #[cfg_attr(mutants, mutants::skip)]
    async fn get(&self, key: &str) -> Result<Option<Arc<String>>, CacheError> {
        if let Some(entry) = self.store.get(key) {
            // Check TTL expiration
            if let Some(expires_at) = entry.expires_at
                && Instant::now() > expires_at
            {
                // Entry has expired — remove it lazily
                drop(entry);
                self.store.remove(key);
                return Ok(None);
            }
            // Cheap pointer clone instead of deep string copy
            Ok(Some(entry.value.clone()))
        } else {
            Ok(None)
        }
    }

    async fn put(&self, key: &str, value: &str, ttl_secs: Option<u64>) -> Result<(), CacheError> {
        let expires_at = ttl_secs.map(|secs| Instant::now() + std::time::Duration::from_secs(secs));
        self.store.insert(
            key.to_string(),
            CacheEntry {
                value: Arc::new(value.to_string()),
                expires_at,
            },
        );
        Ok(())
    }

    async fn forget(&self, key: &str) -> Result<(), CacheError> {
        self.store.remove(key);
        Ok(())
    }

    async fn flush(&self) -> Result<(), CacheError> {
        self.store.clear();
        Ok(())
    }

    async fn has(&self, key: &str) -> Result<bool, CacheError> {
        Ok(self.get(key).await?.is_some())
    }
}

// ─── Redis Driver (behind feature flag) ─────────────────────────────────────

#[cfg(feature = "cache-redis")]
pub mod redis_driver {
    //! Redis-backed cache driver. Requires the `cache-redis` feature.
    use super::*;

    /// Cache driver backed by Redis.
    ///
    /// Uses `SET`/`GET` with `EX` for TTL support. Ideal for distributed
    /// multi-instance deployments where cache must be shared.
    pub struct RedisDriver {
        client: redis::Client,
        prefix: String,
    }

    impl RedisDriver {
        /// Create a new Redis cache driver.
        ///
        /// All keys are prefixed with `rullst:cache:` to avoid collisions.
        pub fn new(redis_url: &str) -> Result<Self, CacheError> {
            let client = redis::Client::open(redis_url)
                .map_err(|e| CacheError::Driver(format!("Failed to connect to Redis: {}", e)))?;
            Ok(Self {
                client,
                prefix: "rullst:cache:".to_string(),
            })
        }

        #[cfg_attr(mutants, mutants::skip)]
        fn prefixed_key(&self, key: &str) -> String {
            format!("{}{}", self.prefix, key)
        }
    }

    #[async_trait]
    impl CacheDriver for RedisDriver {
        #[cfg_attr(mutants, mutants::skip)]
        async fn get(&self, key: &str) -> Result<Option<Arc<String>>, CacheError> {
            let mut con = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| CacheError::Driver(format!("Redis connection failed: {}", e)))?;
            let result: Option<String> = redis::cmd("GET")
                .arg(self.prefixed_key(key))
                .query_async(&mut con)
                .await
                .map_err(|e| CacheError::Driver(format!("Redis GET failed: {}", e)))?;
            Ok(result.map(Arc::new))
        }

        #[cfg_attr(mutants, mutants::skip)]
        async fn put(
            &self,
            key: &str,
            value: &str,
            ttl_secs: Option<u64>,
        ) -> Result<(), CacheError> {
            let mut con = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| CacheError::Driver(format!("Redis connection failed: {}", e)))?;
            let pk = self.prefixed_key(key);
            if let Some(ttl) = ttl_secs {
                redis::cmd("SETEX")
                    .arg(&pk)
                    .arg(ttl as i64)
                    .arg(value)
                    .query_async::<()>(&mut con)
                    .await
                    .map_err(|e| CacheError::Driver(format!("Redis SETEX failed: {}", e)))?;
            } else {
                redis::cmd("SET")
                    .arg(&pk)
                    .arg(value)
                    .query_async::<()>(&mut con)
                    .await
                    .map_err(|e| CacheError::Driver(format!("Redis SET failed: {}", e)))?;
            }
            Ok(())
        }

        async fn forget(&self, key: &str) -> Result<(), CacheError> {
            let mut con = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| CacheError::Driver(format!("Redis connection failed: {}", e)))?;
            redis::cmd("UNLINK")
                .arg(self.prefixed_key(key))
                .query_async::<i64>(&mut con)
                .await
                .map_err(|e| CacheError::Driver(format!("Redis UNLINK failed: {}", e)))?;
            Ok(())
        }

        async fn flush(&self) -> Result<(), CacheError> {
            let mut con = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| CacheError::Driver(format!("Redis connection failed: {}", e)))?;
            let pattern = format!("{}*", self.prefix);
            let mut cursor: u64 = 0;
            loop {
                let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(&pattern)
                    .arg("COUNT")
                    .arg(100)
                    .query_async(&mut con)
                    .await
                    .map_err(|e| CacheError::Driver(format!("Redis SCAN failed: {}", e)))?;

                if !keys.is_empty() {
                    redis::cmd("UNLINK")
                        .arg(&keys)
                        .query_async::<i64>(&mut con)
                        .await
                        .map_err(|e| CacheError::Driver(format!("Redis UNLINK failed: {}", e)))?;
                }

                cursor = next_cursor;
                if cursor == 0 {
                    break;
                }
            }
            Ok(())
        }

        async fn has(&self, key: &str) -> Result<bool, CacheError> {
            let mut con = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| CacheError::Driver(format!("Redis connection failed: {}", e)))?;
            let exists: bool = redis::cmd("EXISTS")
                .arg(self.prefixed_key(key))
                .query_async(&mut con)
                .await
                .map_err(|e| CacheError::Driver(format!("Redis EXISTS failed: {}", e)))?;
            Ok(exists)
        }
    }
}

// ─── Cache Facade ───────────────────────────────────────────────────────────

/// The main cache facade for storing and retrieving cached values.
///
/// Provides a driver-agnostic API. Create with `Cache::memory()` or `Cache::redis()`.
///
/// # Thread Safety
/// The `Cache` is `Send + Sync` and can be safely shared across async tasks
/// and Axum handlers via `Arc` or Axum's `State`.
pub struct Cache {
    driver: Arc<Box<dyn CacheDriver>>,
}

impl Cache {
    /// Create a cache backed by an in-memory `DashMap`. Zero configuration.
    ///
    /// Data is lost on process restart. Perfect for development and single-instance apps.
    pub fn memory() -> Self {
        Self {
            driver: Arc::new(Box::new(MemoryDriver::new())),
        }
    }

    /// Create a cache backed by Redis. Requires the `cache-redis` feature.
    ///
    /// Data persists across restarts and is shared between instances.
    #[cfg(feature = "cache-redis")]
    pub fn redis(redis_url: &str) -> Result<Self, CacheError> {
        let driver = redis_driver::RedisDriver::new(redis_url)?;
        Ok(Self {
            driver: Arc::new(Box::new(driver)),
        })
    }

    /// Create a cache from any custom driver implementing `CacheDriver`.
    pub fn custom(driver: Box<dyn CacheDriver>) -> Self {
        Self {
            driver: Arc::new(driver),
        }
    }

    /// Retrieve a value by key.
    pub async fn get(&self, key: &str) -> Result<Option<Arc<String>>, CacheError> {
        self.driver.get(key).await
    }

    /// Store a value with an optional TTL in seconds.
    ///
    /// Pass `None` for TTL to store indefinitely.
    pub async fn put(
        &self,
        key: &str,
        value: &str,
        ttl_secs: Option<u64>,
    ) -> Result<(), CacheError> {
        self.driver.put(key, value, ttl_secs).await
    }

    /// Remove a key from the cache.
    pub async fn forget(&self, key: &str) -> Result<(), CacheError> {
        self.driver.forget(key).await
    }

    /// Remove all keys from the cache.
    pub async fn flush(&self) -> Result<(), CacheError> {
        self.driver.flush().await
    }

    /// Check if a key exists and has not expired.
    pub async fn has(&self, key: &str) -> Result<bool, CacheError> {
        self.driver.has(key).await
    }

    /// Retrieve a cached value, or compute it with the provided closure and cache the result.
    ///
    /// This is the **cache-aside** (or "remember") pattern — the most common caching strategy.
    ///
    /// # Example
    /// ```rust,ignore
    /// let bio = cache.remember("user:42:bio", 300, || async {
    ///     let user = User::find(42).await.map_err(|e| CacheError::Driver(e.to_string()))?;
    ///     Ok(user.bio)
    /// }).await?;
    /// ```
    pub async fn remember<F, Fut>(
        &self,
        key: &str,
        ttl_secs: u64,
        f: F,
    ) -> Result<Arc<String>, CacheError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<String, CacheError>>,
    {
        // Try the cache first
        if let Some(cached) = self.get(key).await? {
            return Ok(cached);
        }
        // Cache miss — compute the value
        let value = f().await?;
        // Store in cache
        self.put(key, &value, Some(ttl_secs)).await?;
        Ok(Arc::new(value))
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_cache_put_get() {
        let cache = Cache::memory();
        cache.put("key1", "value1", None).await.unwrap();
        let result = cache.get("key1").await.unwrap();
        assert_eq!(result, Some(Arc::new("value1".to_string())));
    }

    #[tokio::test]
    async fn test_memory_cache_miss() {
        let cache = Cache::memory();
        let result = cache.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_memory_cache_forget() {
        let cache = Cache::memory();
        cache.put("key1", "value1", None).await.unwrap();
        cache.forget("key1").await.unwrap();
        let result = cache.get("key1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_memory_cache_flush() {
        let cache = Cache::memory();
        cache.put("a", "1", None).await.unwrap();
        cache.put("b", "2", None).await.unwrap();
        cache.flush().await.unwrap();
        assert!(cache.get("a").await.unwrap().is_none());
        assert!(cache.get("b").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_memory_cache_has() {
        let cache = Cache::memory();
        assert!(!cache.has("key1").await.unwrap());
        cache.put("key1", "value1", None).await.unwrap();
        assert!(cache.has("key1").await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_cache_remember_miss() {
        let cache = Cache::memory();
        let value = cache
            .remember("computed", 60, || async { Ok("hello".to_string()) })
            .await
            .unwrap();
        assert_eq!(*value, "hello");
        // Should be cached now
        let cached = cache.get("computed").await.unwrap();
        assert_eq!(cached, Some(Arc::new("hello".to_string())));
    }

    #[tokio::test]
    async fn test_memory_cache_remember_hit() {
        let cache = Cache::memory();
        cache
            .put("existing", "already_cached", Some(300))
            .await
            .unwrap();
        let value = cache
            .remember("existing", 60, || async {
                panic!("This closure should NOT be called on cache hit");
            })
            .await
            .unwrap();
        assert_eq!(*value, "already_cached");
    }

    #[tokio::test]
    async fn test_memory_cache_overwrite() {
        let cache = Cache::memory();
        cache.put("key", "v1", None).await.unwrap();
        cache.put("key", "v2", None).await.unwrap();
        assert_eq!(
            cache.get("key").await.unwrap(),
            Some(Arc::new("v2".to_string()))
        );
    }

    struct MockDriver;
    #[async_trait]
    impl CacheDriver for MockDriver {
        async fn get(&self, _key: &str) -> Result<Option<Arc<String>>, CacheError> {
            Ok(Some(Arc::new("mocked".to_string())))
        }
        async fn put(&self, _k: &str, _v: &str, _t: Option<u64>) -> Result<(), CacheError> {
            Ok(())
        }
        async fn forget(&self, _k: &str) -> Result<(), CacheError> {
            Ok(())
        }
        async fn flush(&self) -> Result<(), CacheError> {
            Ok(())
        }
        async fn has(&self, _k: &str) -> Result<bool, CacheError> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_custom_cache_driver() {
        let cache = Cache::custom(Box::new(MockDriver));
        let result = cache.get("anything").await.unwrap();
        assert_eq!(result, Some(Arc::new("mocked".to_string())));
    }

    #[cfg(feature = "cache-redis")]
    #[test]
    fn test_redis_cache_initialization() {
        // Just verify that the constructor exists and returns a Result
        // We use an invalid URL so it fails parsing the connection string
        let result = Cache::redis("invalid-url-format://host:9999");
        assert!(result.is_err());
    }
}
