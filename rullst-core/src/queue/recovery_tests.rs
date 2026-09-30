//! SQLite stalled-lease recovery contracts.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{QueueDriver, SqliteDriver};
use std::time::Duration;

async fn row(driver: &SqliteDriver, id: &str) -> (String, i64, Option<String>) {
    sqlx::query_as("SELECT status, stalled_recoveries, error FROM rullst_jobs WHERE id = ?")
        .bind(id)
        .fetch_one(&driver.pool)
        .await
        .unwrap()
}

/// Claims the job and ages its lease as if the worker died an hour ago.
async fn claim_and_stall(driver: &SqliteDriver, id: &str) -> u32 {
    let claim = driver.pop().await.unwrap().expect("claimable job");
    assert_eq!(claim.id, id);
    sqlx::query("UPDATE rullst_jobs SET updated_at = datetime('now', '-1 hour') WHERE id = ?")
        .bind(id)
        .execute(&driver.pool)
        .await
        .unwrap();
    claim.attempts
}

#[tokio::test]
async fn a_job_that_keeps_stalling_its_worker_is_failed_instead_of_recovered_forever() {
    let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
    driver.push("poison", "resize_image", "{}").await.unwrap();

    for stall in 1..=4_i64 {
        claim_and_stall(&driver, "poison").await;
        assert_eq!(
            driver
                .recover_stalled(Duration::from_secs(1))
                .await
                .unwrap(),
            1
        );
        let (status, recoveries, _) = row(&driver, "poison").await;
        assert_eq!((status.as_str(), recoveries), ("pending", stall));
    }

    // The fifth stalled lease fails the job instead of requeuing it.
    assert_eq!(claim_and_stall(&driver, "poison").await, 5);
    assert_eq!(
        driver
            .recover_stalled(Duration::from_secs(1))
            .await
            .unwrap(),
        1
    );
    let (status, _, error) = row(&driver, "poison").await;
    assert_eq!(status, "failed");
    assert!(error.unwrap().contains("stalled 5 times"));
    assert!(driver.pop().await.unwrap().is_none());

    // A manual retry starts a fresh stalled-lease count.
    driver.retry_failed_job("poison").await.unwrap();
    claim_and_stall(&driver, "poison").await;
    assert_eq!(
        driver
            .recover_stalled(Duration::from_secs(1))
            .await
            .unwrap(),
        1
    );
    let (status, recoveries, _) = row(&driver, "poison").await;
    assert_eq!((status.as_str(), recoveries), ("pending", 1));
}

#[tokio::test]
async fn recovery_leaves_fresh_leases_and_other_states_alone() {
    let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
    driver.push("running", "job", "{}").await.unwrap();
    driver.pop().await.unwrap().unwrap();
    driver.push("waiting", "job", "{}").await.unwrap();

    assert_eq!(
        driver
            .recover_stalled(Duration::from_secs(60))
            .await
            .unwrap(),
        0
    );
    assert_eq!(row(&driver, "running").await.0, "processing");
    assert_eq!(row(&driver, "waiting").await.0, "pending");
}

#[tokio::test]
async fn an_existing_table_gains_the_stalled_lease_counter() {
    let directory = tempfile::tempdir().unwrap();
    let url = format!(
        "sqlite://{}?mode=rwc",
        directory.path().join("legacy.sqlite").display()
    );
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::query(
        "CREATE TABLE rullst_jobs (id TEXT PRIMARY KEY, name TEXT NOT NULL, \
         payload TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'pending', error TEXT, \
         attempts INTEGER NOT NULL DEFAULT 0, \
         created_at TEXT NOT NULL DEFAULT (datetime('now')), \
         updated_at TEXT NOT NULL DEFAULT (datetime('now')))",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO rullst_jobs (id, name, payload) VALUES ('legacy', 'job', '{}')")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let driver = SqliteDriver::new(url).await.unwrap();
    assert_eq!(row(&driver, "legacy").await.1, 0);
    assert_eq!(driver.pop().await.unwrap().unwrap().id, "legacy");
}

#[tokio::test]
async fn the_stalled_lease_ceiling_is_configurable_and_bounded() {
    let driver = SqliteDriver::new("sqlite::memory:")
        .await
        .unwrap()
        .try_with_max_stalled_leases(1)
        .unwrap();
    driver.push("fragile", "job", "{}").await.unwrap();
    claim_and_stall(&driver, "fragile").await;
    assert_eq!(
        driver
            .recover_stalled(Duration::from_secs(1))
            .await
            .unwrap(),
        1
    );
    assert_eq!(row(&driver, "fragile").await.0, "failed");

    for invalid in [0, super::MAX_STALLED_LEASES_LIMIT + 1] {
        let error = SqliteDriver::new("sqlite::memory:")
            .await
            .unwrap()
            .try_with_max_stalled_leases(invalid)
            .err()
            .expect("an out-of-range ceiling is rejected");
        assert!(matches!(error, super::QueueError::InvalidConfiguration(_)));
    }
}

#[tokio::test]
async fn a_claim_lease_outlives_a_shorter_recovery_age() {
    let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
    driver
        .push("report", "generate_report", "{}")
        .await
        .unwrap();
    driver
        .pop_with_lease(Duration::from_secs(3_600))
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE rullst_jobs SET updated_at = datetime('now', '-2 hours')")
        .execute(&driver.pool)
        .await
        .unwrap();

    assert_eq!(
        driver
            .recover_stalled(Duration::from_secs(1))
            .await
            .unwrap(),
        0
    );
    assert_eq!(row(&driver, "report").await.0, "processing");
}

#[tokio::test]
async fn an_expired_claim_lease_is_recovered_whatever_the_recovery_age() {
    let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
    driver.push("email", "send_email", "{}").await.unwrap();
    driver
        .pop_with_lease(Duration::from_millis(1))
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;

    assert_eq!(
        driver
            .recover_stalled(Duration::from_secs(3_600))
            .await
            .unwrap(),
        1
    );
    assert_eq!(row(&driver, "email").await.0, "pending");
    assert_eq!(driver.pop().await.unwrap().unwrap().attempts, 2);
}

/// Pool A runs 30-minute-class jobs; pool B has a short `stalled_after`.
/// B's periodic, queue-wide recovery must not requeue A's running job.
#[tokio::test]
async fn a_short_stalled_after_pool_does_not_duplicate_a_longer_running_job() {
    use crate::queue::{Queue, Worker};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let directory = tempfile::tempdir().unwrap();
    let url = format!(
        "sqlite://{}?mode=rwc",
        directory.path().join("pools.sqlite").display()
    );
    let reports = Queue::sqlite(url.clone()).await.unwrap();
    let emails = Queue::sqlite(url).await.unwrap();
    let id = reports
        .dispatch("generate_report", serde_json::json!({}))
        .await
        .unwrap();

    let runs = Arc::new(AtomicUsize::new(0));
    let runs_for_handler = Arc::clone(&runs);
    let mut pool_a = Worker::new(&reports)
        .poll_interval(10)
        .max_concurrency(2)
        .job_timeout(Duration::from_secs(30))
        .stalled_after(Duration::from_secs(60));
    pool_a.register("generate_report", move |_| {
        let runs = Arc::clone(&runs_for_handler);
        async move {
            runs.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(3_500)).await;
            Ok(())
        }
    });
    let pool_a = pool_a.run().unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while runs.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    let mut pool_b = Worker::new(&emails)
        .poll_interval(10)
        .job_timeout(Duration::from_millis(500))
        .stalled_after(Duration::from_secs(1));
    pool_b.register("send_email", |_| async { Ok(()) });
    let pool_b = pool_b.run().unwrap();

    // Pool B recovers at start and then every second while A's job runs.
    // Without claim leases it requeues A's job, which then either runs a
    // second time in A or is claimed (and handed back) by B.
    tokio::time::sleep(Duration::from_millis(4_500)).await;
    pool_b
        .shutdown()
        .await
        .expect("pool B must not recover and claim pool A's running job");
    pool_a.shutdown().await.unwrap();

    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "pool B requeued pool A's running job"
    );
    let remaining: Option<String> =
        sqlx::query_scalar("SELECT status FROM rullst_jobs WHERE id = ?")
            .bind(&id)
            .fetch_optional(
                &SqliteDriver::new(format!(
                    "sqlite://{}?mode=rwc",
                    directory.path().join("pools.sqlite").display()
                ))
                .await
                .unwrap()
                .pool,
            )
            .await
            .unwrap();
    assert_eq!(remaining, None, "the job completed exactly once");
}
