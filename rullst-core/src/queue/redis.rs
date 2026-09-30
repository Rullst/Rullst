//! Redis-backed distributed queue driver with recoverable processing leases.

mod scripts;

#[cfg(feature = "queue-redis")]
/// Redis queue driver implementation and its recoverable lease protocol.
pub mod redis_driver {
    use super::super::{
        QueueDriver, QueueError, QueuedJob, QueuedJobDetail, unix_timestamp_millis_ceil,
    };
    use super::scripts::{
        CLAIM_SCRIPT, PENDING_COUNT_SCRIPT, RECOVER_SCRIPT, REJECT_SCRIPT, REQUEUE_AFTER_SCRIPT,
    };
    use crate::redis_connection::{RedisConnection, SharedRedisConnection};
    use async_trait::async_trait;
    use serde::Deserialize;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    mod inspection;
    mod transitions;

    /// Failed jobs and dead letters each retained by default (newest kept).
    pub const DEFAULT_FAILURE_RETENTION: usize = 10_000;
    /// Upper bound accepted by [`RedisDriver::try_with_failure_retention`].
    pub const MAX_FAILURE_RETENTION: usize = 100_000;

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
    ///
    /// Failed jobs (with their payloads) and dead letters are retained up to
    /// [`DEFAULT_FAILURE_RETENTION`] each; the oldest are evicted atomically
    /// when a new one is recorded. See [`Self::try_with_failure_retention`].
    pub struct RedisDriver {
        shared: SharedRedisConnection,
        queue_key: String,
        processing_key: String,
        processing_index_key: String,
        scheduled_key: String,
        failed_key: String,
        failed_index_key: String,
        dead_letter_key: String,
        failed_retention: usize,
        dead_letter_retention: usize,
        max_stalled_leases: u32,
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
            self.failed_index_key = format!("{queue_key}:failed:index");
            self.dead_letter_key = format!("{queue_key}:dead-letter");
            Ok(self)
        }

        /// Sets how many failed jobs and dead letters are retained, each
        /// between 1 and [`MAX_FAILURE_RETENTION`].
        ///
        /// Recording a failure beyond the limit evicts the oldest failed job
        /// (by failure time) or dead letter in the same atomic script. Failed
        /// jobs recorded before this retention index existed are not counted
        /// or evicted.
        pub fn try_with_failure_retention(
            mut self,
            failed_jobs: usize,
            dead_letters: usize,
        ) -> Result<Self, QueueError> {
            for limit in [failed_jobs, dead_letters] {
                if !(1..=MAX_FAILURE_RETENTION).contains(&limit) {
                    return Err(QueueError::InvalidConfiguration(format!(
                        "Redis failure retention must be between 1 and {MAX_FAILURE_RETENTION}"
                    )));
                }
            }
            self.failed_retention = failed_jobs;
            self.dead_letter_retention = dead_letters;
            Ok(self)
        }

        fn from_client(client: redis::Client, queue_key: String) -> Self {
            Self {
                processing_key: format!("{queue_key}:processing"),
                processing_index_key: format!("{queue_key}:processing:index"),
                scheduled_key: format!("{queue_key}:scheduled"),
                failed_key: format!("{queue_key}:failed"),
                failed_index_key: format!("{queue_key}:failed:index"),
                dead_letter_key: format!("{queue_key}:dead-letter"),
                queue_key,
                shared: SharedRedisConnection::new(client),
                failed_retention: DEFAULT_FAILURE_RETENTION,
                dead_letter_retention: DEFAULT_FAILURE_RETENTION,
                max_stalled_leases: super::super::DEFAULT_MAX_STALLED_LEASES,
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
                .arg(self.dead_letter_retention)
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
                .arg(self.dead_letter_retention)
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

        async fn requeue_attempt_after(
            &self,
            job_id: &str,
            attempt: u32,
            _reason: &str,
            delay: Duration,
        ) -> Result<(), QueueError> {
            let delay_ms = u64::try_from(delay.as_millis()).map_err(|_| {
                QueueError::InvalidConfiguration(
                    "requeue delay exceeds the Redis score range".to_string(),
                )
            })?;
            let expected = attempt.to_string();
            self.transition(
                REQUEUE_AFTER_SCRIPT,
                &[
                    &self.processing_key,
                    &self.processing_index_key,
                    &self.scheduled_key,
                ],
                &[job_id, &delay_ms.to_string(), &expected],
                job_id,
                "requeue_after",
                Some(attempt),
            )
            .await
        }

        async fn recover_stalled(&self, stale_after: Duration) -> Result<u64, QueueError> {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| QueueError::Driver(format!("System clock error: {error}")))?;
            let cutoff = now.as_millis().saturating_sub(stale_after.as_millis());
            let mut connection = self.connection().await?;
            redis::cmd("EVAL")
                .arg(RECOVER_SCRIPT)
                .arg(6)
                .arg(&self.processing_key)
                .arg(&self.processing_index_key)
                .arg(&self.queue_key)
                .arg(&self.dead_letter_key)
                .arg(&self.failed_key)
                .arg(&self.failed_index_key)
                .arg(cutoff.to_string())
                .arg(self.dead_letter_retention)
                .arg(self.max_stalled_leases)
                .arg(format!(
                    "lease stalled {} times without finishing; failed instead of requeued",
                    self.max_stalled_leases
                ))
                .arg(self.failed_retention)
                .query_async::<u64>(&mut connection)
                .await
                .map_err(|error| {
                    QueueError::Driver(format!("Failed to recover Redis jobs: {error}"))
                })
        }

        /// Lists at most `limit` (capped at 1,000) jobs: failed jobs and dead
        /// letters (newest first), then processing, pending and scheduled
        /// ones. Failures recorded before the failure index existed are not
        /// listed.
        async fn list_all_jobs(&self, limit: u32) -> Result<Vec<QueuedJobDetail>, QueueError> {
            self.list_jobs(limit).await
        }

        /// Moves a failed job to the tail of the pending list, keeping its
        /// attempt counter (SQLite resets it).
        async fn retry_failed_job(&self, job_id: &str) -> Result<(), QueueError> {
            self.retry_failed(job_id).await
        }

        async fn purge_completed_jobs(&self) -> Result<(), QueueError> {
            self.purge_failures().await
        }

        /// Deletes every failed job and every dead letter.
        async fn purge_failed_jobs(&self) -> Result<(), QueueError> {
            self.purge_failures().await
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
    mod tests;
}
