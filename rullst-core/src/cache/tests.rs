#![allow(clippy::expect_used, clippy::unwrap_used)]

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
    assert_eq!(
        CacheError::InvalidKey("empty".into()).to_string(),
        "Invalid cache key: empty"
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
async fn memory_inspection_is_sorted_bounded_metadata_only_and_redacted_in_debug() {
    let cache = Cache::memory();
    cache
        .put("private:user:42", "secret-value", None)
        .await
        .unwrap();
    cache.put("alpha", "abc", Some(60)).await.unwrap();
    cache.put("zeta", "longer", None).await.unwrap();

    let inspection = cache.inspect(2).await.unwrap();
    assert_eq!(inspection.entries().len(), 2);
    assert!(inspection.truncated());
    assert_eq!(inspection.entries()[0].logical_key(), "alpha");
    assert_eq!(inspection.entries()[0].value_bytes(), 3);
    assert!(inspection.entries()[0].remaining_ttl_ms().is_some());
    assert_eq!(inspection.entries()[1].logical_key(), "private:user:42");
    let debug = format!("{:?}", inspection.entries()[1]);
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("private:user:42"));
    assert!(!debug.contains("secret-value"));

    assert!(matches!(
        cache.inspect(0).await,
        Err(CacheError::InvalidInspectionLimit)
    ));
    assert!(matches!(
        cache.inspect(MAX_CACHE_INSPECTION_ENTRIES + 1).await,
        Err(CacheError::InvalidInspectionLimit)
    ));
}

#[tokio::test]
async fn test_memory_cache_remember_miss() {
    let cache = Cache::memory();
    let value = cache
        .remember("computed", 60, || async { Ok("hello".to_string()) })
        .await
        .unwrap();
    assert_eq!(*value, "hello");
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

    assert!(matches!(
        result,
        Err(CacheError::Driver(ref message)) if message == "computation failed"
    ));
    assert!(cache.get("error_key").await.unwrap().is_none());
}

struct MockDriver;

#[async_trait]
impl CacheDriver for MockDriver {
    async fn get(&self, _key: &str) -> Result<Option<Arc<String>>, CacheError> {
        Ok(Some(Arc::new("mocked".to_string())))
    }

    async fn put(&self, _key: &str, _value: &str, _ttl: Option<u64>) -> Result<(), CacheError> {
        Ok(())
    }

    async fn forget(&self, _key: &str) -> Result<(), CacheError> {
        Ok(())
    }

    async fn flush(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn has(&self, _key: &str) -> Result<bool, CacheError> {
        Ok(true)
    }
}

#[tokio::test]
async fn test_custom_cache_driver() {
    let cache = Cache::custom(Box::new(MockDriver));
    let result = cache.get("anything").await.unwrap();
    assert_eq!(result, Some(Arc::new("mocked".to_string())));
}

#[tokio::test]
async fn custom_cache_inspection_fails_explicitly_when_not_implemented() {
    let cache = Cache::custom(Box::new(MockDriver));
    assert!(matches!(
        cache.inspect(1).await,
        Err(CacheError::InspectionUnsupported)
    ));
}

#[cfg(feature = "cache-redis")]
#[test]
fn test_redis_cache_initialization() {
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
