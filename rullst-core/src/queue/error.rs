//! Queue error type.

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
