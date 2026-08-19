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
//! let mut worker = Worker::new(&queue);
//! worker.register("send_email", |payload| async move {
//!     println!("Sending email to: {}", payload["to"]);
//!     Ok(())
//! });
//! worker.run();
//! ```

#[cfg(feature = "queue-redis")]
/// Redis queue driver implementation.
pub mod redis;
/// SQLite queue driver implementation.
pub mod sqlite;
/// Background job worker executor.
pub mod worker;

#[cfg(test)]
mod tests;

#[cfg(feature = "queue-redis")]
pub use redis::redis_driver::RedisDriver;
pub use sqlite::SqliteDriver;
pub use worker::{JobHandler, Worker};

use async_trait::async_trait;
use serde_json::Value;
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
