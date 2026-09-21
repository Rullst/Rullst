#![cfg(feature = "postgres")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "suppression_postgres/failures.rs"]
mod failures;
#[path = "suppression_postgres/lifecycle.rs"]
mod lifecycle;
#[path = "suppression_postgres/support.rs"]
mod support;
#[path = "suppression_postgres/worker.rs"]
mod worker;
use support::*;

#[test]
fn namespace_keys_and_limits_are_explicit() {
    assert!(SuppressionKey::new([0; 32]).is_err());
    assert!(SuppressionKey::new([1; 32]).is_err());
    for (scope, recipients, events) in [
        ("", 1, 1),
        ("a:b", 1, 1),
        ("scope", 0, 1),
        ("scope", 1, 0),
        ("scope", 1_000_001, 1),
    ] {
        assert!(PostgresSuppressionConfig::new(scope, recipients, events).is_err());
    }
    assert!(PostgresSuppressionConfig::new("a".repeat(129), 1, 1).is_err());
    assert!(format!("{:?}", key()).contains("REDACTED"));
}

#[tokio::test]
#[ignore = "requires wrapper-owned PostgreSQL"]
async fn postgres_suppression_contract() {
    let url = url();
    lifecycle::run(&url).await;
    failures::run(&url).await;
    worker::run(&url).await;
}
