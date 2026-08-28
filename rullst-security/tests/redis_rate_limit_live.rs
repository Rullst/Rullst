#![cfg(feature = "redis-rate-limit")]

use rullst_security::{RedisRateLimitMode, RedisRateLimiter};
use std::time::Duration;

#[tokio::test]
async fn independent_clients_share_one_atomic_redis_budget() {
    let Ok(redis_url) = std::env::var("RULLST_TEST_REDIS_URL") else {
        eprintln!("RULLST_TEST_REDIS_URL is unset; skipping the opt-in live Redis contract");
        return;
    };
    let prefix = format!("rullst:live:{}", rand::random::<u64>());
    let first = RedisRateLimiter::new(&redis_url, &prefix, 1, Duration::from_secs(30))
        .expect("first live Redis limiter");
    let second = RedisRateLimiter::new(&redis_url, &prefix, 1, Duration::from_secs(30))
        .expect("second live Redis limiter");
    assert_eq!(first.mode(), RedisRateLimitMode::Distributed);
    first
        .require_distributed()
        .expect("live Redis must satisfy the distributed startup boundary");

    let accepted = first
        .check("learner:7:127.0.0.1")
        .await
        .expect("first check");
    let rejected = second
        .check("learner:7:127.0.0.1")
        .await
        .expect("second check");
    assert!(accepted.allowed);
    assert_eq!(accepted.remaining, 0);
    assert!(!rejected.allowed);
    assert_eq!(rejected.remaining, 0);
    assert!(rejected.retry_after > Duration::ZERO);
    assert!(rejected.retry_after <= Duration::from_secs(30));
}
