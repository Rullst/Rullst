//! # Rullst Task Scheduler (`rullst::scheduler`)
//!
//! Declarative cron jobs with bounded, observable execution lifecycles.

use crate::error_buffer::{ERROR_BUFFER_CAPACITY, ErrorBuffer, ErrorReporter, error_buffer};
use cron_expression::CronSchedule;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
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

#[path = "scheduler_cron.rs"]
mod cron_expression;

/// A single scheduled task with a cron expression and async handler.
pub struct ScheduledTask {
    label: String,
    schedule: CronSchedule,
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

    /// Registers a recurring task using a POSIX five-field cron expression,
    /// evaluated in UTC: `minute hour day-of-month month day-of-week`.
    ///
    /// Day-of-week accepts 0-7 (0 and 7 are Sunday, 1 is Monday) and English
    /// names such as `MON`, in lists, ranges and range steps (`1-5`,
    /// `FRI-SUN`, `1-5/2`, `*/2`). As in POSIX cron, when both day-of-month
    /// and day-of-week are restricted (neither starts with `*`), the task runs
    /// on a day matching either field; otherwise both must match. Minute,
    /// hour, day-of-month and month use the usual ranges, names, lists and
    /// steps. There is no seconds or year field and no time-zone selection.
    ///
    /// # Errors
    /// Returns [`SchedulerError::InvalidCron`] when the expression does not
    /// have exactly five fields or a field is invalid.
    pub fn task<F, Fut>(mut self, cron_expr: &str, handler: F) -> Result<Self, SchedulerError>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let schedule = CronSchedule::parse(cron_expr)
            .map_err(|error| SchedulerError::InvalidCron(cron_expr.to_string(), error))?;
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
        let (errors_tx, errors) = error_buffer(ERROR_BUFFER_CAPACITY, "scheduler");
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
///
/// The handle buffers at most 256 undrained errors. When the buffer is full,
/// newer errors are dropped, counted by [`Self::dropped_errors`] and emitted as
/// `tracing` warnings. Drain [`Self::next_error`] to observe every failure.
#[must_use = "dropping the scheduler handle immediately stops all scheduled tasks"]
pub struct SchedulerHandle {
    shutdown: watch::Sender<bool>,
    loops: Vec<(String, tokio::task::JoinHandle<()>)>,
    errors: ErrorBuffer<SchedulerError>,
}

impl SchedulerHandle {
    /// Waits for the next timeout, panic, or runtime failure reported by a task.
    pub async fn next_error(&mut self) -> Option<SchedulerError> {
        self.errors.next().await
    }

    /// Returns a pending scheduler failure without waiting.
    pub fn try_next_error(&mut self) -> Option<SchedulerError> {
        self.errors.try_next()
    }

    /// Returns how many failures were dropped because the buffer was full.
    pub fn dropped_errors(&self) -> u64 {
        self.errors.dropped()
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
    errors: ErrorReporter<SchedulerError>,
) {
    let mut last_fired = None;
    loop {
        if shutdown_requested(&shutdown) {
            break;
        }

        let Some(next) = next_occurrence(&task.schedule, chrono::Utc::now(), last_fired) else {
            errors.report(SchedulerError::ScheduleExhausted {
                label: task.label.clone(),
            });
            break;
        };

        // The sleep is monotonic, but the schedule is wall-clock time: if the
        // wall clock was stepped back during the sleep, wait again until it
        // actually reaches `next` instead of firing early.
        loop {
            let wait = (next - chrono::Utc::now())
                .to_std()
                .unwrap_or(Duration::ZERO);
            if wait.is_zero() {
                break;
            }
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = wait_for_shutdown(&mut shutdown) => return,
            }
        }
        last_fired = Some(next);

        match execute_handler(&task, timeout, &mut shutdown).await {
            Ok(ExecutionStatus::Completed) => {}
            Ok(ExecutionStatus::ShutDown) => break,
            Err(error) => {
                errors.report(error);
                if policy == SchedulerFailurePolicy::StopTask {
                    break;
                }
            }
        }
    }
}

/// The first occurrence strictly after both `now` and the occurrence that
/// last fired, so one cron instant never runs twice even when the wall clock
/// lags the monotonic timer.
fn next_occurrence(
    schedule: &CronSchedule,
    now: chrono::DateTime<chrono::Utc>,
    last_fired: Option<chrono::DateTime<chrono::Utc>>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    let after = last_fired.map_or(now, |fired| fired.max(now));
    schedule.next_after(&after)
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
#[path = "scheduler_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "scheduler_contract_tests.rs"]
pub(crate) mod contract_tests;
