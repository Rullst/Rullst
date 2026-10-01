//! Unit tests for scheduler task loops and their failure handling.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

fn task_for_test<F, Fut>(handler: F) -> ScheduledTask
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    Scheduler::new()
        .task("* * * * *", handler)
        .unwrap()
        .tasks
        .remove(0)
}

#[test]
fn scheduler_start_without_runtime_is_fallible() {
    let result = Scheduler::new().start();
    assert!(matches!(result, Err(SchedulerError::RuntimeUnavailable)));
}

#[test]
fn invalid_cron_is_rejected() {
    let result = Scheduler::new().task("invalid cron", || async {});
    assert!(matches!(result, Err(SchedulerError::InvalidCron(_, _))));
}

#[tokio::test]
async fn handler_timeout_is_typed_and_contained() {
    let task = task_for_test(|| async {
        tokio::time::sleep(Duration::from_secs(1)).await;
    });
    let (_shutdown_tx, mut shutdown) = watch::channel(false);
    let result = execute_handler(&task, Duration::from_millis(5), &mut shutdown).await;

    assert!(matches!(
        result,
        Err(SchedulerError::TaskTimedOut { timeout_ms: 5, .. })
    ));
}

#[tokio::test]
async fn handler_panic_is_typed_and_does_not_escape() {
    let task = task_for_test(|| async { panic!("test panic") });
    let (_shutdown_tx, mut shutdown) = watch::channel(false);
    let result = execute_handler(&task, Duration::from_secs(1), &mut shutdown).await;

    assert!(matches!(result, Err(SchedulerError::TaskPanicked { .. })));
}

#[tokio::test]
async fn explicit_shutdown_terminates_sleeping_loops() {
    let scheduler = Scheduler::new().task("* * * * *", || async {}).unwrap();
    let handle = scheduler.start().unwrap();
    let result = tokio::time::timeout(Duration::from_millis(100), handle.shutdown()).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_ok());
}

#[tokio::test]
async fn shutdown_aborts_current_handler() {
    let task = task_for_test(|| async {
        std::future::pending::<()>().await;
    });
    let (shutdown_tx, mut shutdown) = watch::channel(false);
    let execution = tokio::spawn(async move {
        execute_handler(&task, Duration::from_secs(60), &mut shutdown).await
    });
    tokio::task::yield_now().await;
    shutdown_tx.send(true).unwrap();

    assert!(matches!(
        execution.await.unwrap().unwrap(),
        ExecutionStatus::ShutDown
    ));
}

#[tokio::test]
async fn task_loop_never_overlaps_slow_executions() {
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let executions = Arc::new(AtomicUsize::new(0));
    let active_for_handler = Arc::clone(&active);
    let maximum_for_handler = Arc::clone(&maximum);
    let executions_for_handler = Arc::clone(&executions);
    let handler: ScheduledHandler = Arc::new(Box::new(move || {
        let active = Arc::clone(&active_for_handler);
        let maximum = Arc::clone(&maximum_for_handler);
        let executions = Arc::clone(&executions_for_handler);
        Box::pin(async move {
            let running = active.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(running, Ordering::SeqCst);
            executions.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(1_100)).await;
            active.fetch_sub(1, Ordering::SeqCst);
        })
    }));
    let task = ScheduledTask {
        label: "every second".to_string(),
        schedule: CronSchedule::every_second(),
        handler,
    };
    let (shutdown_tx, shutdown) = watch::channel(false);
    let (errors_tx, mut errors) = error_buffer(ERROR_BUFFER_CAPACITY, "test");
    let task_loop = tokio::spawn(run_task_loop(
        task,
        Duration::from_secs(5),
        SchedulerFailurePolicy::Continue,
        shutdown,
        errors_tx,
    ));

    tokio::time::sleep(Duration::from_millis(2_300)).await;
    shutdown_tx.send(true).unwrap();
    task_loop.await.unwrap();

    assert!(executions.load(Ordering::SeqCst) >= 1);
    assert_eq!(maximum.load(Ordering::SeqCst), 1);
    assert!(errors.try_next().is_none());
}

#[tokio::test]
async fn aborting_execution_drops_the_handler_future() {
    struct Dropped(Arc<AtomicBool>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let started = Arc::new(AtomicBool::new(false));
    let dropped = Arc::new(AtomicBool::new(false));
    let started_for_handler = Arc::clone(&started);
    let dropped_for_handler = Arc::clone(&dropped);
    let task = task_for_test(move || {
        let started = Arc::clone(&started_for_handler);
        let dropped = Arc::clone(&dropped_for_handler);
        async move {
            let _drop_guard = Dropped(dropped);
            started.store(true, Ordering::SeqCst);
            std::future::pending::<()>().await;
        }
    });
    let (_shutdown_tx, mut shutdown) = watch::channel(false);
    let outer = tokio::spawn(async move {
        execute_handler(&task, Duration::from_secs(60), &mut shutdown).await
    });
    while !started.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }
    outer.abort();
    let _ = outer.await;
    tokio::time::timeout(Duration::from_millis(100), async {
        while !dropped.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
