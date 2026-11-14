//! An unreachable Redis fails `Orm::init_redis_with_namespace` after one
//! reconnection attempt instead of the redis-rs default of six backed-off
//! retries (about 9.5 s on Linux). Port 1 on loopback refuses the connection
//! without any network access.
#![cfg(all(
    feature = "redis",
    not(any(feature = "strict-postgres", feature = "strict-mysql"))
))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::{Duration, Instant};

use rullst_orm::Orm;

#[tokio::test]
async fn an_unreachable_redis_fails_initialization_without_long_retries() {
    let started = Instant::now();
    let result = Orm::init_redis_with_namespace("redis://127.0.0.1:1", "outage-bound").await;
    let elapsed = started.elapsed();
    assert!(result.is_err(), "nothing listens on port 1");
    // Windows takes about two seconds to report a refused loopback connection,
    // so the bound leaves room for two attempts there.
    assert!(
        elapsed < Duration::from_secs(8),
        "initialization took {elapsed:?}"
    );
}
