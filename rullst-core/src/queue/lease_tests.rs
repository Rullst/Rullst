//! Claim fencing and claim hand-back contracts for the worker and drivers.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Driver that hands out one claim and records which transition API the
/// worker used for it.
#[derive(Default)]
struct RecordingState {
    job: Mutex<Option<QueuedJob>>,
    fenced: Mutex<Vec<(&'static str, String, u32)>>,
    unfenced: Mutex<Vec<&'static str>>,
    deferral_unsupported: AtomicBool,
    deferral: Mutex<Option<(String, Duration)>>,
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

    async fn requeue_attempt_after(
        &self,
        job_id: &str,
        attempt: u32,
        reason: &str,
        delay: Duration,
    ) -> Result<(), QueueError> {
        if self.0.deferral_unsupported.load(Ordering::SeqCst) {
            return Err(QueueError::Unsupported("no deferral".to_string()));
        }
        let entry = ("requeue_attempt_after", job_id.to_string(), attempt);
        self.0.fenced.lock().unwrap().push(entry);
        *self.0.deferral.lock().unwrap() = Some((reason.to_string(), delay));
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

#[tokio::test]
async fn a_worker_without_the_handler_hands_the_claim_back_with_a_delay() {
    let (queue, state) = recording_queue("export_csv", 4);
    let mut worker = Worker::new(&queue).poll_interval(2);
    worker.register("send_mail", |_| async { Ok(()) });
    let mut handle = worker.run().unwrap();
    wait_for_transition(&state).await;
    assert!(matches!(
        handle.next_error().await,
        Some(QueueError::HandlerNotFound(name)) if name == "export_csv"
    ));
    handle.shutdown().await.unwrap();

    assert_eq!(
        *state.fenced.lock().unwrap(),
        vec![("requeue_attempt_after", "job".to_string(), 4)]
    );
    let (reason, delay) = state.deferral.lock().unwrap().clone().unwrap();
    assert!(reason.contains("export_csv"));
    assert_eq!(delay, worker::UNHANDLED_JOB_RETRY_DELAY);
    assert!(!delay.is_zero());
    assert!(state.unfenced.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_driver_without_deferral_still_fails_an_unhandled_claim() {
    let (queue, state) = recording_queue("export_csv", 2);
    state.deferral_unsupported.store(true, Ordering::SeqCst);
    let mut handle = Worker::new(&queue).poll_interval(2).run().unwrap();
    wait_for_transition(&state).await;
    assert!(matches!(
        handle.next_error().await,
        Some(QueueError::HandlerNotFound(_))
    ));
    handle.shutdown().await.unwrap();

    assert_eq!(
        *state.fenced.lock().unwrap(),
        vec![("mark_failed_attempt", "job".to_string(), 2)]
    );
}

#[tokio::test]
async fn a_job_no_worker_can_handle_is_failed_after_the_hand_back_ceiling() {
    let below = worker::MAX_UNHANDLED_CLAIM_ATTEMPT - 1;
    let (queue, state) = recording_queue("retired_export", below);
    let handle = Worker::new(&queue).poll_interval(2).run().unwrap();
    wait_for_transition(&state).await;
    drop(handle);
    assert_eq!(
        *state.fenced.lock().unwrap(),
        vec![("requeue_attempt_after", "job".to_string(), below)]
    );

    let ceiling = worker::MAX_UNHANDLED_CLAIM_ATTEMPT;
    let (queue, state) = recording_queue("retired_export", ceiling);
    let mut handle = Worker::new(&queue).poll_interval(2).run().unwrap();
    wait_for_transition(&state).await;
    assert!(matches!(
        handle.next_error().await,
        Some(QueueError::HandlerNotFound(name)) if name == "retired_export"
    ));
    handle.shutdown().await.unwrap();
    assert_eq!(
        *state.fenced.lock().unwrap(),
        vec![("mark_failed_attempt", "job".to_string(), ceiling)]
    );
    assert!(state.deferral.lock().unwrap().is_none());
}

#[tokio::test]
async fn dispatch_rejects_job_names_no_worker_could_handle() {
    let (queue, _state) = recording_queue("unused", 1);
    let far = std::time::SystemTime::now() + Duration::from_secs(60);
    for name in [String::new(), "x".repeat(257)] {
        assert!(matches!(
            queue.dispatch(&name, serde_json::json!({})).await,
            Err(QueueError::InvalidConfiguration(_))
        ));
        assert!(matches!(
            queue.dispatch_at(&name, serde_json::json!({}), far).await,
            Err(QueueError::InvalidConfiguration(_))
        ));
    }
    assert!(
        queue
            .dispatch(&"x".repeat(256), serde_json::json!({}))
            .await
            .is_ok()
    );
}

#[cfg(all(feature = "queue-sqlite", not(miri)))]
mod sqlite {
    use super::*;

    async fn file_queue() -> (tempfile::TempDir, String) {
        let directory = tempfile::tempdir().unwrap();
        let url = format!(
            "sqlite://{}?mode=rwc",
            directory.path().join("jobs.sqlite").display()
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

    /// During a rolling deploy an old worker claims a job only a new worker
    /// can run. It must leave the job for that worker instead of failing it,
    /// and must not re-claim it in a hot loop.
    #[tokio::test]
    async fn an_unhandled_job_is_left_for_a_worker_that_can_run_it() {
        let (_directory, url) = file_queue().await;
        let inspector = SqliteDriver::new(url.clone()).await.unwrap();
        let queue = Queue::sqlite(url.clone()).await.unwrap();
        let id = queue
            .dispatch("export_csv", serde_json::json!({"report": 1}))
            .await
            .unwrap();

        let mut old_worker = Worker::new(&queue).poll_interval(2);
        old_worker.register("send_mail", |_| async { Ok(()) });
        let mut old_handle = old_worker.run().unwrap();
        let error = tokio::time::timeout(Duration::from_secs(5), old_handle.next_error())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(error, QueueError::HandlerNotFound(name) if name == "export_csv"));
        tokio::time::sleep(Duration::from_millis(50)).await;
        old_handle.shutdown().await.unwrap();

        let (job_status, attempts, available_at_ms): (String, i64, i64) = sqlx::query_as(
            "SELECT status, attempts, available_at_ms FROM rullst_jobs WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&inspector.pool)
        .await
        .unwrap();
        assert_eq!(job_status, "pending");
        assert_eq!(attempts, 1, "the old worker must not re-claim it in a loop");
        let now_ms =
            i64::try_from(unix_timestamp_millis_floor(SystemTime::now()).unwrap()).unwrap();
        assert!(available_at_ms > now_ms);

        // Once due, a worker that registered the handler runs it.
        sqlx::query("UPDATE rullst_jobs SET available_at_ms = 0 WHERE id = ?")
            .bind(&id)
            .execute(&inspector.pool)
            .await
            .unwrap();
        let ran = Arc::new(AtomicBool::new(false));
        let ran_for_handler = Arc::clone(&ran);
        let mut new_worker = Worker::new(&queue).poll_interval(2);
        new_worker.register("export_csv", move |_| {
            let ran = Arc::clone(&ran_for_handler);
            async move {
                ran.store(true, Ordering::SeqCst);
                Ok(())
            }
        });
        let new_handle = new_worker.run().unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while status(&inspector, &id).await.is_some() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        new_handle.shutdown().await.unwrap();
        assert!(ran.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn a_deferred_claim_is_fenced_and_not_claimable_before_its_delay() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        let (stale, current) = claimed_twice(&driver, "deferred").await;
        assert!(
            driver
                .requeue_attempt_after("deferred", stale, "stale", Duration::from_millis(60))
                .await
                .is_err()
        );
        driver
            .requeue_attempt_after("deferred", current, "no handler", Duration::from_millis(60))
            .await
            .unwrap();
        assert!(driver.pop().await.unwrap().is_none());
        assert_eq!(driver.pending_count().await.unwrap(), 1);

        tokio::time::sleep(Duration::from_millis(80)).await;
        let reclaimed = driver.pop().await.unwrap().unwrap();
        assert_eq!(reclaimed.id, "deferred");
        assert_eq!(reclaimed.attempts, current + 1);
    }
}
