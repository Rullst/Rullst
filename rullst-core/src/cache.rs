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

mod tenant;
pub use tenant::TenantCache;

// ─── Error Types ────────────────────────────────────────────────────────────

/// Errors that can occur during cache operations.
#[derive(Debug)]
#[non_exhaustive]
pub enum CacheError {
    /// The underlying driver encountered an error.
    Driver(String),
    /// Serialization or deserialization failed.
    Serialization(String),
    /// A tenant-scoped cache key was empty, oversized, or contained unsafe bytes.
    InvalidKey(String),
    /// The selected backend does not expose metadata inspection.
    InspectionUnsupported,
    /// A cache inspection requested zero or more than the fixed maximum.
    InvalidInspectionLimit,
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::Driver(msg) => write!(f, "Cache driver error: {}", msg),
            CacheError::Serialization(msg) => write!(f, "Cache serialization error: {}", msg),
            CacheError::InvalidKey(msg) => write!(f, "Invalid cache key: {}", msg),
            CacheError::InspectionUnsupported => {
                write!(f, "Cache metadata inspection is unsupported by this driver")
            }
            CacheError::InvalidInspectionLimit => write!(
                f,
                "Cache inspection limit must be between 1 and {MAX_CACHE_INSPECTION_ENTRIES}"
            ),
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
    /// Returns bounded metadata without values when the driver supports it.
    async fn inspect(&self, _limit: usize) -> Result<CacheInspection, CacheError> {
        Err(CacheError::InspectionUnsupported)
    }
}

#[path = "cache/inspection.rs"]
mod inspection;
pub use inspection::{CacheEntryMetadata, CacheInspection, MAX_CACHE_INSPECTION_ENTRIES};

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

    async fn inspect(&self, limit: usize) -> Result<CacheInspection, CacheError> {
        inspection::validate_inspection_limit(limit)?;
        self.cleanup_if_due();
        let now = Instant::now();
        let mut entries: Vec<_> = self
            .store
            .iter()
            .filter(|entry| entry.expires_at.is_none_or(|expires_at| expires_at > now))
            .map(|entry| {
                CacheEntryMetadata::new(
                    entry.key().clone(),
                    entry.value.len(),
                    entry
                        .expires_at
                        .map(|expires_at| expires_at.saturating_duration_since(now).as_millis())
                        .and_then(|ttl| u64::try_from(ttl).ok()),
                )
            })
            .collect();
        entries.sort_by(|left, right| left.logical_key.cmp(&right.logical_key));
        let truncated = entries.len() > limit;
        entries.truncate(limit);
        Ok(CacheInspection::new(entries, truncated))
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
#[path = "cache/redis_driver.rs"]
pub mod redis_driver;

// ─── Cache Facade ───────────────────────────────────────────────────────────

/// The main cache facade for storing and retrieving cached values.
///
/// Provides a driver-agnostic API. Create with `Cache::memory()` or `Cache::redis()`.
///
/// # Thread Safety
/// The `Cache` is `Send + Sync` and can be safely shared across async tasks
/// and Axum handlers via `Arc` or Axum's `State`.
#[derive(Clone)]
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

    /// Returns a bounded metadata-only snapshot when supported by the driver.
    pub async fn inspect(&self, limit: usize) -> Result<CacheInspection, CacheError> {
        self.driver.inspect(limit).await
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

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "cache_contract_tests.rs"]
mod contract_tests;
