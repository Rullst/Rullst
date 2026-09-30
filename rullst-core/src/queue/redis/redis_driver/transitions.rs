//! Fenced state transitions for the Redis queue driver.

use super::super::scripts::{COMPLETE_SCRIPT, FAIL_SCRIPT, REQUEUE_SCRIPT};
use super::RedisDriver;
use crate::queue::QueueError;

impl RedisDriver {
    /// Runs a transition script and requires it to change exactly one job.
    pub(super) async fn transition(
        &self,
        script: &str,
        keys: &[&str],
        arguments: &[&str],
        job_id: &str,
        operation: &'static str,
        attempt: Option<u32>,
    ) -> Result<(), QueueError> {
        let mut connection = self.connection().await?;
        let mut command = redis::cmd("EVAL");
        command.arg(script).arg(keys.len());
        for key in keys {
            command.arg(key);
        }
        for argument in arguments {
            command.arg(argument);
        }
        let changed: i64 = command
            .query_async(&mut connection)
            .await
            .map_err(|error| QueueError::StateTransition {
                job_id: job_id.to_string(),
                operation,
                message: error.to_string(),
            })?;
        if changed == 1 {
            return Ok(());
        }
        let message = match attempt {
            Some(attempt) => format!(
                "expected one processing job at claim attempt {attempt}, affected {changed}; \
                 the lease may have been recovered and claimed again"
            ),
            None => format!("expected one processing job, affected {changed}"),
        };
        Err(QueueError::StateTransition {
            job_id: job_id.to_string(),
            operation,
            message,
        })
    }

    pub(super) async fn complete_claim(
        &self,
        job_id: &str,
        attempt: Option<u32>,
    ) -> Result<(), QueueError> {
        let expected = attempt
            .map(|attempt| attempt.to_string())
            .unwrap_or_default();
        self.transition(
            COMPLETE_SCRIPT,
            &[&self.processing_key, &self.processing_index_key],
            &[job_id, &expected],
            job_id,
            "mark_complete",
            attempt,
        )
        .await
    }

    pub(super) async fn fail_claim(
        &self,
        job_id: &str,
        attempt: Option<u32>,
        error: &str,
    ) -> Result<(), QueueError> {
        let expected = attempt
            .map(|attempt| attempt.to_string())
            .unwrap_or_default();
        let retention = self.failed_retention.to_string();
        self.transition(
            FAIL_SCRIPT,
            &[
                &self.processing_key,
                &self.processing_index_key,
                &self.failed_key,
                &self.failed_index_key,
            ],
            &[job_id, error, &expected, &retention],
            job_id,
            "mark_failed",
            attempt,
        )
        .await
    }

    pub(super) async fn requeue_claim(
        &self,
        job_id: &str,
        attempt: Option<u32>,
        reason: &str,
    ) -> Result<(), QueueError> {
        let expected = attempt
            .map(|attempt| attempt.to_string())
            .unwrap_or_default();
        self.transition(
            REQUEUE_SCRIPT,
            &[
                &self.processing_key,
                &self.processing_index_key,
                &self.queue_key,
            ],
            &[job_id, reason, &expected],
            job_id,
            "requeue",
            attempt,
        )
        .await
    }
}
