//! Redis-backed distributed queue driver with recoverable processing leases.

mod scripts;

#[cfg(feature = "queue-redis")]
/// Redis queue driver implementation and its recoverable lease protocol.
pub mod redis_driver {
    use super::super::{QueueDriver, QueueError, QueuedJob, unix_timestamp_millis_ceil};
    use super::scripts::{
        CLAIM_SCRIPT, COMPLETE_SCRIPT, FAIL_SCRIPT, PENDING_COUNT_SCRIPT, RECOVER_SCRIPT,
        REJECT_SCRIPT, REQUEUE_SCRIPT,
    };
    use crate::redis_connection::{RedisConnection, SharedRedisConnection};
    use async_trait::async_trait;
    use serde::Deserialize;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[derive(Deserialize)]
    struct RedisJobEnvelope {
        id: String,
        name: String,
        payload: String,
        attempts: u64,
    }

    /// Redis queue using a pending list, a processing lease set, and failure
    /// hashes. Lua scripts make each state transition atomic.
    ///
    /// Operations share one lazily opened multiplexed connection. If it breaks,
    /// the failing operation returns its error and the next one reconnects.
    pub struct RedisDriver {
        shared: SharedRedisConnection,
        queue_key: String,
        processing_key: String,
        processing_index_key: String,
        scheduled_key: String,
        failed_key: String,
        dead_letter_key: String,
    }

    impl RedisDriver {
        /// Creates a Redis driver without opening a network connection.
        pub fn new(redis_url: impl Into<String>) -> Result<Self, QueueError> {
            let redis_url = redis_url.into();
            let client = redis::Client::open(redis_url).map_err(|error| {
                QueueError::Driver(format!("Failed to connect to Redis: {error}"))
            })?;
            Ok(Self::from_client(
                client,
                "rullst:queue:default".to_string(),
            ))
        }

        /// Selects an isolated queue namespace before the driver is shared with workers.
        pub fn try_with_namespace(
            mut self,
            namespace: impl Into<String>,
        ) -> Result<Self, QueueError> {
            let namespace = namespace.into();
            if namespace.is_empty()
                || namespace.len() > 64
                || !namespace
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            {
                return Err(QueueError::InvalidConfiguration(
                    "Redis queue namespace must be 1-64 ASCII letters, digits, '-' or '_'"
                        .to_string(),
                ));
            }
            let queue_key = format!("rullst:queue:{namespace}");
            self.queue_key = queue_key.clone();
            self.processing_key = format!("{queue_key}:processing");
            self.processing_index_key = format!("{queue_key}:processing:index");
            self.scheduled_key = format!("{queue_key}:scheduled");
            self.failed_key = format!("{queue_key}:failed");
            self.dead_letter_key = format!("{queue_key}:dead-letter");
            Ok(self)
        }

        fn from_client(client: redis::Client, queue_key: String) -> Self {
            Self {
                processing_key: format!("{queue_key}:processing"),
                processing_index_key: format!("{queue_key}:processing:index"),
                scheduled_key: format!("{queue_key}:scheduled"),
                failed_key: format!("{queue_key}:failed"),
                dead_letter_key: format!("{queue_key}:dead-letter"),
                queue_key,
                shared: SharedRedisConnection::new(client),
            }
        }

        async fn reject_claimed(&self, raw: &str, reason: &str) -> Result<(), QueueError> {
            let mut connection = self.connection().await?;
            redis::cmd("EVAL")
                .arg(REJECT_SCRIPT)
                .arg(3)
                .arg(&self.processing_key)
                .arg(&self.processing_index_key)
                .arg(&self.dead_letter_key)
                .arg(raw)
                .arg(reason)
                .query_async::<i64>(&mut connection)
                .await
                .map_err(|error| {
                    QueueError::Driver(format!("Failed to reject Redis job: {error}"))
                })?;
            Ok(())
        }

        async fn connection(&self) -> Result<RedisConnection<'_>, QueueError> {
            self.shared
                .connection()
                .await
                .map_err(|error| QueueError::Driver(format!("Redis connection failed: {error}")))
        }

        async fn transition(
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

        async fn complete_claim(
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

        async fn fail_claim(
            &self,
            job_id: &str,
            attempt: Option<u32>,
            error: &str,
        ) -> Result<(), QueueError> {
            let expected = attempt
                .map(|attempt| attempt.to_string())
                .unwrap_or_default();
            self.transition(
                FAIL_SCRIPT,
                &[
                    &self.processing_key,
                    &self.processing_index_key,
                    &self.failed_key,
                ],
                &[job_id, error, &expected],
                job_id,
                "mark_failed",
                attempt,
            )
            .await
        }

        async fn requeue_claim(
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

    #[async_trait]
    impl QueueDriver for RedisDriver {
        async fn push(&self, id: &str, job_name: &str, payload: &str) -> Result<(), QueueError> {
            let mut connection = self.connection().await?;
            let job_data = serde_json::json!({
                "id": id,
                "name": job_name,
                "payload": payload,
                "attempts": 0
            });
            redis::cmd("RPUSH")
                .arg(&self.queue_key)
                .arg(job_data.to_string())
                .query_async::<i64>(&mut connection)
                .await
                .map_err(|error| QueueError::Driver(format!("Failed to push to Redis: {error}")))?;
            Ok(())
        }

        async fn push_at(
            &self,
            id: &str,
            job_name: &str,
            payload: &str,
            available_at: SystemTime,
        ) -> Result<(), QueueError> {
            let available_at_ms = unix_timestamp_millis_ceil(available_at)?;
            let mut connection = self.connection().await?;
            let job_data = serde_json::json!({
                "id": id,
                "name": job_name,
                "payload": payload,
                "attempts": 0
            });
            redis::cmd("ZADD")
                .arg(&self.scheduled_key)
                .arg(available_at_ms)
                .arg(job_data.to_string())
                .query_async::<i64>(&mut connection)
                .await
                .map_err(|error| {
                    QueueError::Driver(format!("Failed to schedule Redis job: {error}"))
                })?;
            Ok(())
        }

        async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
            let mut connection = self.connection().await?;
            let raw: Option<String> = redis::cmd("EVAL")
                .arg(CLAIM_SCRIPT)
                .arg(5)
                .arg(&self.queue_key)
                .arg(&self.processing_key)
                .arg(&self.processing_index_key)
                .arg(&self.dead_letter_key)
                .arg(&self.scheduled_key)
                .query_async(&mut connection)
                .await
                .map_err(|error| {
                    QueueError::Driver(format!("Failed to claim Redis job: {error}"))
                })?;
            let Some(raw) = raw else {
                return Ok(None);
            };

            let job = match parse_claimed_job(&raw) {
                Ok(job) => job,
                Err(error) => {
                    self.reject_claimed(&raw, &error.to_string())
                        .await
                        .map_err(|transition| QueueError::StateTransition {
                            job_id: "unknown-redis-job".to_string(),
                            operation: "reject_invalid_payload",
                            message: format!("{error}; {transition}"),
                        })?;
                    return Err(error);
                }
            };

            Ok(Some(job))
        }

        async fn mark_complete(&self, job_id: &str) -> Result<(), QueueError> {
            self.complete_claim(job_id, None).await
        }

        async fn mark_failed(&self, job_id: &str, error: &str) -> Result<(), QueueError> {
            self.fail_claim(job_id, None, error).await
        }

        async fn requeue(&self, job_id: &str, reason: &str) -> Result<(), QueueError> {
            self.requeue_claim(job_id, None, reason).await
        }

        async fn mark_complete_attempt(
            &self,
            job_id: &str,
            attempt: u32,
        ) -> Result<(), QueueError> {
            self.complete_claim(job_id, Some(attempt)).await
        }

        async fn mark_failed_attempt(
            &self,
            job_id: &str,
            attempt: u32,
            error: &str,
        ) -> Result<(), QueueError> {
            self.fail_claim(job_id, Some(attempt), error).await
        }

        async fn requeue_attempt(
            &self,
            job_id: &str,
            attempt: u32,
            reason: &str,
        ) -> Result<(), QueueError> {
            self.requeue_claim(job_id, Some(attempt), reason).await
        }

        async fn recover_stalled(&self, stale_after: Duration) -> Result<u64, QueueError> {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| QueueError::Driver(format!("System clock error: {error}")))?;
            let cutoff = now.as_millis().saturating_sub(stale_after.as_millis());
            let mut connection = self.connection().await?;
            redis::cmd("EVAL")
                .arg(RECOVER_SCRIPT)
                .arg(4)
                .arg(&self.processing_key)
                .arg(&self.processing_index_key)
                .arg(&self.queue_key)
                .arg(&self.dead_letter_key)
                .arg(cutoff.to_string())
                .query_async::<u64>(&mut connection)
                .await
                .map_err(|error| {
                    QueueError::Driver(format!("Failed to recover Redis jobs: {error}"))
                })
        }

        async fn pending_count(&self) -> Result<u64, QueueError> {
            let mut connection = self.connection().await?;
            let count: i64 = redis::cmd("EVAL")
                .arg(PENDING_COUNT_SCRIPT)
                .arg(2)
                .arg(&self.queue_key)
                .arg(&self.scheduled_key)
                .query_async(&mut connection)
                .await
                .map_err(|error| {
                    QueueError::Driver(format!("Failed to get queue length: {error}"))
                })?;
            u64::try_from(count)
                .map_err(|_| QueueError::Driver(format!("Redis returned negative LLEN: {count}")))
        }
    }

    fn parse_claimed_job(raw: &str) -> Result<QueuedJob, QueueError> {
        let envelope: RedisJobEnvelope = serde_json::from_str(raw).map_err(|error| {
            QueueError::Serialization(format!("invalid Redis job envelope: {error}"))
        })?;
        if envelope.id.is_empty() || envelope.name.is_empty() {
            return Err(QueueError::Serialization(
                "Redis job id and name must be non-empty".to_string(),
            ));
        }
        let payload = serde_json::from_str(&envelope.payload).map_err(|error| {
            QueueError::Serialization(format!(
                "Redis job '{}' contains invalid JSON: {error}",
                envelope.id
            ))
        })?;
        let attempts = u32::try_from(envelope.attempts).map_err(|_| {
            QueueError::Serialization(format!(
                "Redis job '{}' attempts counter overflowed",
                envelope.id
            ))
        })?;
        Ok(QueuedJob {
            id: envelope.id,
            name: envelope.name,
            payload,
            attempts,
        })
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used, clippy::expect_used)]
    mod tests {
        use super::*;

        #[test]
        fn claimed_envelope_requires_valid_json_payload() {
            let result =
                parse_claimed_job(r#"{"id":"job-1","name":"test","payload":"{bad","attempts":1}"#);
            assert!(matches!(result, Err(QueueError::Serialization(_))));
        }

        #[test]
        fn claimed_envelope_is_strict_and_lossless() {
            let job = parse_claimed_job(
                r#"{"id":"job-1","name":"test","payload":"{\"ok\":true}","attempts":2}"#,
            )
            .unwrap();
            assert_eq!(job.id, "job-1");
            assert_eq!(job.payload["ok"], true);
            assert_eq!(job.attempts, 2);
        }

        #[test]
        fn namespace_is_bounded_and_syntax_checked() {
            let valid = RedisDriver::new("redis://127.0.0.1/")
                .unwrap()
                .try_with_namespace("tenant_42-prod");
            assert!(valid.is_ok());

            for invalid in ["", "../shared", "contains space"] {
                let result = RedisDriver::new("redis://127.0.0.1/")
                    .unwrap()
                    .try_with_namespace(invalid);
                assert!(matches!(result, Err(QueueError::InvalidConfiguration(_))));
            }
        }
    }
}
