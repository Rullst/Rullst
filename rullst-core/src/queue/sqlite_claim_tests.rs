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
