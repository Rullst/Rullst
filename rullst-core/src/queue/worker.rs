//! Bounded background queue worker and lifecycle handle.

use super::{Queue, QueueDriver, QueueError, QueuedJob};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::task::{JoinHandle, JoinSet};

mod execution;

/// Type alias for asynchronous job handler closures.
pub type JobHandler = Box<
    dyn Fn(
            Value,
        ) -> Pin<
            Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>,
        > + Send
        + Sync,
>;

/// Polls a queue and executes registered handlers with bounded concurrency.
pub struct Worker {
    driver: Arc<Box<dyn QueueDriver>>,
    handlers: HashMap<String, Arc<JobHandler>>,
    poll_interval: Duration,
    max_concurrency: usize,
    job_timeout: Duration,
    stalled_after: Duration,
}

impl Worker {
    /// Creates a worker with 16 concurrent slots and five-minute leases.
    pub fn new(queue: &Queue) -> Self {
        Self {
            driver: queue.driver_ref(),
            handlers: HashMap::new(),
            poll_interval: Duration::from_secs(1),
            max_concurrency: 16,
            job_timeout: Duration::from_secs(300),
            stalled_after: Duration::from_secs(600),
        }
    }

    /// Sets the idle polling interval in milliseconds.
    pub fn poll_interval(mut self, milliseconds: u64) -> Self {
        self.poll_interval = Duration::from_millis(milliseconds);
        self
    }

    /// Sets the hard upper bound for simultaneously executing handlers.
    pub fn max_concurrency(mut self, limit: usize) -> Self {
        self.max_concurrency = limit;
        self
    }

    /// Sets the maximum duration of an individual handler execution.
    ///
    /// A handler still running at the deadline is aborted and its job is
    /// failed as timed out. A handler that cannot be interrupted (for example
    /// one that blocks its thread) and then returns is recorded from its own
    /// result instead.
    pub fn job_timeout(mut self, timeout: Duration) -> Self {
        self.job_timeout = timeout;
        self
    }

    /// Sets the age after which a processing lease is recovered at startup.
    pub fn stalled_after(mut self, age: Duration) -> Self {
        self.stalled_after = age;
        self
    }

    /// Registers a handler for a job name.
    pub fn register<F, Fut>(&mut self, name: impl Into<String>, handler: F) -> &mut Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        let boxed: JobHandler = Box::new(move |payload| Box::pin(handler(payload)));
        self.handlers.insert(name.into(), Arc::new(boxed));
        self
    }

    /// Starts the polling loop and returns an observable lifecycle handle.
    ///
    /// # Errors
    /// Returns a typed error for zero concurrency, a zero polling interval, or
    /// invocation outside an active Tokio runtime. No task is spawned on error.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn run(&self) -> Result<WorkerHandle, QueueError> {
        if self.max_concurrency == 0 {
            return Err(QueueError::InvalidConfiguration(
                "max_concurrency must be greater than zero".to_string(),
            ));
        }
        if self.poll_interval.is_zero() {
            return Err(QueueError::InvalidConfiguration(
                "poll_interval must be greater than zero".to_string(),
            ));
        }
        if self.stalled_after <= self.job_timeout {
            return Err(QueueError::InvalidConfiguration(
                "stalled_after must be greater than job_timeout to protect active jobs".to_string(),
            ));
        }
        let runtime =
            tokio::runtime::Handle::try_current().map_err(|_| QueueError::RuntimeUnavailable)?;
        let (shutdown, shutdown_rx) = watch::channel(false);
        let (errors_tx, errors) = mpsc::unbounded_channel();
        let task = runtime.spawn(run_worker_loop(
            Arc::clone(&self.driver),
            self.handlers.clone(),
            self.poll_interval,
            self.max_concurrency,
            self.job_timeout,
            self.stalled_after,
            shutdown_rx,
            errors_tx,
        ));

        Ok(WorkerHandle {
            shutdown,
            task: Some(task),
            errors,
        })
    }
}

/// Owns a running queue worker and exposes asynchronous processing failures.
#[must_use = "dropping the worker handle immediately stops queue processing"]
pub struct WorkerHandle {
    shutdown: watch::Sender<bool>,
    task: Option<JoinHandle<()>>,
    errors: mpsc::UnboundedReceiver<QueueError>,
}

impl WorkerHandle {
    /// Waits for the next driver, handler, timeout, or state-transition error.
    pub async fn next_error(&mut self) -> Option<QueueError> {
        self.errors.recv().await
    }

    /// Returns an already reported worker error without waiting.
    pub fn try_next_error(&mut self) -> Option<QueueError> {
        self.errors.try_recv().ok()
    }

    /// Stops polling, cancels active handlers, and requeues interrupted jobs
    /// when the driver supports recoverable processing states.
    ///
    /// Only a handler that was actually cancelled is requeued. A handler that
    /// finishes before its cancellation takes effect is completed or failed
    /// from its own result, so a success is not run a second time.
    ///
    /// A claim that is already in flight is allowed to finish, so shutdown can
    /// wait for one `pop` call. A job claimed after shutdown was requested is
    /// requeued rather than dispatched. Dropping the handle instead aborts the
    /// worker immediately; a claim interrupted that way is returned only by
    /// stalled-lease recovery.
    ///
    /// # Errors
    /// Returns the first processing error reported before shutdown.
    pub async fn shutdown(mut self) -> Result<(), QueueError> {
        let _ = self.shutdown.send(true);
        let mut first_error = self.try_next_error();
        if let Some(task) = self.task.take()
            && let Err(error) = task.await
            && !error.is_cancelled()
            && first_error.is_none()
        {
            first_error = Some(QueueError::WorkerTask(error.to_string()));
        }
        first_error = first_error.or_else(|| self.try_next_error());
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn abort(&mut self) {
        let _ = self.shutdown.send(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.abort();
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg_attr(mutants, mutants::skip)]
async fn run_worker_loop(
    driver: Arc<Box<dyn QueueDriver>>,
    handlers: HashMap<String, Arc<JobHandler>>,
    poll_interval: Duration,
    max_concurrency: usize,
    job_timeout: Duration,
    stalled_after: Duration,
    mut shutdown: watch::Receiver<bool>,
    errors: mpsc::UnboundedSender<QueueError>,
) {
    match driver.recover_stalled(stalled_after).await {
        Ok(_) | Err(QueueError::Unsupported(_)) => {}
        Err(error) => {
            let _ = errors.send(error);
        }
    }

    let recovery_task = tokio::spawn(recovery_loop(
        Arc::clone(&driver),
        stalled_after,
        shutdown.clone(),
        errors.clone(),
    ));
    let mut recovery_task = AbortOnDrop(recovery_task);

    let mut jobs = JoinSet::new();
    loop {
        if shutdown_requested(&shutdown) {
            break;
        }
        if jobs.len() >= max_concurrency {
            tokio::select! {
                outcome = jobs.join_next() => report_outcome(outcome, &errors),
                _ = wait_for_shutdown(&mut shutdown) => break,
            }
            continue;
        }

        let popped = claim_next(&**driver, &mut jobs, &errors).await;
        if shutdown_requested(&shutdown) {
            release_claim_after_shutdown(popped, &**driver, &errors).await;
            break;
        }
        match popped {
            Ok(Some(job)) => {
                dispatch_job(
                    job,
                    &handlers,
                    Arc::clone(&driver),
                    job_timeout,
                    shutdown.clone(),
                    &mut jobs,
                    &errors,
                )
                .await
            }
            Ok(None) => {
                tokio::select! {
                    _ = tokio::time::sleep(poll_interval) => {}
                    outcome = jobs.join_next(), if !jobs.is_empty() => {
                        report_outcome(outcome, &errors);
                    }
                    _ = wait_for_shutdown(&mut shutdown) => break,
                }
            }
            Err(error) => {
                let _ = errors.send(error);
                tokio::select! {
                    _ = tokio::time::sleep(poll_interval) => {}
                    _ = wait_for_shutdown(&mut shutdown) => break,
                }
            }
        }
    }

    while let Some(outcome) = jobs.join_next().await {
        report_outcome(Some(outcome), &errors);
    }
    if let Err(error) = (&mut recovery_task.0).await
        && !error.is_cancelled()
    {
        let _ = errors.send(QueueError::WorkerTask(error.to_string()));
    }
}

/// Runs one `pop` to completion.
///
/// The built-in drivers commit a claim before the future resolves (an SQLite
/// `UPDATE ... RETURNING`, a Redis `EVAL`), so dropping the future would leave
/// the job in the processing state until stalled-lease recovery. The claim is
/// therefore never raced against shutdown; handlers that finish meanwhile are
/// still reported because `JoinSet::join_next` is cancel-safe.
async fn claim_next(
    driver: &dyn QueueDriver,
    jobs: &mut JoinSet<Result<(), QueueError>>,
    errors: &mpsc::UnboundedSender<QueueError>,
) -> Result<Option<QueuedJob>, QueueError> {
    let mut claim = driver.pop();
    loop {
        tokio::select! {
            popped = &mut claim => return popped,
            outcome = jobs.join_next(), if !jobs.is_empty() => report_outcome(outcome, errors),
        }
    }
}

/// Returns a job claimed while shutdown was requested to the pending state
/// instead of dispatching it.
async fn release_claim_after_shutdown(
    popped: Result<Option<QueuedJob>, QueueError>,
    driver: &dyn QueueDriver,
    errors: &mpsc::UnboundedSender<QueueError>,
) {
    match popped {
        Ok(Some(job)) => {
            if let Err(error) = driver
                .requeue(&job.id, "worker shutdown before dispatch")
                .await
            {
                let _ = errors.send(state_error(&job.id, "requeue_claim_after_shutdown", error));
            }
        }
        Ok(None) => {}
        Err(error) => {
            let _ = errors.send(error);
        }
    }
}

async fn recovery_loop(
    driver: Arc<Box<dyn QueueDriver>>,
    stale_after: Duration,
    mut shutdown: watch::Receiver<bool>,
    errors: mpsc::UnboundedSender<QueueError>,
) {
    let interval = stale_after
        .min(Duration::from_secs(60))
        .max(Duration::from_secs(1));
    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = wait_for_shutdown(&mut shutdown) => break,
        }
        match driver.recover_stalled(stale_after).await {
            Ok(_) => {}
            Err(QueueError::Unsupported(_)) => break,
            Err(error) => {
                let _ = errors.send(error);
            }
        }
    }
}

async fn dispatch_job(
    job: QueuedJob,
    handlers: &HashMap<String, Arc<JobHandler>>,
    driver: Arc<Box<dyn QueueDriver>>,
    timeout: Duration,
    shutdown: watch::Receiver<bool>,
    jobs: &mut JoinSet<Result<(), QueueError>>,
    errors: &mpsc::UnboundedSender<QueueError>,
) {
    let Some(handler) = handlers.get(&job.name).cloned() else {
        let missing = QueueError::HandlerNotFound(job.name.clone());
        if let Err(error) = driver.mark_failed(&job.id, &missing.to_string()).await {
            let _ = errors.send(state_error(&job.id, "mark_failed", error));
        } else {
            let _ = errors.send(missing);
        }
        return;
    };

    jobs.spawn(execution::execute_job(
        driver, handler, job, timeout, shutdown,
    ));
}

fn report_outcome(
    outcome: Option<Result<Result<(), QueueError>, tokio::task::JoinError>>,
    errors: &mpsc::UnboundedSender<QueueError>,
) {
    match outcome {
        Some(Ok(Err(error))) => {
            let _ = errors.send(error);
        }
        Some(Err(error)) => {
            let _ = errors.send(QueueError::WorkerTask(error.to_string()));
        }
        Some(Ok(Ok(()))) | None => {}
    }
}

fn state_error(job_id: &str, operation: &'static str, error: QueueError) -> QueueError {
    QueueError::StateTransition {
        job_id: job_id.to_string(),
        operation,
        message: error.to_string(),
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
