#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

#[tokio::test]
async fn zero_ttl_and_periodic_cleanup_reclaim_expired_entries() {
    let driver = MemoryDriver::default();
    driver
        .put("immediate", "expired", Some(0))
        .await
        .expect("memory put");
    assert!(driver.get("immediate").await.expect("memory get").is_none());
    assert!(!driver.store.contains_key("immediate"));

    driver.store.insert(
        "stale".to_string(),
        CacheEntry {
            value: Arc::new("expired".to_string()),
            expires_at: Some(Instant::now()),
        },
    );
    driver
        .operations_since_cleanup
        .store(256, Ordering::Relaxed);
    assert!(driver.get_sync("missing").is_none());
    assert!(driver.store.is_empty());
}

struct RejectingDriver {
    miss_before_put: bool,
}

#[async_trait]
impl CacheDriver for RejectingDriver {
    async fn get(&self, _key: &str) -> Result<Option<Arc<String>>, CacheError> {
        if self.miss_before_put {
            Ok(None)
        } else {
            Err(CacheError::Driver("get rejected".to_string()))
        }
    }

    async fn put(&self, _key: &str, _value: &str, _ttl: Option<u64>) -> Result<(), CacheError> {
        Err(CacheError::Driver("put rejected".to_string()))
    }

    async fn forget(&self, _key: &str) -> Result<(), CacheError> {
        Err(CacheError::Driver("forget rejected".to_string()))
    }

    async fn flush(&self) -> Result<(), CacheError> {
        Err(CacheError::Driver("flush rejected".to_string()))
    }

    async fn has(&self, _key: &str) -> Result<bool, CacheError> {
        Err(CacheError::Driver("has rejected".to_string()))
    }
}

#[tokio::test]
async fn custom_driver_failures_propagate_through_every_facade_operation() {
    let rejecting = Cache::custom(Box::new(RejectingDriver {
        miss_before_put: false,
    }));
    assert!(rejecting.get("key").await.is_err());
    assert!(rejecting.put("key", "value", None).await.is_err());
    assert!(rejecting.forget("key").await.is_err());
    assert!(rejecting.flush().await.is_err());
    assert!(rejecting.has("key").await.is_err());

    let put_rejecting = Cache::custom(Box::new(RejectingDriver {
        miss_before_put: true,
    }));
    assert!(
        put_rejecting
            .remember("key", 60, || async { Ok("computed".to_string()) })
            .await
            .is_err()
    );
}

#[cfg(feature = "cache-redis")]
#[tokio::test]
async fn redis_operations_map_real_connection_refusals_to_typed_errors() {
    let cache = Cache::redis("redis://127.0.0.1:1/").expect("valid Redis URL");
    let get = cache.get("key").await;
    let set = cache.put("key", "value", None).await;
    let set_with_ttl = cache.put("key", "value", Some(5)).await;
    let forget = cache.forget("key").await;
    let flush = cache.flush().await;
    let has = cache.has("key").await;

    for result in [
        get.map(|_| ()),
        set,
        set_with_ttl,
        forget,
        flush,
        has.map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(CacheError::Driver(message)) if message.contains("Redis connection failed")
        ));
    }
}
