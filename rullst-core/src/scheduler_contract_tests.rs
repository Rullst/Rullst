#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

fn errors() -> (ErrorReporter<SchedulerError>, ErrorBuffer<SchedulerError>) {
    error_buffer(ERROR_BUFFER_CAPACITY, "test")
}

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

/// A scheduler with one task that fires every second, for lifecycle tests
/// that cannot wait for a whole cron minute.
pub(crate) fn every_second<F, Fut>(timeout: Duration, handler: F) -> Scheduler
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let handler: ScheduledHandler = Arc::new(Box::new(move || Box::pin(handler())));
    Scheduler {
        tasks: vec![ScheduledTask {
            label: "every second".to_string(),
            schedule: CronSchedule::every_second(),
            handler,
        }],
        task_timeout: timeout,
        failure_policy: SchedulerFailurePolicy::Continue,
    }
}

fn handle_with(
    loops: Vec<(String, JoinHandle<()>)>,
    errors: ErrorBuffer<SchedulerError>,
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
    let (errors_tx, errors_rx) = errors();
    errors_tx.report(SchedulerError::TaskTimedOut {
        label: "queued".to_string(),
        timeout_ms: 5,
    });
    drop(errors_tx);
    let queued = handle_with(vec![], errors_rx).shutdown().await;
    assert!(matches!(
        queued,
        Err(SchedulerError::TaskTimedOut { timeout_ms: 5, .. })
    ));

    let (_errors_tx, errors_rx) = errors();
    let panicking = tokio::spawn(async { panic!("isolated loop panic") });
    let joined = handle_with(vec![("panic-loop".to_string(), panicking)], errors_rx)
        .shutdown()
        .await;
    assert!(matches!(
        joined,
        Err(SchedulerError::LoopFailed { label, .. }) if label == "panic-loop"
    ));

    let (_errors_tx, errors_rx) = errors();
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
    let (errors_tx, errors_rx) = errors();
    errors_tx.report(SchedulerError::TaskPanicked {
        label: "task".to_string(),
    });
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
    let (_errors_tx, errors_rx) = errors();
    drop(handle_with(vec![("pending".to_string(), task)], errors_rx));
    tokio::time::timeout(Duration::from_millis(100), async {
        while !dropped.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("dropping handle aborts its tasks");
}

#[tokio::test]
async fn undrained_failures_are_bounded_and_counted() {
    let (errors_tx, errors_rx) = errors();
    for _ in 0..ERROR_BUFFER_CAPACITY + 3 {
        errors_tx.report(SchedulerError::TaskPanicked {
            label: "task".to_string(),
        });
    }
    let mut handle = handle_with(vec![], errors_rx);
    assert_eq!(handle.dropped_errors(), 3);

    let mut buffered = 0;
    while handle.try_next_error().is_some() {
        buffered += 1;
    }
    assert_eq!(buffered, ERROR_BUFFER_CAPACITY);
}

#[test]
fn scheduler_defaults_and_duration_saturation_are_explicit() {
    let scheduler = Scheduler::default();
    assert!(scheduler.tasks.is_empty());
    assert_eq!(scheduler.task_timeout, Duration::from_secs(300));
    assert_eq!(scheduler.failure_policy, SchedulerFailurePolicy::Continue);
    assert_eq!(duration_millis_u64(Duration::MAX), u64::MAX);
}

#[test]
fn registered_tasks_use_posix_weekday_numbering() {
    use chrono::{Datelike, TimeZone};

    assert!(Scheduler::new().task("0 3 * * 0", || async {}).is_ok());
    let task = Scheduler::new()
        .task("0 9 * * 1-5", || async {})
        .expect("weekday schedule")
        .tasks
        .remove(0);
    // Thursday 2026-10-01 09:00 UTC is followed by Friday, then Monday.
    let thursday = chrono::Utc
        .with_ymd_and_hms(2026, 10, 1, 9, 0, 0)
        .single()
        .expect("valid instant");
    let friday = task.schedule.next_after(&thursday).expect("next run");
    assert_eq!(friday.weekday(), chrono::Weekday::Fri);
    let monday = task.schedule.next_after(&friday).expect("next run");
    assert_eq!(monday.weekday(), chrono::Weekday::Mon);
}
