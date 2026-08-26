//! # Rullst Cache System (`rullst::cache`)
//!
//! Provides a unified caching API with pluggable drivers.
//!
//! ## Drivers
//! - **In-Memory** (default): `DashMap`-based concurrent store with TTL support. Zero config.
//! - **Redis** (optional): Requires the `cache-redis` feature flag.
//!
//! ## Quick Start
//! ```rust,no_run
//! use rullst_core::cache::{Cache, CacheError};
//!
//! async fn cache_example() -> Result<(), CacheError> {
//!     let cache = Cache::memory();
//!
//!     cache.put("user:42:name", "Alice", Some(60)).await?;
//!     let name = cache.get("user:42:name").await?;
//!     assert_eq!(name.as_deref().map(String::as_str), Some("Alice"));
//!
//!     let value = cache.remember("expensive_key", 300, || async {
//!         Ok("computed_value".to_string())
//!     }).await?;
//!     assert_eq!(value.as_str(), "computed_value");
//!
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Instant;

// ─── Error Types ────────────────────────────────────────────────────────────

/// Errors that can occur during cache operations.
#[derive(Debug)]
#[non_exhaustive]
pub enum CacheError {
    /// The underlying driver encountered an error.
    Driver(String),
    /// Serialization or deserialization failed.
    Serialization(String),
}

impl std::fmt::Display for CacheError {
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
    operations_since_cleanup: AtomicUsize,
}

impl MemoryDriver {
    /// Create a new in-memory cache driver.
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
            operations_since_cleanup: AtomicUsize::new(0),
        }
    }

    /// Reclaims expired entries opportunistically without spawning a task that
    /// could outlive the cache. Entries are also removed immediately when read.
    fn cleanup_if_due(&self) {
        const CLEANUP_INTERVAL_OPERATIONS: usize = 256;
        let previous = self
            .operations_since_cleanup
            .fetch_add(1, Ordering::Relaxed);
        if previous > 0 && previous.is_multiple_of(CLEANUP_INTERVAL_OPERATIONS) {
            let now = Instant::now();
            self.store
                .retain(|_, entry| entry.expires_at.is_none_or(|expires_at| now < expires_at));
        }
    }

    fn get_sync(&self, key: &str) -> Option<Arc<String>> {
        self.cleanup_if_due();
        let entry = self.store.get(key)?;
        if entry
            .expires_at
            .is_some_and(|expires_at| Instant::now() >= expires_at)
        {
            drop(entry);
            self.store.remove(key);
            return None;
        }
        Some(Arc::clone(&entry.value))
    }

    fn put_sync(&self, key: &str, value: &str, ttl_secs: Option<u64>) {
        self.cleanup_if_due();
        let expires_at = ttl_secs.map(|secs| Instant::now() + std::time::Duration::from_secs(secs));
        self.store.insert(
            key.to_string(),
            CacheEntry {
                value: Arc::new(value.to_string()),
                expires_at,
            },
        );
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
        Ok(self.get_sync(key))
    }

    async fn put(&self, key: &str, value: &str, ttl_secs: Option<u64>) -> Result<(), CacheError> {
        self.put_sync(key, value, ttl_secs);
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

// ─── Global Memoize Cache ───────────────────────────────────────────────────

/// Global memory cache functions used by the `#[memoize]` macro.
pub mod memory {
    use super::MemoryDriver;
    use std::sync::OnceLock;

    static GLOBAL_MEMO_CACHE: OnceLock<MemoryDriver> = OnceLock::new();

    fn get_cache() -> &'static MemoryDriver {
        GLOBAL_MEMO_CACHE.get_or_init(MemoryDriver::new)
    }

    /// Retrieve a value from the global memoize cache.
    pub fn get(key: &str) -> Option<String> {
        get_cache().get_sync(key).map(|value| value.to_string())
    }

    /// Store a value in the global memoize cache.
    pub fn set(key: &str, value: &str) {
        get_cache().put_sync(key, value, Some(3600));
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
        pub fn new(redis_url: impl Into<String>) -> Result<Self, CacheError> {
            let redis_url = redis_url.into();
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

        #[cfg_attr(mutants, mutants::skip)]
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

        #[cfg_attr(mutants, mutants::skip)]
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

        #[cfg_attr(mutants, mutants::skip)]
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
    #[cfg_attr(mutants, mutants::skip)]
    pub fn redis(redis_url: impl Into<String>) -> Result<Self, CacheError> {
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
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use rullst_core::cache::{Cache, CacheError};
    ///
    /// async fn load_bio(cache: &Cache) -> Result<Arc<String>, CacheError> {
    ///     cache.remember("user:42:bio", 300, || async {
    ///         Ok("Example biography".to_string())
    ///     }).await
    /// }
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

    #[test]
    fn test_cache_error_display() {
        assert_eq!(
            CacheError::Driver("failed".into()).to_string(),
            "Cache driver error: failed"
        );
        assert_eq!(
            CacheError::Serialization("bad json".into()).to_string(),
            "Cache serialization error: bad json"
        );
    }

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

    #[tokio::test]
    async fn test_memory_cache_remember_error() {
        let cache = Cache::memory();
        let result = cache
            .remember("error_key", 60, || async {
                Err(CacheError::Driver("computation failed".to_string()))
            })
            .await;

        assert!(result.is_err());
        if let Err(CacheError::Driver(msg)) = result {
            assert_eq!(msg, "computation failed");
        }

        // Ensure nothing was cached
        let cached = cache.get("error_key").await.unwrap();
        assert!(cached.is_none());
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

    #[test]
    fn test_memory_cache() {
        memory::set("test_mem_key", "test_mem_val");
        assert_eq!(memory::get("test_mem_key").unwrap(), "test_mem_val");
        assert_eq!(memory::get("non_existent_mem_key"), None);
    }

    #[tokio::test]
    async fn global_memoize_cache_is_safe_inside_a_tokio_runtime() {
        memory::set("runtime_mem_key", "runtime_mem_val");
        assert_eq!(
            memory::get("runtime_mem_key").as_deref(),
            Some("runtime_mem_val")
        );
    }
}
