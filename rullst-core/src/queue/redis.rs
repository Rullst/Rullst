// src/queue/redis.rs — Redis-backed distributed queue driver.

#[cfg(feature = "queue-redis")]
pub mod redis_driver {
    //! Redis-backed queue driver. Requires the `queue-redis` feature.
    use super::super::{QueueDriver, QueueError, QueuedJob};
    use async_trait::async_trait;
    use serde_json::Value;

    /// Queue driver backed by Redis lists.
    ///
    /// Uses `RPUSH`/`LPOP` for FIFO ordering on the `rullst:queue:default` key.
    /// Ideal for high-throughput production workloads with distributed workers.
    pub struct RedisDriver {
        client: redis::Client,
        queue_key: String,
    }

    impl RedisDriver {
        /// Create a new Redis queue driver.
        pub fn new(redis_url: &str) -> Result<Self, QueueError> {
            let client = redis::Client::open(redis_url)
                .map_err(|e| QueueError::Driver(format!("Failed to connect to Redis: {}", e)))?;
            Ok(Self {
                client,
                queue_key: "rullst:queue:default".to_string(),
            })
        }
    }

    #[async_trait]
    impl QueueDriver for RedisDriver {
        #[cfg_attr(mutants, mutants::skip)]
        async fn push(&self, id: &str, job_name: &str, payload: &str) -> Result<(), QueueError> {
            let mut con = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| QueueError::Driver(format!("Redis connection failed: {}", e)))?;
            let job_data = serde_json::json!({
                "id": id,
                "name": job_name,
                "payload": payload,
                "attempts": 0
            });
            redis::cmd("RPUSH")
                .arg(&self.queue_key)
                .arg(job_data.to_string())
                .query_async::<i64>(&mut con)
                .await
                .map_err(|e| QueueError::Driver(format!("Failed to push to Redis: {}", e)))?;
            Ok(())
        }

        #[cfg_attr(mutants, mutants::skip)]
        async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
            let mut con = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| QueueError::Driver(format!("Redis connection failed: {}", e)))?;
            let result: Option<String> = redis::cmd("LPOP")
                .arg(&self.queue_key)
                .query_async(&mut con)
                .await
                .map_err(|e| QueueError::Driver(format!("Failed to pop from Redis: {}", e)))?;
            match result {
                Some(data) => {
                    let parsed: serde_json::Value = serde_json::from_str(&data)
                        .map_err(|e| QueueError::Serialization(e.to_string()))?;
                    let payload_str = parsed["payload"].as_str().unwrap_or("{}");
                    let payload = serde_json::from_str(payload_str).unwrap_or(Value::Null);
                    Ok(Some(QueuedJob {
                        id: parsed["id"].as_str().unwrap_or("").to_string(),
                        name: parsed["name"].as_str().unwrap_or("").to_string(),
                        payload,
                        attempts: parsed["attempts"].as_u64().unwrap_or(0) as u32 + 1,
                    }))
                }
                None => Ok(None),
            }
        }

        async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
            // In Redis list mode, the job is already removed by LPOP.
            Ok(())
        }

        async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
            // In basic Redis mode, failed jobs are simply logged.
            Ok(())
        }

        #[cfg_attr(mutants, mutants::skip)]
        async fn pending_count(&self) -> Result<u64, QueueError> {
            let mut con = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| QueueError::Driver(format!("Redis connection failed: {}", e)))?;
            let count: i64 = redis::cmd("LLEN")
                .arg(&self.queue_key)
                .query_async(&mut con)
                .await
                .map_err(|e| QueueError::Driver(format!("Failed to get queue length: {}", e)))?;
            Ok(count as u64)
        }
    }
}
