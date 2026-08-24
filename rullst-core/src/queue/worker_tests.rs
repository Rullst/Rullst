#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct SharedDriverState {
    jobs: Mutex<VecDeque<QueuedJob>>,
    completed: AtomicUsize,
    failed: AtomicUsize,
    requeued: AtomicUsize,
    fail_complete: AtomicBool,
}

impl SharedDriverState {
    fn with_jobs(count: usize) -> Arc<Self> {
        let jobs = (0..count)
            .map(|index| QueuedJob {
                id: format!("job-{index}"),
                name: "test".to_string(),
                payload: serde_json::json!({ "index": index }),
                attempts: 1,
            })
            .collect();
        Arc::new(Self {
            jobs: Mutex::new(jobs),
            completed: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            requeued: AtomicUsize::new(0),
            fail_complete: AtomicBool::new(false),
        })
    }
}

struct TestDriver(Arc<SharedDriverState>);

#[async_trait]
impl QueueDriver for TestDriver {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        Ok(self.0.jobs.lock().unwrap().pop_front())
    }

    async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
        if self.0.fail_complete.load(Ordering::SeqCst) {
            return Err(QueueError::Driver("completion write failed".to_string()));
        }
        self.0.completed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
        self.0.failed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn requeue(&self, _job_id: &str, _reason: &str) -> Result<(), QueueError> {
        self.0.requeued.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn recover_stalled(&self, _stale_after: Duration) -> Result<u64, QueueError> {
        Ok(0)
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(self.0.jobs.lock().unwrap().len() as u64)
    }
}

fn test_queue(state: &Arc<SharedDriverState>) -> Queue {
    Queue::custom(Box::new(TestDriver(Arc::clone(state))))
}

#[test]
fn worker_start_outside_runtime_is_fallible() {
    let state = SharedDriverState::with_jobs(0);
    let queue = test_queue(&state);
    let worker = Worker::new(&queue);

    assert!(matches!(worker.run(), Err(QueueError::RuntimeUnavailable)));
}

#[tokio::test]
async fn worker_rejects_zero_concurrency_without_spawning() {
    let state = SharedDriverState::with_jobs(0);
    let queue = test_queue(&state);
    let worker = Worker::new(&queue).max_concurrency(0);

    assert!(matches!(
        worker.run(),
        Err(QueueError::InvalidConfiguration(_))
    ));
}

#[tokio::test]
async fn worker_never_exceeds_concurrency_limit() {
    let state = SharedDriverState::with_jobs(6);
    let queue = test_queue(&state);
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let mut worker = Worker::new(&queue).max_concurrency(2).poll_interval(2);
    let active_for_handler = Arc::clone(&active);
    let maximum_for_handler = Arc::clone(&maximum);
    worker.register("test", move |_| {
        let active = Arc::clone(&active_for_handler);
        let maximum = Arc::clone(&maximum_for_handler);
        async move {
            let now = active.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            active.fetch_sub(1, Ordering::SeqCst);
            Ok(())
        }
    });
    let handle = worker.run().unwrap();

    tokio::time::timeout(Duration::from_secs(1), async {
        while state.completed.load(Ordering::SeqCst) < 6 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    handle.shutdown().await.unwrap();

    assert!(maximum.load(Ordering::SeqCst) <= 2);
}

#[tokio::test]
async fn completion_transition_errors_are_observable() {
    let state = SharedDriverState::with_jobs(1);
    state.fail_complete.store(true, Ordering::SeqCst);
    let queue = test_queue(&state);
    let mut worker = Worker::new(&queue).poll_interval(2);
    worker.register("test", |_| async { Ok(()) });
    let mut handle = worker.run().unwrap();

    let error = tokio::time::timeout(Duration::from_secs(1), handle.next_error())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        error,
        QueueError::StateTransition {
            operation: "mark_complete",
            ..
        }
    ));
    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn graceful_shutdown_requeues_an_interrupted_job() {
    let state = SharedDriverState::with_jobs(1);
    let queue = test_queue(&state);
    let started = Arc::new(AtomicBool::new(false));
    let started_for_handler = Arc::clone(&started);
    let mut worker = Worker::new(&queue).poll_interval(2);
    worker.register("test", move |_| {
        let started = Arc::clone(&started_for_handler);
        async move {
            started.store(true, Ordering::SeqCst);
            std::future::pending::<()>().await;
            Ok(())
        }
    });
    let handle = worker.run().unwrap();

    tokio::time::timeout(Duration::from_secs(1), async {
        while !started.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    handle.shutdown().await.unwrap();

    assert_eq!(state.requeued.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn handler_panics_are_contained_and_failed() {
    let state = SharedDriverState::with_jobs(1);
    let queue = test_queue(&state);
    let mut worker = Worker::new(&queue).poll_interval(2);
    worker.register("test", |_| async { panic!("test panic") });
    let mut handle = worker.run().unwrap();

    let error = tokio::time::timeout(Duration::from_secs(1), handle.next_error())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(error, QueueError::JobPanicked { .. }));
    assert_eq!(state.failed.load(Ordering::SeqCst), 1);
    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn handler_timeouts_are_contained_and_failed() {
    let state = SharedDriverState::with_jobs(1);
    let queue = test_queue(&state);
    let mut worker = Worker::new(&queue)
        .poll_interval(2)
        .job_timeout(Duration::from_millis(5));
    worker.register("test", |_| async {
        std::future::pending::<()>().await;
        Ok(())
    });
    let mut handle = worker.run().unwrap();

    let error = tokio::time::timeout(Duration::from_secs(1), handle.next_error())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        error,
        QueueError::JobTimedOut { timeout_ms: 5, .. }
    ));
    assert_eq!(state.failed.load(Ordering::SeqCst), 1);
    handle.shutdown().await.unwrap();
}
