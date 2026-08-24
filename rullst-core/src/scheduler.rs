//! # Rullst Task Scheduler (`rullst::scheduler`)
//!
//! Declarative cron jobs with bounded, observable execution lifecycles.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

/// Strongly-typed error domain for scheduler operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchedulerError {
    /// Invalid cron expression syntax.
    #[error("invalid cron expression '{0}': {1}")]
    InvalidCron(String, String),
    /// The scheduler was started outside an active Tokio runtime.
    #[error("the scheduler requires an active Tokio runtime")]
    RuntimeUnavailable,
    /// A task exceeded its configured execution deadline.
    #[error("scheduled task '{label}' exceeded its {timeout_ms}ms timeout")]
    TaskTimedOut {
        /// Cron expression identifying the task.
        label: String,
        /// Configured timeout in milliseconds.
        timeout_ms: u64,
    },
    /// A task panicked. The panic is contained in its isolated Tokio task.
    #[error("scheduled task '{label}' panicked")]
    TaskPanicked {
        /// Cron expression identifying the task.
        label: String,
    },
    /// A task execution was unexpectedly cancelled.
    #[error("scheduled task '{label}' was cancelled")]
    TaskCancelled {
        /// Cron expression identifying the task.
        label: String,
    },
    /// A schedule no longer has a future execution.
    #[error("scheduled task '{label}' has no future execution")]
    ScheduleExhausted {
        /// Cron expression identifying the task.
        label: String,
    },
    /// A scheduler loop terminated unexpectedly.
    #[error("scheduler loop for '{label}' failed: {message}")]
    LoopFailed {
        /// Cron expression identifying the task.
        label: String,
        /// Runtime failure description.
        message: String,
    },
}

/// Action taken after a timeout, panic, or unexpected cancellation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchedulerFailurePolicy {
    /// Report the typed failure and continue with the next future cron tick.
    #[default]
    Continue,
    /// Report the typed failure and permanently stop that task's loop.
    StopTask,
}

/// The boxed async handler function type for scheduled tasks.
pub type ScheduledHandler =
    Arc<Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>>;

/// A single scheduled task with a cron expression and async handler.
pub struct ScheduledTask {
    label: String,
    schedule: cron::Schedule,
    handler: ScheduledHandler,
}

/// A declarative scheduler for recurring asynchronous jobs.
///
/// Each registered task has exactly one serial execution loop. A slow run
/// therefore skips already-missed cron instants instead of creating unbounded
/// overlapping tasks. Handler futures execute in isolated Tokio tasks so their
/// panics can be converted into [`SchedulerError`] values.
pub struct Scheduler {
    tasks: Vec<ScheduledTask>,
    task_timeout: Duration,
    failure_policy: SchedulerFailurePolicy,
}

impl Scheduler {
    /// Creates an empty scheduler with a five-minute handler timeout.
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            task_timeout: Duration::from_secs(300),
            failure_policy: SchedulerFailurePolicy::Continue,
        }
    }

    /// Sets the maximum duration of one handler execution.
    pub fn with_task_timeout(mut self, timeout: Duration) -> Self {
        self.task_timeout = timeout;
        self
    }

    /// Selects whether a failing task continues at its next tick or stops.
    pub fn with_failure_policy(mut self, policy: SchedulerFailurePolicy) -> Self {
        self.failure_policy = policy;
        self
    }

    /// Registers a recurring task using a standard five-field cron expression.
    ///
    /// # Errors
    /// Returns [`SchedulerError::InvalidCron`] when the expression cannot be
    /// parsed.
    pub fn task<F, Fut>(mut self, cron_expr: &str, handler: F) -> Result<Self, SchedulerError>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let full_expr = format!("0 {cron_expr} *");
        let schedule: cron::Schedule = full_expr.parse().map_err(|error: cron::error::Error| {
            SchedulerError::InvalidCron(cron_expr.to_string(), error.to_string())
        })?;
        let boxed: ScheduledHandler = Arc::new(Box::new(move || Box::pin(handler())));

        self.tasks.push(ScheduledTask {
            label: cron_expr.to_string(),
            schedule,
            handler: boxed,
        });
        Ok(self)
    }

    /// Starts every registered task and returns its lifecycle handle.
    ///
    /// Dropping the handle aborts all scheduler loops and their current handler
    /// futures. Prefer [`SchedulerHandle::shutdown`] for graceful cancellation.
    ///
    /// # Errors
    /// Returns [`SchedulerError::RuntimeUnavailable`] without spawning anything
    /// when called outside Tokio.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn start(self) -> Result<SchedulerHandle, SchedulerError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| SchedulerError::RuntimeUnavailable)?;
        let (shutdown, _) = watch::channel(false);
        let (errors_tx, errors) = mpsc::unbounded_channel();
        let mut loops = Vec::with_capacity(self.tasks.len());

        for task in self.tasks {
            let label = task.label.clone();
            let shutdown_rx = shutdown.subscribe();
            let errors_tx = errors_tx.clone();
            let timeout = self.task_timeout;
            let policy = self.failure_policy;
            let task_loop =
                runtime.spawn(run_task_loop(task, timeout, policy, shutdown_rx, errors_tx));
            loops.push((label, task_loop));
        }
        drop(errors_tx);

        Ok(SchedulerHandle {
            shutdown,
            loops,
            errors,
        })
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Owns running scheduler loops and exposes their typed failures.
#[must_use = "dropping the scheduler handle immediately stops all scheduled tasks"]
pub struct SchedulerHandle {
    shutdown: watch::Sender<bool>,
    loops: Vec<(String, tokio::task::JoinHandle<()>)>,
    errors: mpsc::UnboundedReceiver<SchedulerError>,
}

impl SchedulerHandle {
    /// Waits for the next timeout, panic, or runtime failure reported by a task.
    pub async fn next_error(&mut self) -> Option<SchedulerError> {
        self.errors.recv().await
    }

    /// Returns a pending scheduler failure without waiting.
    pub fn try_next_error(&mut self) -> Option<SchedulerError> {
        self.errors.try_recv().ok()
    }

    /// Gracefully stops task loops, aborting any current handler execution.
    ///
    /// # Errors
    /// Returns the first reported task failure, or a typed loop join failure.
    pub async fn shutdown(mut self) -> Result<(), SchedulerError> {
        let _ = self.shutdown.send(true);
        let mut first_error = self.try_next_error();

        for (label, task_loop) in self.loops.drain(..) {
            if let Err(error) = task_loop.await
                && !error.is_cancelled()
                && first_error.is_none()
            {
                first_error = Some(SchedulerError::LoopFailed {
                    label,
                    message: error.to_string(),
                });
            }
        }

        first_error = first_error.or_else(|| self.try_next_error());
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn abort(&mut self) {
        let _ = self.shutdown.send(true);
        for (_, task_loop) in &self.loops {
            task_loop.abort();
        }
        self.loops.clear();
    }
}

impl Drop for SchedulerHandle {
    fn drop(&mut self) {
        self.abort();
    }
}

#[cfg_attr(mutants, mutants::skip)]
async fn run_task_loop(
    task: ScheduledTask,
    timeout: Duration,
    policy: SchedulerFailurePolicy,
    mut shutdown: watch::Receiver<bool>,
    errors: mpsc::UnboundedSender<SchedulerError>,
) {
    loop {
        if shutdown_requested(&shutdown) {
            break;
        }

        let now = chrono::Utc::now();
        let Some(next) = task.schedule.upcoming(chrono::Utc).next() else {
            let _ = errors.send(SchedulerError::ScheduleExhausted {
                label: task.label.clone(),
            });
            break;
        };
        let wait = (next - now).to_std().unwrap_or(Duration::ZERO);

        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = wait_for_shutdown(&mut shutdown) => break,
        }

        match execute_handler(&task, timeout, &mut shutdown).await {
            Ok(ExecutionStatus::Completed) => {}
            Ok(ExecutionStatus::ShutDown) => break,
            Err(error) => {
                let _ = errors.send(error);
                if policy == SchedulerFailurePolicy::StopTask {
                    break;
                }
            }
        }
    }
}

enum ExecutionStatus {
    Completed,
    ShutDown,
}

#[cfg_attr(mutants, mutants::skip)]
async fn execute_handler(
    task: &ScheduledTask,
    timeout: Duration,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<ExecutionStatus, SchedulerError> {
    let handler = Arc::clone(&task.handler);
    let mut execution = AbortOnDrop(tokio::spawn(async move { handler().await }));
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);

    tokio::select! {
        result = &mut execution.0 => match result {
            Ok(()) => Ok(ExecutionStatus::Completed),
            Err(error) if error.is_panic() => Err(SchedulerError::TaskPanicked {
                label: task.label.clone(),
            }),
            Err(_) => Err(SchedulerError::TaskCancelled {
                label: task.label.clone(),
            }),
        },
        _ = &mut deadline => {
            execution.0.abort();
            let _ = (&mut execution.0).await;
            Err(SchedulerError::TaskTimedOut {
                label: task.label.clone(),
                timeout_ms: duration_millis_u64(timeout),
            })
        }
        _ = wait_for_shutdown(shutdown) => {
            execution.0.abort();
            let _ = (&mut execution.0).await;
            Ok(ExecutionStatus::ShutDown)
        }
    }
}

fn shutdown_requested(shutdown: &watch::Receiver<bool>) -> bool {
    *shutdown.borrow()
}

async fn wait_for_shutdown(shutdown: &mut watch::Receiver<bool>) {
    while !shutdown_requested(shutdown) {
        if shutdown.changed().await.is_err() {
            break;
        }
    }
}

fn duration_millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

struct AbortOnDrop<T>(JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
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
            schedule: "* * * * * * *".parse().unwrap(),
            handler,
        };
        let (shutdown_tx, shutdown) = watch::channel(false);
        let (errors_tx, mut errors) = mpsc::unbounded_channel();
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
        assert!(errors.try_recv().is_err());
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
}
