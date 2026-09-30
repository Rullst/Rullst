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

/// Identity of one claim. Transitions are fenced on its attempt number so a
/// stale worker cannot finish a job that was recovered and claimed again.
struct Claim {
    id: String,
    name: String,
    attempt: u32,
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
    let claim = Claim {
        id: job.id.clone(),
        name: job.name.clone(),
        attempt: job.attempts,
    };
    let job_id = claim.id.as_str();
    let mut execution = AbortOnDrop(tokio::spawn(async move { handler(job.payload).await }));
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);

    let interruption = tokio::select! {
        biased;
        outcome = &mut execution.0 => {
            return record_outcome(&**driver, &claim, outcome).await;
        }
        _ = &mut deadline => Interruption::Deadline,
        _ = wait_for_shutdown(&mut shutdown) => Interruption::Shutdown,
    };
    execution.0.abort();
    match (&mut execution.0).await {
        Err(error) if error.is_cancelled() => match interruption {
            Interruption::Deadline => {
                let failure = QueueError::JobTimedOut {
                    job_id: job_id.to_string(),
                    timeout_ms: duration_millis_u64(timeout),
                };
                driver
                    .mark_failed_attempt(job_id, claim.attempt, &failure.to_string())
                    .await
                    .map_err(|error| state_error(job_id, "mark_failed_after_timeout", error))?;
                Err(failure)
            }
            Interruption::Shutdown => driver
                .requeue_attempt(
                    job_id,
                    claim.attempt,
                    "worker shutdown interrupted execution",
                )
                .await
                .map_err(|error| state_error(job_id, "requeue_after_shutdown", error)),
        },
        finished => record_outcome(&**driver, &claim, finished).await,
    }
}

async fn record_outcome(
    driver: &dyn QueueDriver,
    claim: &Claim,
    outcome: HandlerOutcome,
) -> Result<(), QueueError> {
    let job_id = claim.id.as_str();
    let (failure, operation) = match outcome {
        Ok(Ok(())) => {
            return driver
                .mark_complete_attempt(job_id, claim.attempt)
                .await
                .map_err(|error| state_error(job_id, "mark_complete", error));
        }
        Ok(Err(error)) => (
            QueueError::JobFailed(format!("'{}' ({job_id}): {error}", claim.name)),
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
        .mark_failed_attempt(job_id, claim.attempt, &failure.to_string())
        .await
        .map_err(|error| state_error(job_id, operation, error))?;
    Err(failure)
}
