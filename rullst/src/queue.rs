//! # Rullst Queue System (`rullst::queue`)
//!
//! Provides a unified API for dispatching and processing background jobs.
//!
//! ## Drivers
//! - **SQLite** (default): Uses an auto-created `rullst_jobs` table. Zero config.
//! - **Redis** (optional): Requires the `queue-redis` feature flag.
//!
//! ## Quick Start
//! ```rust,ignore
//! use rullst::queue::{Queue, Worker};
//!
//! // Dispatch a job
//! let queue = Queue::sqlite("sqlite://rullst.db").await?;
//! queue.dispatch("send_email", serde_json::json!({"to": "user@example.com"})).await?;
//!
//! // Process jobs in the background
//! let mut worker = Worker::new(queue);
//! worker.register("send_email", |payload| async move {
//!     println!("Sending email to: {}", payload["to"]);
//!     Ok(())
//! });
//! worker.run().await;
//! ```

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

// ─── Error Types ────────────────────────────────────────────────────────────

/// Errors that can occur during queue operations.
#[derive(Debug)]
pub enum QueueError {
    /// The underlying database or connection failed.
    Driver(String),
    /// Serialization/deserialization of job payloads failed.
    Serialization(String),
    /// A job handler was not found for the given job name.
    HandlerNotFound(String),
    /// The job execution itself failed.
    JobFailed(String),
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::Driver(msg) => write!(f, "Queue driver error: {}", msg),
            QueueError::Serialization(msg) => write!(f, "Queue serialization error: {}", msg),
            QueueError::HandlerNotFound(name) => {
                write!(f, "No handler registered for job: {}", name)
            }
            QueueError::JobFailed(msg) => write!(f, "Job execution failed: {}", msg),
        }
    }
}

impl std::error::Error for QueueError {}

// ─── Queued Job ─────────────────────────────────────────────────────────────

/// A job that has been placed on the queue and is ready for processing.
#[derive(Debug, Clone)]
pub struct QueuedJob {
    /// Unique identifier for this job instance.
    pub id: String,
    /// The job type name (used to look up the handler).
    pub name: String,
    /// The JSON payload associated with this job.
    pub payload: Value,
    /// Number of times this job has been attempted.
    pub attempts: u32,
}

/// Detailed job information, used for dashboard monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueuedJobDetail {
    pub id: String,
    pub name: String,
    pub payload: String,
    pub status: String,
    pub error: Option<String>,
    pub attempts: i32,
    pub created_at: String,
    pub updated_at: String,
}

// ─── Queue Driver Trait ─────────────────────────────────────────────────────

/// Abstraction over queue storage backends.
///
/// Implement this trait to add support for new queue backends.
/// The framework ships with `SqliteDriver` and (optionally) `RedisDriver`.
#[async_trait]
pub trait QueueDriver: Send + Sync {
    /// Push a new job onto the queue.
    async fn push(&self, id: &str, job_name: &str, payload: &str) -> Result<(), QueueError>;
    /// Pop the next available job from the queue (FIFO).
    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError>;
    /// Mark a job as successfully completed (removes from queue).
    async fn mark_complete(&self, job_id: &str) -> Result<(), QueueError>;
    /// Mark a job as failed, recording the error message.
    async fn mark_failed(&self, job_id: &str, error: &str) -> Result<(), QueueError>;
    /// Return the count of pending jobs.
    async fn pending_count(&self) -> Result<u64, QueueError>;
    /// List all recent jobs for monitoring
    async fn list_all_jobs(&self, _limit: u32) -> Result<Vec<QueuedJobDetail>, QueueError> {
        Ok(vec![])
    }
    /// Retry a failed job
    async fn retry_failed_job(&self, _job_id: &str) -> Result<(), QueueError> {
        Ok(())
    }
    /// Purge completed or failed jobs
    async fn purge_completed_jobs(&self) -> Result<(), QueueError> {
        Ok(())
    }
}

// ─── SQLite Driver ──────────────────────────────────────────────────────────

/// Queue driver backed by a SQLite database.
///
/// Uses an auto-created `rullst_jobs` table. Perfect for local development
/// and small-to-medium production workloads. Zero external dependencies.
pub struct SqliteDriver {
    pool: sqlx::SqlitePool,
}

impl SqliteDriver {
    /// Create a new SQLite queue driver. Automatically creates the `rullst_jobs`
    /// table if it doesn't exist.
    pub async fn new(database_url: &str) -> Result<Self, QueueError> {
        let pool = sqlx::SqlitePool::connect(database_url)
            .await
            .map_err(|e| QueueError::Driver(format!("Failed to connect to SQLite: {}", e)))?;

        // Auto-create the jobs table
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS rullst_jobs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                payload TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                error TEXT,
                attempts INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )"#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Driver(format!("Failed to create rullst_jobs table: {}", e)))?;

        Ok(Self { pool })
    }

    pub fn get_pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }

    #[allow(clippy::type_complexity)]
    pub async fn list_all_jobs(&self, limit: u32) -> Result<Vec<QueuedJobDetail>, QueueError> {
        let rows: Vec<(String, String, String, String, Option<String>, i32, String, String)> = sqlx::query_as(
            "SELECT id, name, payload, status, error, attempts, created_at, updated_at FROM rullst_jobs ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QueueError::Driver(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(id, name, payload, status, error, attempts, created_at, updated_at)| {
                    QueuedJobDetail {
                        id,
                        name,
                        payload,
                        status,
                        error,
                        attempts,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect())
    }

    pub async fn retry_failed_job(&self, job_id: &str) -> Result<(), QueueError> {
        sqlx::query("UPDATE rullst_jobs SET status = 'pending', attempts = 0, error = NULL, updated_at = datetime('now') WHERE id = ? AND status = 'failed'")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::Driver(e.to_string()))?;
        Ok(())
    }

    pub async fn purge_completed_jobs(&self) -> Result<(), QueueError> {
        sqlx::query("DELETE FROM rullst_jobs WHERE status = 'failed'")
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::Driver(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl QueueDriver for SqliteDriver {
    async fn push(&self, id: &str, job_name: &str, payload: &str) -> Result<(), QueueError> {
        sqlx::query("INSERT INTO rullst_jobs (id, name, payload) VALUES (?, ?, ?)")
            .bind(id)
            .bind(job_name)
            .bind(payload)
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::Driver(format!("Failed to push job: {}", e)))?;
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        // Atomically select and mark the oldest pending job as 'processing'
        let row: Option<(String, String, String, i32)> = sqlx::query_as(
            r#"UPDATE rullst_jobs
               SET status = 'processing', attempts = attempts + 1, updated_at = datetime('now')
               WHERE id = (
                   SELECT id FROM rullst_jobs WHERE status = 'pending' ORDER BY created_at ASC LIMIT 1
               )
               RETURNING id, name, payload, attempts"#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::Driver(format!("Failed to pop job: {}", e)))?;

        Ok(row.map(|(id, name, payload_str, attempts)| {
            let payload = serde_json::from_str(&payload_str).unwrap_or(Value::Null);
            QueuedJob {
                id,
                name,
                payload,
                attempts: attempts as u32,
            }
        }))
    }

    async fn mark_complete(&self, job_id: &str) -> Result<(), QueueError> {
        sqlx::query("DELETE FROM rullst_jobs WHERE id = ?")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::Driver(format!("Failed to mark job complete: {}", e)))?;
        Ok(())
    }

    async fn mark_failed(&self, job_id: &str, error: &str) -> Result<(), QueueError> {
        sqlx::query(
            "UPDATE rullst_jobs SET status = 'failed', error = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(error)
        .bind(job_id)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Driver(format!("Failed to mark job failed: {}", e)))?;
        Ok(())
    }

    async fn pending_count(&self) -> Result<u64, QueueError> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM rullst_jobs WHERE status = 'pending'")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| QueueError::Driver(format!("Failed to count pending jobs: {}", e)))?;
        Ok(count as u64)
    }

    async fn list_all_jobs(&self, limit: u32) -> Result<Vec<QueuedJobDetail>, QueueError> {
        self.list_all_jobs(limit).await
    }

    async fn retry_failed_job(&self, job_id: &str) -> Result<(), QueueError> {
        self.retry_failed_job(job_id).await
    }

    async fn purge_completed_jobs(&self) -> Result<(), QueueError> {
        self.purge_completed_jobs().await
    }
}

// ─── Redis Driver (behind feature flag) ─────────────────────────────────────

#[cfg(feature = "queue-redis")]
pub mod redis_driver {
    //! Redis-backed queue driver. Requires the `queue-redis` feature.
    use super::*;

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
            // For advanced use cases, a dead-letter or processing set could be used.
            Ok(())
        }

        async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
            // In basic Redis mode, failed jobs are simply logged.
            // Future: push to a dead-letter queue (rullst:queue:failed).
            Ok(())
        }

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

// ─── Queue Facade ───────────────────────────────────────────────────────────

/// The main queue facade for dispatching background jobs.
///
/// Provides a driver-agnostic API. Create with `Queue::sqlite()` or `Queue::redis()`.
pub struct Queue {
    driver: Arc<Box<dyn QueueDriver>>,
}

impl Queue {
    /// Create a queue backed by SQLite. The `rullst_jobs` table is auto-created.
    ///
    /// # Example
    /// ```rust,ignore
    /// let queue = Queue::sqlite("sqlite://rullst.db").await?;
    /// ```
    pub async fn sqlite(database_url: &str) -> Result<Self, QueueError> {
        let driver = SqliteDriver::new(database_url).await?;
        Ok(Self {
            driver: Arc::new(Box::new(driver)),
        })
    }

    /// Create a queue backed by Redis. Requires the `queue-redis` feature.
    ///
    /// # Example
    /// ```rust,ignore
    /// let queue = Queue::redis("redis://127.0.0.1:6379")?;
    /// ```
    #[cfg(feature = "queue-redis")]
    pub fn redis(redis_url: &str) -> Result<Self, QueueError> {
        let driver = redis_driver::RedisDriver::new(redis_url)?;
        Ok(Self {
            driver: Arc::new(Box::new(driver)),
        })
    }

    /// Create a queue from any custom driver implementing `QueueDriver`.
    pub fn custom(driver: Box<dyn QueueDriver>) -> Self {
        Self {
            driver: Arc::new(driver),
        }
    }

    /// Dispatch a named job with a JSON payload onto the queue.
    ///
    /// # Example
    /// ```rust,ignore
    /// queue.dispatch("send_welcome_email", serde_json::json!({
    ///     "user_id": 42,
    ///     "email": "user@example.com"
    /// })).await?;
    /// ```
    pub async fn dispatch(&self, job_name: &str, payload: Value) -> Result<String, QueueError> {
        let id = Uuid::new_v4().to_string();
        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| QueueError::Serialization(e.to_string()))?;
        self.driver.push(&id, job_name, &payload_str).await?;
        Ok(id)
    }

    /// Return the number of pending jobs in the queue.
    pub async fn pending_count(&self) -> Result<u64, QueueError> {
        self.driver.pending_count().await
    }

    /// List all recent jobs for visual monitoring
    pub async fn list_all_jobs(&self, limit: u32) -> Result<Vec<QueuedJobDetail>, QueueError> {
        self.driver.list_all_jobs(limit).await
    }

    /// Retry a failed job in the queue
    pub async fn retry_failed_job(&self, job_id: &str) -> Result<(), QueueError> {
        self.driver.retry_failed_job(job_id).await
    }

    /// Purge failed jobs from the queue database
    pub async fn purge_completed_jobs(&self) -> Result<(), QueueError> {
        self.driver.purge_completed_jobs().await
    }

    /// Get an `Arc` reference to the internal driver (for sharing with `Worker`).
    pub(crate) fn driver_ref(&self) -> Arc<Box<dyn QueueDriver>> {
        Arc::clone(&self.driver)
    }
}

// ─── Worker ─────────────────────────────────────────────────────────────────

/// Type alias for job handler closures.
type JobHandler = Box<
    dyn Fn(
            Value,
        ) -> Pin<
            Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>,
        > + Send
        + Sync,
>;

/// Background worker that polls the queue and executes jobs.
///
/// Register handlers by job name, then call `.run()` to start processing.
///
/// # Example
/// ```rust,ignore
/// let mut worker = Worker::new(queue);
/// worker.register("send_email", |payload| async move {
///     let to = payload["to"].as_str().unwrap_or("unknown");
///     println!("Sending email to {}", to);
///     Ok(())
/// });
/// worker.run().await;
/// ```
pub struct Worker {
    driver: Arc<Box<dyn QueueDriver>>,
    handlers: HashMap<String, Arc<JobHandler>>,
    poll_interval_ms: u64,
}

impl Worker {
    /// Create a new worker attached to the given queue.
    pub fn new(queue: &Queue) -> Self {
        Self {
            driver: queue.driver_ref(),
            handlers: HashMap::new(),
            poll_interval_ms: 1000,
        }
    }

    /// Set the polling interval in milliseconds (default: 1000ms).
    pub fn poll_interval(mut self, ms: u64) -> Self {
        self.poll_interval_ms = ms;
        self
    }

    /// Register a handler for a specific job name.
    ///
    /// When a job with this name is popped from the queue, the handler
    /// closure is called with the job's JSON payload.
    pub fn register<F, Fut>(&mut self, name: &str, handler: F) -> &mut Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        let boxed: JobHandler = Box::new(move |payload| Box::pin(handler(payload)));
        self.handlers.insert(name.to_string(), Arc::new(boxed));
        self
    }

    /// Start processing jobs in the background.
    ///
    /// This spawns a Tokio task that continuously polls the queue.
    /// Call this during server startup (e.g., before `Server::run()`).
    pub fn run(&self) {
        let driver = Arc::clone(&self.driver);
        let handlers = self.handlers.clone();
        let poll_interval = self.poll_interval_ms;

        tokio::spawn(async move {
            println!(
                "🔄 Rullst Worker started. Polling every {}ms...",
                poll_interval
            );
            loop {
                let mut processed_job = false;
                match driver.pop().await {
                    Ok(Some(job)) => {
                        processed_job = true;
                        if let Some(handler) = handlers.get(&job.name) {
                            let handler = Arc::clone(handler);
                            let driver = Arc::clone(&driver);
                            let job_id = job.id.clone();
                            let job_name = job.name.clone();

                            tokio::spawn(async move {
                                match handler(job.payload).await {
                                    Ok(()) => {
                                        let _ = driver.mark_complete(&job_id).await;
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "❌ Job '{}' ({}) failed: {}",
                                            job_name, job_id, e
                                        );
                                        let _ = driver.mark_failed(&job_id, &e.to_string()).await;
                                    }
                                }
                            });
                        } else {
                            eprintln!("⚠️ No handler registered for job: {}", job.name);
                            let _ = driver.mark_failed(&job.id, "No handler registered").await;
                        }
                    }
                    Ok(None) => {
                        // No jobs available, wait before polling again
                    }
                    Err(e) => {
                        eprintln!("❌ Queue poll error: {}", e);
                    }
                }
                if !processed_job {
                    tokio::time::sleep(tokio::time::Duration::from_millis(poll_interval)).await;
                }
            }
        });
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_queue_push_pop() {
        let queue = Queue::sqlite("sqlite::memory:").await.unwrap();
        let job_id = queue
            .dispatch("test_job", serde_json::json!({"key": "value"}))
            .await
            .unwrap();
        assert!(!job_id.is_empty());

        // After dispatch, pending count should be 1
        let count = queue.pending_count().await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_sqlite_queue_pop_returns_correct_job() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        driver
            .push("job-1", "send_email", r#"{"to":"a@b.com"}"#)
            .await
            .unwrap();
        driver
            .push("job-2", "process_image", r#"{"path":"/img.png"}"#)
            .await
            .unwrap();

        // First pop should return job-1 (FIFO)
        let job = driver.pop().await.unwrap().unwrap();
        assert_eq!(job.id, "job-1");
        assert_eq!(job.name, "send_email");
        assert_eq!(job.payload["to"], "a@b.com");

        // Mark complete and pop next
        driver.mark_complete("job-1").await.unwrap();
        let job2 = driver.pop().await.unwrap().unwrap();
        assert_eq!(job2.id, "job-2");
        assert_eq!(job2.name, "process_image");
    }

    #[tokio::test]
    async fn test_sqlite_driver_get_pool() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        let pool = driver.get_pool();

        // Ensure the pool is valid and connected to the correct database schema
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_jobs")
            .fetch_one(pool)
            .await
            .unwrap();

        assert_eq!(row.0, 0);

        // Add a job and check the count again
        driver.push("test-job", "test_handler", "{}").await.unwrap();
        let row_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_jobs")
            .fetch_one(pool)
            .await
            .unwrap();

        assert_eq!(row_after.0, 1);
    }

    #[tokio::test]
    async fn test_sqlite_queue_mark_failed() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        driver.push("fail-job", "bad_job", r#"{}"#).await.unwrap();

        let job = driver.pop().await.unwrap().unwrap();
        driver
            .mark_failed(&job.id, "Something went wrong")
            .await
            .unwrap();

        // Job should no longer be pending
        let count = driver.pending_count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_sqlite_queue_empty_pop() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        let result = driver.pop().await.unwrap();
        assert!(result.is_none());
    }

    #[test]
    #[cfg(feature = "queue-redis")]
    fn test_queue_redis_creation() {
        // Happy path
        // `redis::Client::open` does not actually connect to the server, it only parses the URL
        let valid_result = Queue::redis("redis://127.0.0.1:6379");
        assert!(
            valid_result.is_ok(),
            "Failed to create Redis queue with valid URL"
        );

        // Error path
        let invalid_result = Queue::redis("invalid_url");
        assert!(
            invalid_result.is_err(),
            "Expected error for invalid Redis URL"
        );

        match invalid_result {
            Err(QueueError::Driver(msg)) => {
                assert!(
                    msg.contains("Failed to connect to Redis"),
                    "Unexpected error message: {}",
                    msg
                );
                assert!(
                    msg.contains("Redis URL did not parse"),
                    "Unexpected error message details: {}",
                    msg
                );
            }
            _ => panic!("Expected QueueError::Driver, got something else"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_queue_list_all_jobs_happy_path() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        // Initially empty
        let jobs = driver.list_all_jobs(10).await.unwrap();
        assert_eq!(jobs.len(), 0);

        // Manually insert jobs with specific timestamps to guarantee order
        sqlx::query("INSERT INTO rullst_jobs (id, name, payload, created_at) VALUES (?, ?, ?, ?)")
            .bind("job-1")
            .bind("test_job_1")
            .bind(r#"{"test": 1}"#)
            .bind("2020-01-01 10:00:00")
            .execute(&driver.pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO rullst_jobs (id, name, payload, created_at) VALUES (?, ?, ?, ?)")
            .bind("job-2")
            .bind("test_job_2")
            .bind(r#"{"test": 2}"#)
            .bind("2020-01-01 11:00:00")
            .execute(&driver.pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO rullst_jobs (id, name, payload, created_at) VALUES (?, ?, ?, ?)")
            .bind("job-3")
            .bind("test_job_3")
            .bind(r#"{"test": 3}"#)
            .bind("2020-01-01 12:00:00")
            .execute(&driver.pool)
            .await
            .unwrap();

        // Limit works, ordering works (DESC means newest first)
        let jobs = driver.list_all_jobs(2).await.unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].id, "job-3");
        assert_eq!(jobs[1].id, "job-2");

        let all_jobs = driver.list_all_jobs(10).await.unwrap();
        assert_eq!(all_jobs.len(), 3);
        assert_eq!(all_jobs[0].id, "job-3");
        assert_eq!(all_jobs[1].id, "job-2");
        assert_eq!(all_jobs[2].id, "job-1");
    }

    #[tokio::test]
    async fn test_sqlite_queue_list_all_jobs_error() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        // Drop the table to simulate DB error
        sqlx::query("DROP TABLE rullst_jobs")
            .execute(&driver.pool)
            .await
            .unwrap();

        let result = driver.list_all_jobs(10).await;
        assert!(result.is_err());
        match result {
            Err(QueueError::Driver(msg)) => {
                assert!(msg.contains("no such table"));
            }
            _ => panic!("Expected Driver error"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_queue_list_all_jobs_wrapper() {
        let queue = Queue::sqlite("sqlite::memory:").await.unwrap();

        // Dispatch some jobs
        queue
            .dispatch("job1", serde_json::json!({"data": 1}))
            .await
            .unwrap();
        queue
            .dispatch("job2", serde_json::json!({"data": 2}))
            .await
            .unwrap();

        // List jobs
        let jobs = queue.list_all_jobs(10).await.unwrap();
        assert_eq!(jobs.len(), 2);

        // Verify both jobs are retrieved successfully
        assert!(jobs.iter().any(|j| j.name == "job1"));
        assert!(jobs.iter().any(|j| j.name == "job2"));
    }

    #[tokio::test]
    async fn test_queue_list_all_jobs_error() {
        struct ErrorMockDriver;

        #[async_trait]
        impl QueueDriver for ErrorMockDriver {
            async fn push(
                &self,
                _id: &str,
                _job_name: &str,
                _payload: &str,
            ) -> Result<(), QueueError> {
                Ok(())
            }
            async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
                Ok(None)
            }
            async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
                Ok(())
            }
            async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
                Ok(())
            }
            async fn pending_count(&self) -> Result<u64, QueueError> {
                Ok(0)
            }
            async fn list_all_jobs(&self, _limit: u32) -> Result<Vec<QueuedJobDetail>, QueueError> {
                Err(QueueError::Driver("simulated db error".into()))
            }
        }

        let queue = Queue::custom(Box::new(ErrorMockDriver));
        let result = queue.list_all_jobs(10).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            QueueError::Driver(msg) => assert_eq!(msg, "simulated db error"),
            _ => panic!("Expected QueueError::Driver"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_queue_purge_completed_jobs() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        // Push two jobs
        driver
            .push("job-to-fail", "fail_job", r#"{}"#)
            .await
            .unwrap();
        driver
            .push("job-pending", "pending_job", r#"{}"#)
            .await
            .unwrap();

        // Pop the first job and mark it failed
        let job = driver.pop().await.unwrap().unwrap();
        assert_eq!(job.id, "job-to-fail");
        driver.mark_failed(&job.id, "Error").await.unwrap();

        // Now purge failed jobs
        driver.purge_completed_jobs().await.unwrap();

        // Ensure pending job still exists, but failed job is gone
        let pending = driver.pending_count().await.unwrap();
        assert_eq!(pending, 1);

        // Pop the pending job to verify it's the right one
        let job2 = driver.pop().await.unwrap().unwrap();
        assert_eq!(job2.id, "job-pending");

        // The failed job should not be retryable anymore (it's deleted)
        let _retry_result = driver.retry_failed_job("job-to-fail").await;
        // The retry function doesn't actually throw an error if job is not found,
        // it just updates 0 rows, so we will manually check via list_all_jobs or just rely on the above checks.
    }

    #[tokio::test]
    async fn test_sqlite_queue_purge_error() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        // Close the pool to simulate a driver error
        driver.pool.close().await;

        let result = driver.purge_completed_jobs().await;
        assert!(result.is_err());
        if let Err(QueueError::Driver(msg)) = result {
            assert!(
                msg.contains("PoolClosed") || msg.contains("closed"),
                "Unexpected error message: {}",
                msg
            );
        } else {
            panic!("Expected QueueError::Driver, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_sqlite_queue_list_all_jobs_driver() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();

        // 1. Test empty queue
        let jobs = driver.list_all_jobs(10).await.unwrap();
        assert!(jobs.is_empty(), "Expected empty jobs list initially");

        // 2. Populate queue and test limit
        driver
            .push("job-1", "job_type_a", r#"{"data": 1}"#)
            .await
            .unwrap();
        driver
            .push("job-2", "job_type_b", r#"{"data": 2}"#)
            .await
            .unwrap();
        driver
            .push("job-3", "job_type_c", r#"{"data": 3}"#)
            .await
            .unwrap();

        // Check if all 3 jobs are returned when limit is large enough
        let all_jobs = driver.list_all_jobs(10).await.unwrap();
        assert_eq!(all_jobs.len(), 3, "Expected 3 jobs");

        // SQLite order: "ORDER BY created_at DESC"
        // Since we insert them back-to-back, sometimes they have the exact same created_at
        // so we'll just check that they are all present
        assert!(all_jobs.iter().any(|j| j.id == "job-1"));
        assert!(all_jobs.iter().any(|j| j.id == "job-2"));
        assert!(all_jobs.iter().any(|j| j.id == "job-3"));

        // Test limit
        let limited_jobs = driver.list_all_jobs(2).await.unwrap();
        assert_eq!(
            limited_jobs.len(),
            2,
            "Expected exactly 2 jobs due to limit"
        );

        // 3. Test Err path
        // To trigger an error, we can close the connection pool and then attempt to query.
        driver.pool.close().await;

        let result = driver.list_all_jobs(10).await;
        assert!(result.is_err(), "Expected error after pool is closed");
        match result {
            Err(QueueError::Driver(msg)) => {
                assert!(
                    msg.contains("pool timed out") || msg.contains("closed"),
                    "Unexpected error message: {}",
                    msg
                );
            }
            _ => panic!("Expected QueueError::Driver"),
        }
    }

    #[tokio::test]
    async fn test_custom_queue_driver() {
        // Since we need to check was_push_called, we'll share state via Arc.
        let push_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        struct ArcMockDriver {
            push_called: std::sync::Arc<std::sync::atomic::AtomicBool>,
        }

        #[async_trait]
        impl QueueDriver for ArcMockDriver {
            async fn push(
                &self,
                _id: &str,
                _job_name: &str,
                _payload: &str,
            ) -> Result<(), QueueError> {
                self.push_called
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
            async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
                Ok(None)
            }
            async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
                Ok(())
            }
            async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
                Ok(())
            }
            async fn pending_count(&self) -> Result<u64, QueueError> {
                Ok(0)
            }
        }

        let driver = Box::new(ArcMockDriver {
            push_called: push_called.clone(),
        });

        let queue = Queue::custom(driver);
        let _id = queue
            .dispatch("test_custom_job", serde_json::json!({}))
            .await
            .unwrap();

        assert!(push_called.load(std::sync::atomic::Ordering::SeqCst));
    }

    pub struct MockPendingCountDriver {
        pub should_fail: bool,
    }

    #[async_trait]
    impl QueueDriver for MockPendingCountDriver {
        async fn push(&self, _id: &str, _job_name: &str, _payload: &str) -> Result<(), QueueError> {
            Ok(())
        }
        async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
            Ok(None)
        }
        async fn mark_complete(&self, _job_id: &str) -> Result<(), QueueError> {
            Ok(())
        }
        async fn mark_failed(&self, _job_id: &str, _error: &str) -> Result<(), QueueError> {
            Ok(())
        }
        async fn pending_count(&self) -> Result<u64, QueueError> {
            if self.should_fail {
                Err(QueueError::Driver("mock failure".to_string()))
            } else {
                Ok(42)
            }
        }
    }

    #[tokio::test]
    async fn test_queue_pending_count_ok() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let count = queue.pending_count().await.unwrap();
        assert_eq!(count, 42);
    }

    #[tokio::test]
    async fn test_queue_pending_count_err() {
        let driver = Box::new(MockPendingCountDriver { should_fail: true });
        let queue = Queue::custom(driver);
        let err = queue.pending_count().await.unwrap_err();
        match err {
            QueueError::Driver(msg) => assert_eq!(msg, "mock failure"),
            _ => panic!("Expected QueueError::Driver"),
        }
    }
}

#[cfg(test)]
mod tests_additional {
    use super::*;
    use super::tests::MockPendingCountDriver;
    #[tokio::test]
    async fn test_queue_retry_failed_job() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let res = queue.retry_failed_job("1").await;
        assert!(res.is_err()); // because Mock doesn't implement failed jobs fetching
    }
    #[tokio::test]
    async fn test_queue_list_all_jobs() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let res = queue.list_all_jobs(10).await;
        assert!(res.is_err());
    }
    #[tokio::test]
    async fn test_queue_dispatch() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let res = queue.dispatch("job", serde_json::json!({})).await;
        assert!(res.is_ok());
    }
    #[tokio::test]
    async fn test_queue_purge_completed_jobs() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let res = queue.purge_completed_jobs().await;
        assert!(res.is_err());
    }
    #[tokio::test]
    async fn test_queue_pending_count() {
        let driver = Box::new(MockPendingCountDriver { should_fail: false });
        let queue = Queue::custom(driver);
        let res = queue.pending_count().await;
        assert_eq!(res.unwrap(), 42);
    }
}
