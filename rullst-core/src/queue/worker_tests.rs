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
async fn worker_rejects_zero_concurrency_or_job_timeout_without_spawning() {
    let state = SharedDriverState::with_jobs(0);
    let queue = test_queue(&state);
    for worker in [
        Worker::new(&queue).max_concurrency(0),
        // A zero deadline would fail every job as timed out before it runs.
        Worker::new(&queue).job_timeout(Duration::ZERO),
    ] {
        assert!(matches!(
            worker.run(),
            Err(QueueError::InvalidConfiguration(_))
        ));
    }
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

/// Driver whose every claim fails, like an unreachable Redis server.
struct UnavailableDriver(Arc<AtomicUsize>);

#[async_trait]
impl QueueDriver for UnavailableDriver {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(QueueError::Driver("backend unavailable".to_string()))
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
async fn undrained_worker_errors_are_bounded_and_counted() {
    let polls = Arc::new(AtomicUsize::new(0));
    let queue = Queue::custom(Box::new(UnavailableDriver(Arc::clone(&polls))));
    let mut handle = Worker::new(&queue).poll_interval(1).run().unwrap();
    let capacity = crate::error_buffer::ERROR_BUFFER_CAPACITY;

    tokio::time::timeout(Duration::from_secs(10), async {
        while polls.load(Ordering::SeqCst) <= capacity + 16 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();

    let mut buffered = 0;
    while handle.try_next_error().is_some() {
        buffered += 1;
    }
    assert_eq!(buffered, capacity);
    assert!(handle.dropped_errors() >= 16);
    assert!(matches!(
        handle.shutdown().await,
        Ok(()) | Err(QueueError::Driver(_))
    ));
}

/// A handler that blocks its thread cannot be interrupted by `abort`, so it
/// still returns `Ok` after the deadline branch was selected. Its side effects
/// happened; the worker must record a completion, not a timeout.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_handler_that_succeeds_after_its_deadline_is_completed_not_timed_out() {
    let state = SharedDriverState::with_jobs(1);
    let queue = test_queue(&state);
    let mut worker = Worker::new(&queue)
        .poll_interval(2)
        .job_timeout(Duration::from_millis(20));
    worker.register("test", |_| async {
        std::thread::sleep(Duration::from_millis(150));
        Ok(())
    });
    let handle = worker.run().unwrap();

    tokio::time::timeout(Duration::from_secs(2), async {
        while state.completed.load(Ordering::SeqCst) + state.failed.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    handle.shutdown().await.unwrap();

    assert_eq!(state.completed.load(Ordering::SeqCst), 1);
    assert_eq!(state.failed.load(Ordering::SeqCst), 0);
}

/// Shutdown selected while a blocking handler was finishing successfully must
/// complete the job instead of requeuing it for a second execution.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_handler_that_succeeds_during_shutdown_is_completed_not_requeued() {
    let state = SharedDriverState::with_jobs(1);
    let queue = test_queue(&state);
    let started = Arc::new(AtomicBool::new(false));
    let started_for_handler = Arc::clone(&started);
    let mut worker = Worker::new(&queue).poll_interval(2);
    worker.register("test", move |_| {
        let started = Arc::clone(&started_for_handler);
        async move {
            started.store(true, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(150));
            Ok(())
        }
    });
    let handle = worker.run().unwrap();

    tokio::time::timeout(Duration::from_secs(1), async {
        while !started.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    handle.shutdown().await.unwrap();

    assert_eq!(state.completed.load(Ordering::SeqCst), 1);
    assert_eq!(state.requeued.load(Ordering::SeqCst), 0);
}

/// Driver whose claim of the `gated` job commits immediately (like the SQLite
/// `UPDATE ... RETURNING` and Redis `EVAL` claims) but resolves only after the
/// test releases it, so a worker that drops the pop future strands the job.
struct GatedClaimState {
    jobs: Mutex<VecDeque<QueuedJob>>,
    gated_claim_started: tokio::sync::Notify,
    release_gated_claim: tokio::sync::Notify,
    completed: Mutex<Vec<String>>,
    requeued: Mutex<Vec<String>>,
}

impl GatedClaimState {
    fn with_jobs(ids: &[&str]) -> Arc<Self> {
        let jobs = ids
            .iter()
            .map(|id| QueuedJob {
                id: (*id).to_string(),
                name: "test".to_string(),
                payload: serde_json::json!({ "id": id }),
                attempts: 1,
            })
            .collect();
        Arc::new(Self {
            jobs: Mutex::new(jobs),
            gated_claim_started: tokio::sync::Notify::new(),
            release_gated_claim: tokio::sync::Notify::new(),
            completed: Mutex::new(Vec::new()),
            requeued: Mutex::new(Vec::new()),
        })
    }
}

struct GatedClaimDriver(Arc<GatedClaimState>);

#[async_trait]
impl QueueDriver for GatedClaimDriver {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        let claimed = self.0.jobs.lock().unwrap().pop_front();
        if claimed.as_ref().is_some_and(|job| job.id == "gated") {
            self.0.gated_claim_started.notify_one();
            self.0.release_gated_claim.notified().await;
        }
        Ok(claimed)
    }

    async fn mark_complete(&self, job_id: &str) -> Result<(), QueueError> {
        self.0.completed.lock().unwrap().push(job_id.to_string());
        Ok(())
    }

    async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn requeue(&self, job_id: &str, _reason: &str) -> Result<(), QueueError> {
        self.0.requeued.lock().unwrap().push(job_id.to_string());
        Ok(())
    }

    async fn recover_stalled(&self, _stale_after: Duration) -> Result<u64, QueueError> {
        Ok(0)
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(self.0.jobs.lock().unwrap().len() as u64)
    }
}

#[tokio::test]
async fn a_claim_in_flight_is_dispatched_even_when_a_handler_finishes_first() {
    let state = GatedClaimState::with_jobs(&["first", "gated"]);
    let queue = Queue::custom(Box::new(GatedClaimDriver(Arc::clone(&state))));
    let release_first = Arc::new(tokio::sync::Notify::new());
    let release_for_handler = Arc::clone(&release_first);
    let mut worker = Worker::new(&queue).poll_interval(2);
    worker.register("test", move |payload| {
        let release_first = Arc::clone(&release_for_handler);
        async move {
            if payload["id"] == "first" {
                release_first.notified().await;
                return Err("first job failed".into());
            }
            Ok(())
        }
    });
    let mut handle = worker.run().unwrap();
    let wait = Duration::from_secs(1);

    // `first` is running and the claim of `gated` has committed but not resolved.
    tokio::time::timeout(wait, state.gated_claim_started.notified())
        .await
        .unwrap();
    // `first` finishes while the claim is in flight; the worker observes that
    // outcome before the claim resolves.
    release_first.notify_one();
    let error = tokio::time::timeout(wait, handle.next_error())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(error, QueueError::JobFailed(_)));
    state.release_gated_claim.notify_one();

    tokio::time::timeout(wait, async {
        while !state
            .completed
            .lock()
            .unwrap()
            .contains(&"gated".to_string())
        {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("the claimed job must be dispatched, not stranded in processing");
    handle.shutdown().await.unwrap();
    assert!(state.requeued.lock().unwrap().is_empty());
}

#[tokio::test]
async fn shutdown_during_a_claim_requeues_the_claimed_job() {
    let state = GatedClaimState::with_jobs(&["gated"]);
    let queue = Queue::custom(Box::new(GatedClaimDriver(Arc::clone(&state))));
    let mut worker = Worker::new(&queue).poll_interval(2);
    worker.register("test", |_| async {
        std::future::pending::<()>().await;
        Ok(())
    });
    let handle = worker.run().unwrap();

    tokio::time::timeout(Duration::from_secs(1), state.gated_claim_started.notified())
        .await
        .unwrap();
    let mut stopping = tokio::spawn(handle.shutdown());
    // A worker that races the claim against shutdown stops here and drops it.
    let early = tokio::time::timeout(Duration::from_millis(100), &mut stopping).await;
    state.release_gated_claim.notify_one();
    let stopped = match early {
        Ok(joined) => joined,
        Err(_) => tokio::time::timeout(Duration::from_secs(1), stopping)
            .await
            .unwrap(),
    };
    stopped.unwrap().unwrap();

    assert_eq!(*state.requeued.lock().unwrap(), vec!["gated".to_string()]);
    assert!(state.completed.lock().unwrap().is_empty());
}
