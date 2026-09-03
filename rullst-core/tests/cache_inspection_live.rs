#![cfg(feature = "cache-redis")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_core::Cache;
use std::time::{SystemTime, UNIX_EPOCH};
use testcontainers::GenericImage;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;

struct RedisFixture {
    _container: Option<testcontainers::ContainerAsync<GenericImage>>,
    url: String,
}

async fn live_redis() -> Option<RedisFixture> {
    if let Ok(url) = std::env::var("RULLST_TEST_REDIS_URL") {
        return Some(RedisFixture {
            _container: None,
            url,
        });
    }
    let container = match GenericImage::new(
        "redis",
        "7.4-alpine@sha256:ff02b58f971e7d7d156a1267e283fcbbeee91773b6aa36c49dac28ecfe28eadf",
    )
    .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
    .with_exposed_port(6379.tcp())
    .start()
    .await
    {
        Ok(container) => container,
        Err(error) => {
            if std::env::var("RULLST_REQUIRE_TESTCONTAINERS").as_deref() == Ok("true") {
                panic!("Redis testcontainer is required but could not start: {error}");
            }
            eprintln!("skipping Redis cache inspection contract: {error}");
            return None;
        }
    };
    let host = container.get_host().await.expect("Redis host");
    let port = container
        .get_host_port_ipv4(6379)
        .await
        .expect("Redis port");
    Some(RedisFixture {
        _container: Some(container),
        url: format!("redis://{host}:{port}"),
    })
}

#[tokio::test]
async fn redis_inspection_returns_bounded_metadata_and_never_values() {
    let Some(redis) = live_redis().await else {
        return;
    };
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock")
        .as_nanos();
    let first_key = format!("inspection-{unique}-alpha");
    let second_key = format!("inspection-{unique}-zeta");
    let cache = Cache::redis(redis.url).expect("live Redis cache");
    cache
        .put(&first_key, "private-value", Some(60))
        .await
        .expect("first cache fixture");
    cache
        .put(&second_key, "abc", None)
        .await
        .expect("second cache fixture");

    let inspection = cache.inspect(200).await.expect("Redis inspection");
    let first = inspection
        .entries()
        .iter()
        .find(|entry| entry.logical_key() == first_key)
        .expect("first metadata");
    assert_eq!(first.value_bytes(), "private-value".len());
    assert!(first.remaining_ttl_ms().is_some());
    let second = inspection
        .entries()
        .iter()
        .find(|entry| entry.logical_key() == second_key)
        .expect("second metadata");
    assert_eq!(second.value_bytes(), 3);
    assert_eq!(second.remaining_ttl_ms(), None);
    let debug = format!("{inspection:?}");
    assert!(!debug.contains("private-value"));
    assert!(!debug.contains(&first_key));

    cache
        .forget(&first_key)
        .await
        .expect("remove first fixture");
    cache
        .forget(&second_key)
        .await
        .expect("remove second fixture");
}
