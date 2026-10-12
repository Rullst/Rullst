//! Default `QueueDriver` methods of a driver that implements only the
//! required ones, and SQLite purges reached through the trait.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use async_trait::async_trait;

/// Implements only the required `QueueDriver` methods.
struct MinimalDriver;

#[async_trait]
impl QueueDriver for MinimalDriver {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        Ok(None)
    }

    async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(0)
    }
}

#[tokio::test]
async fn optional_transitions_fail_closed_on_a_minimal_driver() {
    let driver = MinimalDriver;
    assert!(matches!(
        driver.requeue("job", "interrupted").await,
        Err(QueueError::Unsupported(_))
    ));
    assert!(matches!(
        driver.requeue_attempt("job", 1, "interrupted").await,
        Err(QueueError::Unsupported(_))
    ));
    assert!(matches!(
        driver
            .requeue_attempt_after("job", 1, "no handler", Duration::from_secs(5))
            .await,
        Err(QueueError::Unsupported(_))
    ));
    assert!(matches!(
        driver.recover_stalled(Duration::from_secs(60)).await,
        Err(QueueError::Unsupported(_))
    ));
    assert!(matches!(
        driver.purge_completed_history().await,
        Err(QueueError::Unsupported(_))
    ));
    assert!(matches!(
        Queue::custom(Box::new(MinimalDriver))
            .purge_completed_history()
            .await,
        Err(QueueError::Unsupported(_))
    ));
}

#[cfg(feature = "queue-sqlite")]
#[tokio::test]
async fn sqlite_purges_reached_through_the_driver_trait_remove_rows() {
    let queue = Queue::sqlite_with_completed_history("sqlite::memory:", 10)
        .await
        .unwrap();
    let driver = queue.driver_ref();
    driver.push("done", "report", "{}").await.unwrap();
    let job = driver.pop().await.unwrap().unwrap();
    driver.mark_complete(&job.id).await.unwrap();
    assert_eq!(queue.list_all_jobs(10).await.unwrap().len(), 1);
    queue.purge_completed_history().await.unwrap();
    assert!(queue.list_all_jobs(10).await.unwrap().is_empty());

    // The legacy, misnamed hook purges failed jobs.
    let sqlite = SqliteDriver::new("sqlite::memory:").await.unwrap();
    sqlite.push("broken", "report", "{}").await.unwrap();
    let job = sqlite.pop().await.unwrap().unwrap();
    sqlite.mark_failed(&job.id, "boom").await.unwrap();
    assert_eq!(sqlite.list_all_jobs(10).await.unwrap().len(), 1);
    QueueDriver::purge_completed_jobs(&sqlite).await.unwrap();
    assert!(sqlite.list_all_jobs(10).await.unwrap().is_empty());
}
