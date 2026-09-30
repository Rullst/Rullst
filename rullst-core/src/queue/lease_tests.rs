//! Claim fencing and claim hand-back contracts for the worker and drivers.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::sync::Mutex;

/// Driver that hands out one claim and records which transition API the
/// worker used for it.
#[derive(Default)]
struct RecordingState {
    job: Mutex<Option<QueuedJob>>,
    fenced: Mutex<Vec<(&'static str, String, u32)>>,
    unfenced: Mutex<Vec<&'static str>>,
}

struct RecordingDriver(Arc<RecordingState>);

#[async_trait]
impl QueueDriver for RecordingDriver {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        Ok(self.0.job.lock().unwrap().take())
    }

    async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
        self.0.unfenced.lock().unwrap().push("mark_complete");
        Ok(())
    }

    async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
        self.0.unfenced.lock().unwrap().push("mark_failed");
        Ok(())
    }

    async fn mark_complete_attempt(&self, job_id: &str, attempt: u32) -> Result<(), QueueError> {
        let entry = ("mark_complete_attempt", job_id.to_string(), attempt);
        self.0.fenced.lock().unwrap().push(entry);
        Ok(())
    }

    async fn mark_failed_attempt(
        &self,
        job_id: &str,
        attempt: u32,
        _error: &str,
    ) -> Result<(), QueueError> {
        let entry = ("mark_failed_attempt", job_id.to_string(), attempt);
        self.0.fenced.lock().unwrap().push(entry);
        Ok(())
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(0)
    }
}

fn recording_queue(name: &str, attempts: u32) -> (Queue, Arc<RecordingState>) {
    let state = Arc::new(RecordingState::default());
    *state.job.lock().unwrap() = Some(QueuedJob {
        id: "job".to_string(),
        name: name.to_string(),
        payload: serde_json::json!({}),
        attempts,
    });
    (
        Queue::custom(Box::new(RecordingDriver(Arc::clone(&state)))),
        state,
    )
}

async fn wait_for_transition(state: &RecordingState) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while state.fenced.lock().unwrap().is_empty() && state.unfenced.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn worker_fences_transitions_on_the_claimed_attempt() {
    let (queue, state) = recording_queue("test", 7);
    let mut worker = Worker::new(&queue).poll_interval(2);
    worker.register("test", |_| async { Ok(()) });
    let handle = worker.run().unwrap();
    wait_for_transition(&state).await;
    handle.shutdown().await.unwrap();

    assert_eq!(
        *state.fenced.lock().unwrap(),
        vec![("mark_complete_attempt", "job".to_string(), 7)]
    );
    assert!(state.unfenced.lock().unwrap().is_empty());

    let (queue, state) = recording_queue("test", 3);
    let mut worker = Worker::new(&queue).poll_interval(2);
    worker.register("test", |_| async { Err("handler failed".into()) });
    let mut handle = worker.run().unwrap();
    wait_for_transition(&state).await;
    assert!(matches!(
        handle.next_error().await,
        Some(QueueError::JobFailed(_))
    ));
    handle.shutdown().await.unwrap();

    assert_eq!(
        *state.fenced.lock().unwrap(),
        vec![("mark_failed_attempt", "job".to_string(), 3)]
    );
}

#[cfg(all(feature = "queue-sqlite", not(miri)))]
mod sqlite {
    use super::*;

    /// Removes the temporary queue directory when the test ends.
    struct TempDir(std::path::PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    async fn file_queue() -> (TempDir, String) {
        let directory =
            TempDir(std::env::temp_dir().join(format!("rullst-lease-{}", uuid::Uuid::new_v4())));
        std::fs::create_dir_all(&directory.0).unwrap();
        let url = format!(
            "sqlite://{}?mode=rwc",
            directory.0.join("jobs.sqlite").display()
        );
        (directory, url)
    }

    /// Stalled-lease recovery hands a job to another worker while the first
    /// one is paused. The first worker's late success must not finish or
    /// delete the newer claim.
    #[tokio::test]
    async fn a_stale_worker_cannot_complete_a_reclaimed_job() {
        let (_directory, url) = file_queue().await;
        let other_worker = SqliteDriver::new(url.clone()).await.unwrap();
        let queue = Queue::sqlite(url).await.unwrap();
        let id = queue.dispatch("slow", serde_json::json!({})).await.unwrap();

        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let started_for_handler = Arc::clone(&started);
        let release_for_handler = Arc::clone(&release);
        let mut worker = Worker::new(&queue).poll_interval(5).max_concurrency(1);
        worker.register("slow", move |_| {
            let started = Arc::clone(&started_for_handler);
            let release = Arc::clone(&release_for_handler);
            async move {
                started.notify_one();
                release.notified().await;
                Ok(())
            }
        });
        let mut handle = worker.run().unwrap();
        tokio::time::timeout(Duration::from_secs(5), started.notified())
            .await
            .unwrap();

        // Recovery returns the lease to pending and another worker claims it.
        sqlx::query("UPDATE rullst_jobs SET status = 'pending' WHERE id = ?")
            .bind(&id)
            .execute(&other_worker.pool)
            .await
            .unwrap();
        let reclaimed = other_worker.pop().await.unwrap().unwrap();
        assert_eq!(reclaimed.attempts, 2);

        release.notify_one();
        let error = tokio::time::timeout(Duration::from_secs(5), handle.next_error())
            .await
            .expect("the stale completion must be rejected and reported")
            .unwrap();
        assert!(matches!(
            error,
            QueueError::StateTransition {
                operation: "mark_complete",
                ..
            }
        ));

        let (status, attempts): (String, i64) =
            sqlx::query_as("SELECT status, attempts FROM rullst_jobs WHERE id = ?")
                .bind(&id)
                .fetch_one(&other_worker.pool)
                .await
                .unwrap();
        assert_eq!((status.as_str(), attempts), ("processing", 2));
        other_worker
            .mark_failed(&reclaimed.id, "second claim failed")
            .await
            .unwrap();
        handle.shutdown().await.unwrap();
    }

    async fn claimed_twice(driver: &SqliteDriver, id: &str) -> (u32, u32) {
        driver.push(id, "job", "{}").await.unwrap();
        let first = driver.pop().await.unwrap().unwrap();
        driver.requeue(&first.id, "recovered").await.unwrap();
        let second = driver.pop().await.unwrap().unwrap();
        assert_eq!(second.id, id);
        (first.attempts, second.attempts)
    }

    async fn status(driver: &SqliteDriver, id: &str) -> Option<String> {
        sqlx::query_scalar("SELECT status FROM rullst_jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(&driver.pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn fenced_transitions_apply_only_to_the_current_claim_attempt() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        let (stale, current) = claimed_twice(&driver, "complete").await;
        assert!(matches!(
            driver.mark_complete_attempt("complete", stale).await,
            Err(QueueError::StateTransition { message, .. }) if message.contains("claim attempt")
        ));
        driver
            .mark_complete_attempt("complete", current)
            .await
            .unwrap();
        assert_eq!(status(&driver, "complete").await, None);

        let (stale, current) = claimed_twice(&driver, "fail").await;
        assert!(
            driver
                .mark_failed_attempt("fail", stale, "stale")
                .await
                .is_err()
        );
        assert_eq!(status(&driver, "fail").await.as_deref(), Some("processing"));
        driver
            .mark_failed_attempt("fail", current, "current")
            .await
            .unwrap();
        assert_eq!(status(&driver, "fail").await.as_deref(), Some("failed"));

        let (stale, current) = claimed_twice(&driver, "requeue").await;
        assert!(
            driver
                .requeue_attempt("requeue", stale, "stale")
                .await
                .is_err()
        );
        driver
            .requeue_attempt("requeue", current, "current")
            .await
            .unwrap();
        assert_eq!(status(&driver, "requeue").await.as_deref(), Some("pending"));
    }

    #[tokio::test]
    async fn fenced_completion_keeps_retained_history_atomic() {
        let driver = SqliteDriver::new("sqlite::memory:")
            .await
            .unwrap()
            .try_with_completed_history_limit(5)
            .unwrap();
        let (stale, current) = claimed_twice(&driver, "retained").await;
        assert!(
            driver
                .mark_complete_attempt("retained", stale)
                .await
                .is_err()
        );
        assert_eq!(
            status(&driver, "retained").await.as_deref(),
            Some("processing")
        );
        driver
            .mark_complete_attempt("retained", current)
            .await
            .unwrap();
        assert_eq!(
            status(&driver, "retained").await.as_deref(),
            Some("completed")
        );
    }
}
