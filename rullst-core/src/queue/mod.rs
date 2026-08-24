//! # Rullst Queue System (`rullst::queue`)
//!
//! Provides a unified API for dispatching and processing background jobs.
//!
//! ## Drivers
//! - **SQLite** (`queue-sqlite`, enabled by default): Uses an auto-created
//!   `rullst_jobs` table. Zero config.
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
//! let mut worker = Worker::new(&queue);
//! worker.register("send_email", |payload| async move {
//!     println!("Sending email to: {}", payload["to"]);
//!     Ok(())
//! });
//! let worker_handle = worker.run()?;
//! # let _ = worker_handle;
//! ```

#[cfg(feature = "queue-redis")]
/// Redis queue driver implementation.
pub mod redis;
#[cfg(feature = "queue-sqlite")]
/// SQLite queue driver implementation.
pub mod sqlite;
/// Background job worker executor.
pub mod worker;

#[cfg(all(test, feature = "queue-sqlite"))]
mod tests;
#[cfg(test)]
mod worker_tests;

#[cfg(feature = "queue-redis")]
pub use redis::redis_driver::RedisDriver;
#[cfg(feature = "queue-sqlite")]
pub use sqlite::SqliteDriver;
pub use worker::{JobHandler, Worker, WorkerHandle};

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

// ─── Error Types ────────────────────────────────────────────────────────────

/// Errors that can occur during queue operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QueueError {
    /// The underlying database or connection failed.
    #[error("Queue driver error: {0}")]
    Driver(String),
    /// Serialization/deserialization of job payloads failed.
    #[error("Queue serialization error: {0}")]
    Serialization(String),
    /// A job handler was not found for the given job name.
    #[error("No handler registered for job: {0}")]
    HandlerNotFound(String),
    /// The job execution itself failed.
    #[error("Job execution failed: {0}")]
    JobFailed(String),
    /// The worker was started outside an active Tokio runtime.
    #[error("the queue worker requires an active Tokio runtime")]
    RuntimeUnavailable,
    /// A worker option would make safe processing impossible.
    #[error("invalid queue worker configuration: {0}")]
    InvalidConfiguration(String),
    /// Persisting a job state transition failed.
    #[error("job '{job_id}' could not transition via '{operation}': {message}")]
    StateTransition {
        /// Job whose state could not be persisted.
        job_id: String,
        /// Attempted transition operation.
        operation: &'static str,
        /// Driver failure description.
        message: String,
    },
    /// A handler exceeded the configured execution deadline.
    #[error("job '{job_id}' exceeded its {timeout_ms}ms execution timeout")]
    JobTimedOut {
        /// Timed-out job identifier.
        job_id: String,
        /// Configured timeout in milliseconds.
        timeout_ms: u64,
    },
    /// A handler panicked and was isolated by the worker.
    #[error("job '{job_id}' handler panicked")]
    JobPanicked {
        /// Panicking job identifier.
        job_id: String,
    },
    /// An internal worker task terminated unexpectedly.
    #[error("queue worker task failed: {0}")]
    WorkerTask(String),
    /// A custom queue backend does not implement an optional lifecycle action.
    #[error("queue operation is unsupported: {0}")]
    Unsupported(String),
}

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
    /// Unique identifier of the queued job.
    pub id: String,
    /// The name/type of the job.
    pub name: String,
    /// The JSON payload of the job as a string.
    pub payload: String,
    /// Current execution status of the job (e.g. "pending", "processing", "failed").
    pub status: String,
    /// Error message if the job failed.
    pub error: Option<String>,
    /// Number of processing attempts made so far.
    pub attempts: i32,
    /// Time when the job was originally created/pushed.
    pub created_at: String,
    /// Time when the job status was last updated.
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
    /// Return an interrupted processing job to the pending state.
    async fn requeue(&self, _job_id: &str, _reason: &str) -> Result<(), QueueError> {
        Err(QueueError::Unsupported(
            "this driver cannot requeue interrupted jobs".to_string(),
        ))
    }
    /// Recover processing leases left behind by a crashed worker.
    async fn recover_stalled(&self, _stale_after: std::time::Duration) -> Result<u64, QueueError> {
        Err(QueueError::Unsupported(
            "this driver cannot recover stalled jobs".to_string(),
        ))
    }
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

// ─── Queue Facade ───────────────────────────────────────────────────────────

/// The main queue facade for dispatching background jobs.
///
/// Provides a driver-agnostic API. Create with `Queue::sqlite()` or `Queue::redis()`.
pub struct Queue {
    driver: Arc<Box<dyn QueueDriver>>,
}

impl Queue {
    /// Create a queue backed by SQLite. The `rullst_jobs` table is auto-created.
    #[cfg(feature = "queue-sqlite")]
    pub async fn sqlite(database_url: &str) -> Result<Self, QueueError> {
        let driver = SqliteDriver::new(database_url).await?;
        Ok(Self {
            driver: Arc::new(Box::new(driver)),
        })
    }

    /// Create a queue backed by Redis. Requires the `queue-redis` feature.
    #[cfg(feature = "queue-redis")]
    #[cfg_attr(mutants, mutants::skip)]
    pub fn redis(redis_url: &str) -> Result<Self, QueueError> {
        let driver = redis::redis_driver::RedisDriver::new(redis_url)?;
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
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn retry_failed_job(&self, job_id: &str) -> Result<(), QueueError> {
        self.driver.retry_failed_job(job_id).await
    }

    /// Purge failed jobs from the queue database
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn purge_completed_jobs(&self) -> Result<(), QueueError> {
        self.driver.purge_completed_jobs().await
    }

    /// Get an `Arc` reference to the internal driver (for sharing with `Worker`).
    pub(crate) fn driver_ref(&self) -> Arc<Box<dyn QueueDriver>> {
        Arc::clone(&self.driver)
    }
}
