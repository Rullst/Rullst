//! Transitions of claims the worker does not execute: jobs claimed during
//! shutdown and jobs without a registered handler.

use super::{MAX_UNHANDLED_CLAIM_ATTEMPT, UNHANDLED_JOB_RETRY_DELAY, state_error};
use crate::error_buffer::ErrorReporter;
use crate::queue::{QueueActivity, QueueDriver, QueueError, QueuedJob};

/// Returns a job claimed while shutdown was requested to the pending state
/// instead of dispatching it.
pub(super) async fn release_claim_after_shutdown(
    popped: Result<Option<QueuedJob>, QueueError>,
    driver: &dyn QueueDriver,
    errors: &ErrorReporter<QueueError>,
    activity: &QueueActivity,
) {
    match popped {
        Ok(Some(job)) => {
            match driver
                .requeue_attempt(&job.id, job.attempts, "worker shutdown before dispatch")
                .await
            {
                Ok(()) => activity.retried(),
                Err(error) => {
                    errors.report(state_error(&job.id, "requeue_claim_after_shutdown", error));
                }
            }
        }
        Ok(None) => {}
        Err(error) => {
            errors.report(error);
        }
    }
}

/// Returns a job this worker has no handler for to the queue, so a worker
/// that registered its name can claim it. The delay keeps this worker from
/// re-claiming the job in a hot loop; `HandlerNotFound` is still reported.
/// Drivers without delayed requeue keep the previous behaviour and fail it,
/// and a claim at [`MAX_UNHANDLED_CLAIM_ATTEMPT`] or later is failed too, so
/// a job whose name no worker handles reaches a terminal state.
pub(super) async fn hand_back_unhandled(
    job: &QueuedJob,
    driver: &dyn QueueDriver,
    errors: &ErrorReporter<QueueError>,
    activity: &QueueActivity,
) {
    let missing = QueueError::HandlerNotFound(job.name.clone());
    let reason = missing.to_string();
    let fail = |reason: String| async move {
        let failed = driver
            .mark_failed_attempt(&job.id, job.attempts, &reason)
            .await
            .map_err(|error| state_error(&job.id, "mark_failed", error));
        if failed.is_ok() {
            activity.failed();
        }
        failed
    };
    let transition = if job.attempts >= MAX_UNHANDLED_CLAIM_ATTEMPT {
        fail(format!(
            "{reason}; failed at claim attempt {} instead of being handed back again",
            job.attempts
        ))
        .await
    } else {
        match driver
            .requeue_attempt_after(&job.id, job.attempts, &reason, UNHANDLED_JOB_RETRY_DELAY)
            .await
        {
            Err(QueueError::Unsupported(_)) => fail(reason).await,
            Ok(()) => {
                activity.retried();
                Ok(())
            }
            Err(error) => Err(state_error(&job.id, "requeue_unhandled", error)),
        }
    };
    match transition {
        Ok(()) => errors.report(missing),
        Err(error) => errors.report(error),
    }
}

#[cfg(test)]
#[path = "transitions_tests.rs"]
mod tests;
