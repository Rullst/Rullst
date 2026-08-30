//! # Rullst Queue System (`rullst::queue`)
//!
//! Provides a unified API for dispatching and processing background jobs.
//!
//! ## Drivers
//! - **SQLite** (`queue-sqlite`, opt-in): Uses an auto-created `rullst_jobs`
//!   table.
//! - **Redis** (optional): Requires the `queue-redis` feature flag.
//!
//! ## Quick Start
//! ```rust,no_run
//! use rullst_core::queue::{Queue, QueueError, Worker, WorkerHandle};
//!
//! async fn configure_worker(queue: &Queue) -> Result<WorkerHandle, QueueError> {
//!     queue
//!         .dispatch("send_email", serde_json::json!({"to": "user@example.com"}))
//!         .await?;
//!
//!     let mut worker = Worker::new(queue);
//!     worker.register("send_email", |payload| async move {
//!         println!("Sending email to: {}", payload["to"]);
//!         Ok(())
//!     });
//!     worker.run()
//! }
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
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const MAX_SCHEDULE_DELAY: Duration = Duration::from_secs(366 * 24 * 60 * 60);

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
    /// Push a job that must not be claimed before `available_at`.
    ///
    /// Custom drivers remain source-compatible. Their default implementation accepts jobs that
    /// are already due and fails closed for future delivery until the backend implements durable
    /// scheduling.
    async fn push_at(
        &self,
        id: &str,
        job_name: &str,
        payload: &str,
        available_at: SystemTime,
    ) -> Result<(), QueueError> {
        if available_at > SystemTime::now() {
            Err(QueueError::Unsupported(
                "this driver cannot persist future scheduled jobs".to_string(),
            ))
        } else {
            self.push(id, job_name, payload).await
        }
    }
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
        Err(QueueError::Unsupported(
            "this queue driver does not expose job inspection".to_string(),
        ))
    }
    /// Retry a failed job
    async fn retry_failed_job(&self, _job_id: &str) -> Result<(), QueueError> {
        Err(QueueError::Unsupported(
            "this queue driver does not expose failed-job retry".to_string(),
        ))
    }
    /// Legacy, misnamed hook that purges failed jobs in the SQLite driver.
    async fn purge_completed_jobs(&self) -> Result<(), QueueError> {
        Err(QueueError::Unsupported(
            "this queue driver does not expose failed-job purge".to_string(),
        ))
    }
    /// Purge failed jobs retained by the backend.
    async fn purge_failed_jobs(&self) -> Result<(), QueueError> {
        self.purge_completed_jobs().await
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
    pub async fn sqlite(database_url: impl Into<String>) -> Result<Self, QueueError> {
        let driver = SqliteDriver::new(database_url).await?;
        Ok(Self {
            driver: Arc::new(Box::new(driver)),
        })
    }

    /// Create a queue backed by Redis. Requires the `queue-redis` feature.
    #[cfg(feature = "queue-redis")]
    #[cfg_attr(mutants, mutants::skip)]
    pub fn redis(redis_url: impl Into<String>) -> Result<Self, QueueError> {
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

    /// Dispatches a durable job that cannot be claimed before `available_at`.
    ///
    /// The built-in SQLite and Redis drivers support millisecond scheduling for at most 366 days
    /// ahead. Actual execution occurs on the first worker poll after the timestamp; wall-clock
    /// precision, provider acceptance, and exactly-once delivery are not implied.
    pub async fn dispatch_at(
        &self,
        job_name: &str,
        payload: Value,
        available_at: SystemTime,
    ) -> Result<String, QueueError> {
        if available_at.duration_since(UNIX_EPOCH).is_err() {
            return Err(QueueError::InvalidConfiguration(
                "scheduled timestamp predates the Unix epoch".to_string(),
            ));
        }
        let now = SystemTime::now();
        if available_at
            .duration_since(now)
            .is_ok_and(|delay| delay > MAX_SCHEDULE_DELAY)
        {
            return Err(QueueError::InvalidConfiguration(
                "scheduled jobs may be dispatched at most 366 days ahead".to_string(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        let payload_str = serde_json::to_string(&payload)
            .map_err(|error| QueueError::Serialization(error.to_string()))?;
        self.driver
            .push_at(&id, job_name, &payload_str, available_at)
            .await?;
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

    /// Purge failed jobs retained by the queue backend.
    pub async fn purge_failed_jobs(&self) -> Result<(), QueueError> {
        self.driver.purge_failed_jobs().await
    }

    /// Legacy compatibility name for [`Self::purge_failed_jobs`].
    #[deprecated(
        since = "12.0.0",
        note = "use purge_failed_jobs; this method has always targeted failed rows"
    )]
    #[cfg_attr(mutants, mutants::skip)]
    pub async fn purge_completed_jobs(&self) -> Result<(), QueueError> {
        self.purge_failed_jobs().await
    }

    /// Get an `Arc` reference to the internal driver (for sharing with `Worker`).
    pub(crate) fn driver_ref(&self) -> Arc<Box<dyn QueueDriver>> {
        Arc::clone(&self.driver)
    }
}

#[cfg(any(feature = "queue-sqlite", feature = "queue-redis"))]
pub(crate) fn unix_timestamp_millis_ceil(timestamp: SystemTime) -> Result<u64, QueueError> {
    let duration = timestamp.duration_since(UNIX_EPOCH).map_err(|_| {
        QueueError::InvalidConfiguration("timestamp predates Unix epoch".to_string())
    })?;
    let whole_millis = duration.as_millis();
    let has_fractional_millisecond = duration.subsec_nanos() % 1_000_000 != 0;
    let rounded_millis = whole_millis
        .checked_add(u128::from(has_fractional_millisecond))
        .ok_or_else(|| {
            QueueError::InvalidConfiguration("timestamp exceeds queue storage range".to_string())
        })?;
    u64::try_from(rounded_millis).map_err(|_| {
        QueueError::InvalidConfiguration("timestamp exceeds queue storage range".to_string())
    })
}

#[cfg(feature = "queue-sqlite")]
pub(crate) fn unix_timestamp_millis_floor(timestamp: SystemTime) -> Result<u64, QueueError> {
    let duration = timestamp.duration_since(UNIX_EPOCH).map_err(|_| {
        QueueError::InvalidConfiguration("timestamp predates Unix epoch".to_string())
    })?;
    u64::try_from(duration.as_millis()).map_err(|_| {
        QueueError::InvalidConfiguration("timestamp exceeds queue storage range".to_string())
    })
}
