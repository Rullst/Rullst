//! Handler execution and outcome recording for the queue worker.

use super::{AbortOnDrop, JobHandler, duration_millis_u64, state_error, wait_for_shutdown};
use crate::queue::{QueueDriver, QueueError, QueuedJob};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

type HandlerOutcome =
    Result<Result<(), Box<dyn std::error::Error + Send + Sync>>, tokio::task::JoinError>;

enum Interruption {
    Deadline,
    Shutdown,
}

/// Runs one handler and records its outcome.
///
/// When the deadline or shutdown wins, the handler task is aborted and then
/// awaited. The recorded transition follows what the handler actually did:
/// only a task that was really cancelled is failed as timed out or requeued.
/// A handler that had already finished (or that blocked past the deadline and
/// then returned) is completed or failed from its own result, so a success is
/// never recorded as a timeout or run a second time after shutdown.
#[cfg_attr(mutants, mutants::skip)]
pub(super) async fn execute_job(
    driver: Arc<Box<dyn QueueDriver>>,
    handler: Arc<JobHandler>,
    job: QueuedJob,
    timeout: Duration,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), QueueError> {
    let job_id = job.id.clone();
    let job_name = job.name.clone();
    let mut execution = AbortOnDrop(tokio::spawn(async move { handler(job.payload).await }));
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);

    let interruption = tokio::select! {
        biased;
        outcome = &mut execution.0 => {
            return record_outcome(&**driver, &job_id, &job_name, outcome).await;
        }
        _ = &mut deadline => Interruption::Deadline,
        _ = wait_for_shutdown(&mut shutdown) => Interruption::Shutdown,
    };
    execution.0.abort();
    match (&mut execution.0).await {
        Err(error) if error.is_cancelled() => match interruption {
            Interruption::Deadline => {
                let failure = QueueError::JobTimedOut {
                    job_id: job_id.clone(),
                    timeout_ms: duration_millis_u64(timeout),
                };
                driver
                    .mark_failed(&job_id, &failure.to_string())
                    .await
                    .map_err(|error| state_error(&job_id, "mark_failed_after_timeout", error))?;
                Err(failure)
            }
            Interruption::Shutdown => driver
                .requeue(&job_id, "worker shutdown interrupted execution")
                .await
                .map_err(|error| state_error(&job_id, "requeue_after_shutdown", error)),
        },
        finished => record_outcome(&**driver, &job_id, &job_name, finished).await,
    }
}

async fn record_outcome(
    driver: &dyn QueueDriver,
    job_id: &str,
    job_name: &str,
    outcome: HandlerOutcome,
) -> Result<(), QueueError> {
    let (failure, operation) = match outcome {
        Ok(Ok(())) => {
            return driver
                .mark_complete(job_id)
                .await
                .map_err(|error| state_error(job_id, "mark_complete", error));
        }
        Ok(Err(error)) => (
            QueueError::JobFailed(format!("'{job_name}' ({job_id}): {error}")),
            "mark_failed",
        ),
        Err(error) if error.is_panic() => (
            QueueError::JobPanicked {
                job_id: job_id.to_string(),
            },
            "mark_failed_after_panic",
        ),
        Err(error) => (
            QueueError::WorkerTask(error.to_string()),
            "mark_failed_after_cancel",
        ),
    };
    driver
        .mark_failed(job_id, &failure.to_string())
        .await
        .map_err(|error| state_error(job_id, operation, error))?;
    Err(failure)
}
