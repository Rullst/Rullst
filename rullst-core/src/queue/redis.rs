//! Redis-backed distributed queue driver with recoverable processing leases.

#[cfg(feature = "queue-redis")]
/// Redis queue driver implementation and its recoverable lease protocol.
pub mod redis_driver {
    use super::super::{QueueDriver, QueueError, QueuedJob};
    use async_trait::async_trait;
    use serde::Deserialize;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const CLAIM_SCRIPT: &str = r#"
local raw = redis.call('LPOP', KEYS[1])
if not raw then return nil end
local ok, envelope = pcall(cjson.decode, raw)
if ok and type(envelope) == 'table' and type(envelope.attempts) == 'number' then
    envelope.attempts = envelope.attempts + 1
    raw = cjson.encode(envelope)
end
local now = redis.call('TIME')
local claimed_at_ms = (tonumber(now[1]) * 1000) + math.floor(tonumber(now[2]) / 1000)
if ok and type(envelope) == 'table' and type(envelope.id) == 'string' and envelope.id ~= '' then
    if redis.call('HEXISTS', KEYS[3], envelope.id) == 1 then
        redis.call('RPUSH', KEYS[4], cjson.encode({ raw = raw, error = 'duplicate processing job id' }))
        return redis.error_reply('duplicate processing job id')
    end
    redis.call('HSET', KEYS[3], envelope.id, raw)
end
redis.call('ZADD', KEYS[2], claimed_at_ms, raw)
return raw
"#;

    const REJECT_SCRIPT: &str = r#"
redis.call('ZREM', KEYS[1], ARGV[1])
local ok, envelope = pcall(cjson.decode, ARGV[1])
if ok and type(envelope) == 'table' and type(envelope.id) == 'string' then
    redis.call('HDEL', KEYS[2], envelope.id)
end
redis.call('RPUSH', KEYS[3], cjson.encode({ raw = ARGV[1], error = ARGV[2] }))
return 1
"#;

    const COMPLETE_SCRIPT: &str = r#"
local raw = redis.call('HGET', KEYS[2], ARGV[1])
if not raw then return 0 end
redis.call('ZREM', KEYS[1], raw)
redis.call('HDEL', KEYS[2], ARGV[1])
return 1
"#;

    const FAIL_SCRIPT: &str = r#"
local raw = redis.call('HGET', KEYS[2], ARGV[1])
if not raw then return 0 end
redis.call('ZREM', KEYS[1], raw)
redis.call('HDEL', KEYS[2], ARGV[1])
redis.call('HSET', KEYS[3], ARGV[1], cjson.encode({ raw = raw, error = ARGV[2] }))
return 1
"#;

    const REQUEUE_SCRIPT: &str = r#"
local raw = redis.call('HGET', KEYS[2], ARGV[1])
if not raw then return 0 end
redis.call('ZREM', KEYS[1], raw)
redis.call('HDEL', KEYS[2], ARGV[1])
redis.call('LPUSH', KEYS[3], raw)
return 1
"#;

    const RECOVER_SCRIPT: &str = r#"
local stalled = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1])
local recovered = 0
for _, raw in ipairs(stalled) do
    redis.call('ZREM', KEYS[1], raw)
    local ok, envelope = pcall(cjson.decode, raw)
    if ok and type(envelope) == 'table' and type(envelope.id) == 'string' then
        redis.call('HDEL', KEYS[2], envelope.id)
        redis.call('RPUSH', KEYS[3], raw)
        recovered = recovered + 1
    else
        redis.call('RPUSH', KEYS[4], cjson.encode({ raw = raw, error = 'invalid stalled job envelope' }))
    end
end
return recovered
"#;

    #[derive(Deserialize)]
    struct RedisJobEnvelope {
        id: String,
        name: String,
        payload: String,
        attempts: u64,
    }

    /// Redis queue using a pending list, a processing lease set, and failure
    /// hashes. Lua scripts make each state transition atomic.
    pub struct RedisDriver {
        client: redis::Client,
        queue_key: String,
        processing_key: String,
        processing_index_key: String,
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
            let queue_key = "rullst:queue:default".to_string();
            Ok(Self {
                processing_key: format!("{queue_key}:processing"),
                processing_index_key: format!("{queue_key}:processing:index"),
                failed_key: format!("{queue_key}:failed"),
                dead_letter_key: format!("{queue_key}:dead-letter"),
                queue_key,
                client,
            })
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

        async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, QueueError> {
            self.client
                .get_multiplexed_async_connection()
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
                Ok(())
            } else {
                Err(QueueError::StateTransition {
                    job_id: job_id.to_string(),
                    operation,
                    message: format!("expected one processing job, affected {changed}"),
                })
            }
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

        async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
            let mut connection = self.connection().await?;
            let raw: Option<String> = redis::cmd("EVAL")
                .arg(CLAIM_SCRIPT)
                .arg(4)
                .arg(&self.queue_key)
                .arg(&self.processing_key)
                .arg(&self.processing_index_key)
                .arg(&self.dead_letter_key)
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
            self.transition(
                COMPLETE_SCRIPT,
                &[&self.processing_key, &self.processing_index_key],
                &[job_id],
                job_id,
                "mark_complete",
            )
            .await
        }

        async fn mark_failed(&self, job_id: &str, error: &str) -> Result<(), QueueError> {
            self.transition(
                FAIL_SCRIPT,
                &[
                    &self.processing_key,
                    &self.processing_index_key,
                    &self.failed_key,
                ],
                &[job_id, error],
                job_id,
                "mark_failed",
            )
            .await
        }

        async fn requeue(&self, job_id: &str, reason: &str) -> Result<(), QueueError> {
            self.transition(
                REQUEUE_SCRIPT,
                &[
                    &self.processing_key,
                    &self.processing_index_key,
                    &self.queue_key,
                ],
                &[job_id, reason],
                job_id,
                "requeue",
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
            let count: i64 = redis::cmd("LLEN")
                .arg(&self.queue_key)
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
    }
}
