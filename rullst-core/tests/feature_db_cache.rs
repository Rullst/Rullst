//! `DbFeatureDriver` caching against a real SQLite pool.
//!
//! `--all-features` selects the strict PostgreSQL pool type, so this SQLite
//! contract runs with `--features orm`; the cache policy itself is also
//! covered by the driver's unit tests in every feature set.
#![cfg(all(
    feature = "orm",
    not(feature = "strict-postgres"),
    not(feature = "strict-mysql")
))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::feature::{DbFeatureDriver, FeatureDriver};
use std::time::Duration;

const TTL: Duration = Duration::from_millis(400);

async fn execute(sql: &'static str) {
    let pool = rullst_core::db::safe_pool().expect("initialized pool");
    sqlx::query(sql).execute(pool).await.expect(sql);
}

async fn past_ttl() {
    tokio::time::sleep(TTL + Duration::from_millis(150)).await;
}

/// One test owns the process-wide ORM pool, so the scenarios run in order.
#[tokio::test]
async fn misses_and_failures_are_cached_for_the_ttl_and_stale_values_survive_errors() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("flags.sqlite");
    rullst_orm::Orm::init_with_options(&format!("sqlite:{}?mode=rwc", database.display()), 4, 5)
        .await
        .expect("SQLite ORM initialization");
    let driver = DbFeatureDriver::with_ttl(TTL);

    // A missing table is a failed lookup; it is cached instead of re-queried.
    assert_eq!(driver.enabled("beta-banner").await, None);
    execute(
        "CREATE TABLE rullst_feature_flags (name TEXT PRIMARY KEY, enabled INTEGER NOT NULL, \
         rollout_percentage INTEGER, variants TEXT)",
    )
    .await;
    execute("INSERT INTO rullst_feature_flags (name, enabled) VALUES ('beta-banner', 1)").await;
    assert_eq!(
        driver.enabled("beta-banner").await,
        None,
        "the failed lookup must be cached for the TTL"
    );
    past_ttl().await;
    assert_eq!(driver.enabled("beta-banner").await, Some(true));

    // A flag without a row is cached as absent for the TTL.
    assert_eq!(driver.enabled("new-checkout").await, None);
    execute("INSERT INTO rullst_feature_flags (name, enabled) VALUES ('new-checkout', 1)").await;
    assert_eq!(
        driver.enabled("new-checkout").await,
        None,
        "the miss must be cached for the TTL"
    );
    past_ttl().await;
    assert_eq!(driver.enabled("new-checkout").await, Some(true));

    // A failure after expiry keeps serving the last value read.
    execute("DROP TABLE rullst_feature_flags").await;
    past_ttl().await;
    assert_eq!(
        driver.enabled("new-checkout").await,
        Some(true),
        "a failed refresh must serve the last known value"
    );
    assert_eq!(driver.enabled("never-defined").await, None);

    // An explicit invalidation still forces a new lookup.
    DbFeatureDriver::invalidate_process_cache();
    execute(
        "CREATE TABLE rullst_feature_flags (name TEXT PRIMARY KEY, enabled INTEGER NOT NULL, \
         rollout_percentage INTEGER, variants TEXT)",
    )
    .await;
    execute("INSERT INTO rullst_feature_flags (name, enabled) VALUES ('new-checkout', 0)").await;
    assert_eq!(driver.enabled("new-checkout").await, Some(false));
}
