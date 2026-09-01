#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

fn task_for_test<F, Fut>(handler: F) -> ScheduledTask
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    Scheduler::new()
        .task("* * * * *", handler)
        .expect("valid cron")
        .tasks
        .remove(0)
}

fn handle_with(
    loops: Vec<(String, JoinHandle<()>)>,
    errors: mpsc::UnboundedReceiver<SchedulerError>,
) -> SchedulerHandle {
    let (shutdown, _) = watch::channel(false);
    SchedulerHandle {
        shutdown,
        loops,
        errors,
    }
}

#[tokio::test]
async fn completed_handler_and_closed_shutdown_channel_are_observable() {
    let task = task_for_test(|| async {});
    let (_shutdown_tx, mut shutdown) = watch::channel(false);
    assert!(matches!(
        execute_handler(&task, Duration::from_secs(1), &mut shutdown)
            .await
            .expect("completed handler"),
        ExecutionStatus::Completed
    ));

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    drop(shutdown_tx);
    tokio::time::timeout(
        Duration::from_millis(50),
        wait_for_shutdown(&mut shutdown_rx),
    )
    .await
    .expect("closed channel terminates wait");
}

#[tokio::test]
async fn handle_reports_queued_and_join_failures_but_ignores_cancellation() {
    let (errors_tx, errors_rx) = mpsc::unbounded_channel();
    errors_tx
        .send(SchedulerError::TaskTimedOut {
            label: "queued".to_string(),
            timeout_ms: 5,
        })
        .expect("error receiver alive");
    drop(errors_tx);
    let queued = handle_with(vec![], errors_rx).shutdown().await;
    assert!(matches!(
        queued,
        Err(SchedulerError::TaskTimedOut { timeout_ms: 5, .. })
    ));

    let (_errors_tx, errors_rx) = mpsc::unbounded_channel();
    let panicking = tokio::spawn(async { panic!("isolated loop panic") });
    let joined = handle_with(vec![("panic-loop".to_string(), panicking)], errors_rx)
        .shutdown()
        .await;
    assert!(matches!(
        joined,
        Err(SchedulerError::LoopFailed { label, .. }) if label == "panic-loop"
    ));

    let (_errors_tx, errors_rx) = mpsc::unbounded_channel();
    let cancelled = tokio::spawn(std::future::pending::<()>());
    cancelled.abort();
    assert!(
        handle_with(vec![("cancelled".to_string(), cancelled)], errors_rx)
            .shutdown()
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn next_error_and_drop_abort_have_deterministic_lifecycles() {
    let (errors_tx, errors_rx) = mpsc::unbounded_channel();
    errors_tx
        .send(SchedulerError::TaskPanicked {
            label: "task".to_string(),
        })
        .expect("error receiver alive");
    drop(errors_tx);
    let mut handle = handle_with(vec![], errors_rx);
    assert!(matches!(
        handle.next_error().await,
        Some(SchedulerError::TaskPanicked { .. })
    ));
    assert!(handle.next_error().await.is_none());

    struct Dropped(Arc<AtomicBool>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let started = Arc::new(AtomicBool::new(false));
    let dropped = Arc::new(AtomicBool::new(false));
    let started_in_task = Arc::clone(&started);
    let dropped_in_task = Arc::clone(&dropped);
    let task = tokio::spawn(async move {
        let _guard = Dropped(dropped_in_task);
        started_in_task.store(true, Ordering::SeqCst);
        std::future::pending::<()>().await;
    });
    while !started.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }
    let (_errors_tx, errors_rx) = mpsc::unbounded_channel();
    drop(handle_with(vec![("pending".to_string(), task)], errors_rx));
    tokio::time::timeout(Duration::from_millis(100), async {
        while !dropped.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("dropping handle aborts its tasks");
}

#[test]
fn scheduler_defaults_and_duration_saturation_are_explicit() {
    let scheduler = Scheduler::default();
    assert!(scheduler.tasks.is_empty());
    assert_eq!(scheduler.task_timeout, Duration::from_secs(300));
    assert_eq!(scheduler.failure_policy, SchedulerFailurePolicy::Continue);
    assert_eq!(duration_millis_u64(Duration::MAX), u64::MAX);
}
