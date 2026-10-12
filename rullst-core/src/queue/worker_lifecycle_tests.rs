//! Worker lifecycle: periodic stalled-lease recovery, a dropped handle and a
//! worker task that panics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use async_trait::async_trait;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

const BUDGET: Duration = Duration::from_secs(10);

/// Sends once when dropped, so a test can see a future being cancelled.
struct DropSignal(Option<mpsc::UnboundedSender<()>>);

impl Drop for DropSignal {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

/// Serves `jobs`, counts requeues and reports every stalled-lease recovery.
/// The second recovery, the first one made by the periodic loop, never
/// returns, and reports when it is cancelled.
struct ProbeDriver {
    jobs: Mutex<Vec<QueuedJob>>,
    requeued: Arc<AtomicUsize>,
    recoveries: AtomicUsize,
    recovery_started: mpsc::UnboundedSender<usize>,
    recovery_cancelled: mpsc::UnboundedSender<()>,
}

#[async_trait]
impl QueueDriver for ProbeDriver {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        Ok(self.jobs.lock().unwrap().pop())
    }

    async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn requeue(&self, _job_id: &str, _reason: &str) -> Result<(), QueueError> {
        self.requeued.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn recover_stalled(&self, _stale_after: Duration) -> Result<u64, QueueError> {
        let call = self.recoveries.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.recovery_started.send(call);
        if call >= 2 {
            let _cancelled = DropSignal(Some(self.recovery_cancelled.clone()));
            std::future::pending::<()>().await;
        }
        Ok(0)
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(0)
    }
}

struct Probe {
    queue: Queue,
    requeued: Arc<AtomicUsize>,
    recovery_started: mpsc::UnboundedReceiver<usize>,
    recovery_cancelled: mpsc::UnboundedReceiver<()>,
}

fn probe(jobs: Vec<QueuedJob>) -> Probe {
    let (started_tx, recovery_started) = mpsc::unbounded_channel();
    let (cancelled_tx, recovery_cancelled) = mpsc::unbounded_channel();
    let requeued = Arc::new(AtomicUsize::new(0));
    let driver = ProbeDriver {
        jobs: Mutex::new(jobs),
        requeued: Arc::clone(&requeued),
        recoveries: AtomicUsize::new(0),
        recovery_started: started_tx,
        recovery_cancelled: cancelled_tx,
    };
    Probe {
        queue: Queue::custom(Box::new(driver)),
        requeued,
        recovery_started,
        recovery_cancelled,
    }
}

async fn within_budget<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(BUDGET, future)
        .await
        .expect("the worker makes progress within the budget")
}

#[tokio::test]
async fn stalled_leases_are_recovered_periodically_until_the_handle_is_dropped() {
    let mut probe = probe(Vec::new());
    // A one-second lease age gives the shortest recovery interval, one second.
    let handle = Worker::new(&probe.queue)
        .poll_interval(5)
        .job_timeout(Duration::from_millis(500))
        .stalled_after(Duration::from_secs(1))
        .run()
        .unwrap();

    assert_eq!(within_budget(probe.recovery_started.recv()).await, Some(1));
    assert_eq!(within_budget(probe.recovery_started.recv()).await, Some(2));

    // Dropping the handle aborts the recovery task with the worker.
    drop(handle);
    assert_eq!(
        within_budget(probe.recovery_cancelled.recv()).await,
        Some(())
    );
}

#[tokio::test]
async fn dropping_the_handle_aborts_running_handlers_without_requeuing_them() {
    let probe = probe(vec![QueuedJob {
        id: "job-1".to_string(),
        name: "test".to_string(),
        payload: serde_json::json!({}),
        attempts: 1,
    }]);
    let (started_tx, mut started) = mpsc::unbounded_channel();
    let (cancelled_tx, mut cancelled) = mpsc::unbounded_channel();
    let mut worker = Worker::new(&probe.queue).poll_interval(5);
    worker.register("test", move |_| {
        let started = started_tx.clone();
        let cancelled = DropSignal(Some(cancelled_tx.clone()));
        async move {
            let _cancelled = cancelled;
            let _ = started.send(());
            std::future::pending::<()>().await;
            Ok(())
        }
    });
    let handle = worker.run().unwrap();
    assert_eq!(within_budget(started.recv()).await, Some(()));

    drop(handle);
    assert_eq!(within_budget(cancelled.recv()).await, Some(()));
    // A graceful shutdown would requeue the interrupted job; an aborted
    // worker leaves it to stalled-lease recovery.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(probe.requeued.load(Ordering::SeqCst), 0);
}

/// Panics on its first claim, after reporting it.
struct PanickingDriver(mpsc::UnboundedSender<()>);

#[async_trait]
impl QueueDriver for PanickingDriver {
    async fn push(&self, _id: &str, _name: &str, _payload: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        let _ = self.0.send(());
        panic!("driver failure while claiming");
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
async fn shutdown_reports_a_worker_task_that_panicked() {
    let (claimed_tx, mut claimed) = mpsc::unbounded_channel();
    let queue = Queue::custom(Box::new(PanickingDriver(claimed_tx)));
    let handle = Worker::new(&queue).poll_interval(5).run().unwrap();
    assert_eq!(within_budget(claimed.recv()).await, Some(()));

    let result = within_budget(handle.shutdown()).await;
    assert!(
        matches!(result, Err(QueueError::WorkerTask(_))),
        "{result:?}"
    );
}
