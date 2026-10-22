//! The process-local activity counters the production monitor reads.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::test_support::shut_down;
use super::*;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Default)]
struct Driver {
    jobs: Mutex<VecDeque<QueuedJob>>,
    first_recovery: AtomicU64,
    defer_supported: AtomicBool,
    fail_complete: AtomicBool,
    retry_calls: AtomicU64,
    reject_retry: AtomicBool,
}

impl Driver {
    fn with_jobs(names: &[&str]) -> Arc<Self> {
        let driver = Self::default();
        *driver.jobs.lock().unwrap() = names
            .iter()
            .enumerate()
            .map(|(index, name)| QueuedJob {
                id: format!("job-{index}"),
                name: (*name).to_string(),
                payload: serde_json::json!({}),
                attempts: 1,
            })
            .collect();
        driver.defer_supported.store(true, Ordering::SeqCst);
        Arc::new(driver)
    }
}

struct Shared(Arc<Driver>);

#[async_trait]
impl QueueDriver for Shared {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        Ok(self.0.jobs.lock().unwrap().pop_front())
    }

    async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
        if self.0.fail_complete.load(Ordering::SeqCst) {
            return Err(QueueError::Driver("write failed".to_string()));
        }
        Ok(())
    }

    async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn requeue(&self, _job_id: &str, _reason: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn requeue_attempt_after(
        &self,
        _job_id: &str,
        _attempt: u32,
        _reason: &str,
        _delay: Duration,
    ) -> Result<(), QueueError> {
        if self.0.defer_supported.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(QueueError::Unsupported("no deferral".to_string()))
        }
    }

    async fn recover_stalled(&self, _stale_after: Duration) -> Result<u64, QueueError> {
        Ok(self.0.first_recovery.swap(0, Ordering::SeqCst))
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(self.0.jobs.lock().unwrap().len() as u64)
    }

    async fn retry_failed_job(&self, _job_id: &str) -> Result<(), QueueError> {
        self.0.retry_calls.fetch_add(1, Ordering::SeqCst);
        if self.0.reject_retry.load(Ordering::SeqCst) {
            Err(QueueError::Driver("no such job".to_string()))
        } else {
            Ok(())
        }
    }
}

fn queue(driver: &Arc<Driver>) -> Queue {
    Queue::custom(Box::new(Shared(Arc::clone(driver))))
}

/// Polls `condition` until it holds, failing the test after ten seconds.
async fn eventually(queue: &Queue, condition: impl Fn(QueueActivitySnapshot) -> bool) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !condition(queue.activity().snapshot()) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "activity never reached the expected state: {:?}",
            queue.activity().snapshot()
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

fn worker(queue: &Queue) -> Worker {
    let mut worker = Worker::new(queue).poll_interval(2);
    worker.register("ok", |_| async { Ok(()) });
    worker.register("boom", |_| async { Err("handler failed".into()) });
    worker
}

#[tokio::test]
async fn completed_failed_handed_back_and_recovered_jobs_are_counted() {
    let driver = Driver::with_jobs(&["ok", "boom", "unknown", "ok"]);
    driver.first_recovery.store(3, Ordering::SeqCst);
    let queue = queue(&driver);
    let handle = worker(&queue).run().unwrap();

    eventually(&queue, |activity| {
        activity.completed == 2 && activity.failed == 1 && activity.retried == 1
    })
    .await;
    let _ = shut_down(handle).await;

    let activity = queue.activity().snapshot();
    assert_eq!(activity.completed, 2);
    assert_eq!(activity.failed, 1);
    assert_eq!(activity.retried, 1);
    assert_eq!(activity.stalled_recovered, 3);
    assert_eq!(activity.in_flight, 0);
}

#[tokio::test]
async fn a_job_without_handler_is_counted_as_failed_when_it_cannot_be_handed_back() {
    let driver = Driver::with_jobs(&["unknown"]);
    driver.defer_supported.store(false, Ordering::SeqCst);
    let queue = queue(&driver);
    let handle = worker(&queue).run().unwrap();

    eventually(&queue, |activity| activity.failed == 1).await;
    let _ = shut_down(handle).await;
    assert_eq!(queue.activity().snapshot().retried, 0);
}

#[tokio::test]
async fn a_running_handler_is_in_flight_until_it_finishes() {
    let driver = Driver::with_jobs(&["slow"]);
    let queue = queue(&driver);
    let release = Arc::new(tokio::sync::Notify::new());
    let mut worker = Worker::new(&queue).poll_interval(2);
    let gate = Arc::clone(&release);
    worker.register("slow", move |_| {
        let gate = Arc::clone(&gate);
        async move {
            gate.notified().await;
            Ok(())
        }
    });
    let handle = worker.run().unwrap();

    eventually(&queue, |activity| activity.in_flight == 1).await;
    assert_eq!(queue.activity().snapshot().completed, 0);
    release.notify_one();
    eventually(&queue, |activity| {
        activity.in_flight == 0 && activity.completed == 1
    })
    .await;
    let _ = shut_down(handle).await;
}

#[tokio::test]
async fn an_interrupted_handler_is_counted_as_retried_and_leaves_flight() {
    let driver = Driver::with_jobs(&["forever"]);
    let queue = queue(&driver);
    let mut worker = Worker::new(&queue).poll_interval(2);
    worker.register("forever", |_| std::future::pending());
    let handle = worker.run().unwrap();

    eventually(&queue, |activity| activity.in_flight == 1).await;
    shut_down(handle).await.unwrap();

    let activity = queue.activity().snapshot();
    assert_eq!(activity.retried, 1);
    assert_eq!(activity.in_flight, 0);
    assert_eq!(activity.failed, 0);
}

#[tokio::test]
async fn a_timed_out_handler_is_counted_as_failed() {
    let driver = Driver::with_jobs(&["forever"]);
    let queue = queue(&driver);
    let mut worker = Worker::new(&queue)
        .poll_interval(2)
        .job_timeout(Duration::from_millis(20));
    worker.register("forever", |_| std::future::pending());
    let handle = worker.run().unwrap();

    eventually(&queue, |activity| activity.failed == 1).await;
    let _ = shut_down(handle).await;
    assert_eq!(queue.activity().snapshot().in_flight, 0);
}

#[tokio::test]
async fn a_completion_the_driver_rejects_is_not_counted() {
    let driver = Driver::with_jobs(&["ok"]);
    driver.fail_complete.store(true, Ordering::SeqCst);
    let queue = queue(&driver);
    let mut handle = worker(&queue).run().unwrap();

    let error = super::test_support::next_error(&mut handle).await.unwrap();
    assert!(matches!(error, QueueError::StateTransition { .. }));
    let _ = shut_down(handle).await;
    let activity = queue.activity().snapshot();
    assert_eq!(activity.completed, 0);
    assert_eq!(activity.in_flight, 0);
}

#[tokio::test]
async fn a_manual_retry_is_counted_only_when_the_driver_accepts_it() {
    let driver = Driver::with_jobs(&[]);
    let queue = queue(&driver);

    queue.retry_failed_job("job-1").await.unwrap();
    assert_eq!(driver.retry_calls.load(Ordering::SeqCst), 1);
    assert_eq!(queue.activity().snapshot().retried, 1);

    driver.reject_retry.store(true, Ordering::SeqCst);
    assert!(matches!(
        queue.retry_failed_job("job-2").await,
        Err(QueueError::Driver(_))
    ));
    assert_eq!(driver.retry_calls.load(Ordering::SeqCst), 2);
    assert_eq!(queue.activity().snapshot().retried, 1);
}

#[tokio::test]
async fn workers_built_from_one_queue_share_its_counters() {
    let driver = Driver::with_jobs(&["ok", "ok"]);
    let queue = queue(&driver);
    let first = worker(&queue).run().unwrap();
    let second = worker(&queue).run().unwrap();

    eventually(&queue, |activity| activity.completed == 2).await;
    let _ = shut_down(first).await;
    let _ = shut_down(second).await;
}
