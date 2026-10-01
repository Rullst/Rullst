//! SQLite claim ordering, lease timing and retry fencing contracts.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{QueueDriver, SqliteDriver};
use std::time::{Duration, SystemTime};

#[tokio::test]
async fn a_due_scheduled_job_is_claimed_before_later_immediate_jobs() {
    let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
    let due = SystemTime::now() + Duration::from_millis(20);
    driver
        .push_at("expire-trial", "expire_trial", "{}", due)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(60)).await;
    // Pushed after the scheduled job became due, so it must not overtake it.
    driver.push("email", "send_email", "{}").await.unwrap();

    assert_eq!(driver.pop().await.unwrap().unwrap().id, "expire-trial");
    assert_eq!(driver.pop().await.unwrap().unwrap().id, "email");
}

#[tokio::test]
async fn a_handed_back_job_is_claimed_before_later_immediate_jobs() {
    let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
    driver.push("export", "export_csv", "{}").await.unwrap();
    let claim = driver.pop().await.unwrap().unwrap();
    driver
        .requeue_attempt_after(
            &claim.id,
            claim.attempts,
            "no handler",
            Duration::from_millis(20),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(60)).await;
    driver.push("email", "send_email", "{}").await.unwrap();

    assert_eq!(driver.pop().await.unwrap().unwrap().id, "export");
    assert_eq!(driver.pop().await.unwrap().unwrap().id, "email");
}

#[tokio::test]
async fn a_lease_free_claim_is_never_recovered_before_its_age() {
    let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
    driver
        .push("report", "generate_report", "{}")
        .await
        .unwrap();
    driver.pop().await.unwrap().unwrap();
    // `updated_at` has whole-second precision: a claim recorded five seconds
    // ago may have been made only just over four seconds ago.
    sqlx::query("UPDATE rullst_jobs SET updated_at = datetime('now', '-5 seconds')")
        .execute(&driver.pool)
        .await
        .unwrap();
    assert_eq!(
        driver
            .recover_stalled(Duration::from_millis(5_900))
            .await
            .unwrap(),
        0
    );

    sqlx::query("UPDATE rullst_jobs SET updated_at = datetime('now', '-7 seconds')")
        .execute(&driver.pool)
        .await
        .unwrap();
    assert_eq!(
        driver
            .recover_stalled(Duration::from_millis(5_900))
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn a_stale_lease_cannot_finish_the_claim_made_after_a_manual_retry() {
    let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
    driver.push("sync", "sync_account", "{}").await.unwrap();
    // A paused worker's claim is recovered and claimed again, and that claim
    // fails; an operator then retries the job and a third worker claims it.
    let stale = driver.pop().await.unwrap().unwrap();
    driver
        .requeue_attempt(&stale.id, stale.attempts, "recovered")
        .await
        .unwrap();
    let second = driver.pop().await.unwrap().unwrap();
    driver
        .mark_failed_attempt(&second.id, second.attempts, "provider down")
        .await
        .unwrap();
    driver.retry_failed_job("sync").await.unwrap();
    let current = driver.pop().await.unwrap().unwrap();
    assert_eq!(current.attempts, 3);

    assert!(
        driver
            .mark_complete_attempt(&stale.id, stale.attempts)
            .await
            .is_err(),
        "the paused worker must not finish the claim made after the retry"
    );
    driver
        .mark_complete_attempt(&current.id, current.attempts)
        .await
        .unwrap();
}
