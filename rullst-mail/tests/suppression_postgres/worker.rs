use super::support::*;
use rullst_core::queue::{Queue, QueueDriver, QueueError, QueuedJob, Worker};
use rullst_mail::{Mail, TenantMailResolver, register_mail_handler};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
struct State {
    jobs: Mutex<VecDeque<QueuedJob>>,
    completed: AtomicUsize,
    failed: AtomicUsize,
}
struct FixtureQueue(Arc<State>);
#[async_trait::async_trait]
impl QueueDriver for FixtureQueue {
    async fn push(&self, id: &str, name: &str, payload: &str) -> Result<(), QueueError> {
        let mut jobs = self.0.jobs.lock().unwrap();
        assert!(jobs.len() < 16);
        jobs.push_back(QueuedJob {
            id: id.to_owned(),
            name: name.to_owned(),
            payload: serde_json::from_str(payload).unwrap(),
            attempts: 1,
        });
        Ok(())
    }
    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        Ok(self.0.jobs.lock().unwrap().pop_front())
    }
    async fn mark_complete(&self, _: &str) -> Result<(), QueueError> {
        self.0.completed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn mark_failed(&self, _: &str, error: &str) -> Result<(), QueueError> {
        assert!(!error.contains("queued@example.com"));
        self.0.failed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn pending_count(&self) -> Result<u64, QueueError> {
        Ok(self.0.jobs.lock().unwrap().len() as u64)
    }
    async fn recover_stalled(&self, _: Duration) -> Result<u64, QueueError> {
        Ok(0)
    }
}
struct ResetDriver;
impl Drop for ResetDriver {
    fn drop(&mut self) {
        Mail::reset_driver();
    }
}

pub async fn run(url: &str) {
    let first = PostgresSuppressionStore::initialize(url, key(), config(&unique()))
        .await
        .unwrap();
    let second = PostgresSuppressionStore::initialize(url, key(), config(&unique()))
        .await
        .unwrap();
    let (driver_a, inbox_a) = MemoryDriver::isolated();
    let (driver_b, inbox_b) = MemoryDriver::isolated();
    let resolver = TenantMailResolver::new();
    resolver
        .register("school-a", SuppressionGuard::new(driver_a, first.clone()))
        .unwrap();
    resolver
        .register("school-b", SuppressionGuard::new(driver_b, second.clone()))
        .unwrap();
    Mail::set_driver(Box::new(resolver));
    let _reset = ResetDriver;
    let state = Arc::new(State::default());
    let queue = Queue::custom(Box::new(FixtureQueue(state.clone())));
    assert!(first.lookup("queued@example.com").await.unwrap().is_none());
    Mail::enqueue_for_tenant(&queue, "school-a", message("queued@example.com"))
        .await
        .unwrap();
    Mail::enqueue_for_tenant(&queue, "school-b", message("queued@example.com"))
        .await
        .unwrap();
    // Feedback arriving after enqueue still prevents delivery at worker time.
    first
        .record(event(
            "queued-complaint",
            "queued@example.com",
            SuppressionReason::SpamComplaint,
            now(),
        ))
        .await
        .unwrap();
    let mut worker = Worker::new(&queue).max_concurrency(1).poll_interval(10);
    register_mail_handler(&mut worker);
    let mut handle = worker.run().unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        while state.completed.load(Ordering::SeqCst) != 1
            || state.failed.load(Ordering::SeqCst) != 1
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let error = tokio::time::timeout(Duration::from_secs(2), handle.next_error())
        .await
        .unwrap();
    assert!(error.is_some());
    handle.shutdown().await.unwrap();
    assert!(inbox_a.lock().unwrap().is_empty());
    assert_eq!(inbox_b.lock().unwrap().len(), 1);
    first.close().await;
    second.close().await;
}
