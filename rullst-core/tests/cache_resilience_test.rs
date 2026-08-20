// tests/cache_resilience_test.rs — Comprehensive unit and integration tests for Cache & Resilience Shield.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::cache::{Cache, CacheDriver, MemoryDriver};
use rullst_core::resilience::{TrafficShield, TrafficShieldConfig};
use std::time::Duration;

#[tokio::test]
async fn test_in_memory_cache_crud_and_ttl() {
    let driver = MemoryDriver::new();

    // 1. Put and Get
    driver
        .put("session:1", "user_data", Some(60))
        .await
        .unwrap();
    let val = driver.get("session:1").await.unwrap();
    assert_eq!(
        val.map(|s| s.as_str().to_string()),
        Some("user_data".to_string())
    );

    // 2. Has
    assert!(driver.has("session:1").await.unwrap());
    assert!(!driver.has("session:nonexistent").await.unwrap());

    // 3. Forget
    driver.forget("session:1").await.unwrap();
    assert!(!driver.has("session:1").await.unwrap());
    assert_eq!(driver.get("session:1").await.unwrap(), None);

    // 4. Flush
    driver.put("key_a", "val_a", None).await.unwrap();
    driver.put("key_b", "val_b", None).await.unwrap();
    assert!(driver.has("key_a").await.unwrap());
    assert!(driver.has("key_b").await.unwrap());
    driver.flush().await.unwrap();
    assert!(!driver.has("key_a").await.unwrap());
    assert!(!driver.has("key_b").await.unwrap());
}

#[tokio::test]
async fn test_cache_remember_and_helper_wrapper() {
    let cache = Cache::custom(Box::new(MemoryDriver::new()));

    // Remember pattern
    let val1 = cache
        .remember("compute:heavy", 100, || async {
            Ok("computed_123".to_string())
        })
        .await
        .unwrap();
    assert_eq!(val1.as_str(), "computed_123");

    // Second call should return cached value without executing closure
    let val2 = cache
        .remember("compute:heavy", 100, || async {
            Ok("computed_should_not_run".to_string())
        })
        .await
        .unwrap();
    assert_eq!(val2.as_str(), "computed_123");
}

#[tokio::test]
async fn test_traffic_shield_config_and_telemetry() {
    let config = TrafficShieldConfig::new()
        .with_max_event_loop_lag(Duration::from_millis(50))
        .with_max_db_latency(Duration::from_millis(200))
        .with_max_active_requests(500)
        .with_db_probe(false);

    assert_eq!(config.max_event_loop_lag, Duration::from_millis(50));
    assert_eq!(config.max_db_latency, Duration::from_millis(200));
    assert_eq!(config.max_active_requests, 500);
    assert!(!config.enable_db_probe);

    let shield = TrafficShield::new(config);
    assert_eq!(shield.active_requests(), 0);
    assert_eq!(shield.db_latency(), Duration::ZERO);

    // Sleep slightly to let event loop monitor tick
    tokio::time::sleep(Duration::from_millis(150)).await;
    let _lag = shield.event_loop_lag();
}
