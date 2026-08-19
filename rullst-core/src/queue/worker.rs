// src/queue/worker.rs — Background worker and async job execution runner.

use super::{Queue, QueueDriver};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for job handler closures.
pub type JobHandler = Box<
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
    #[cfg_attr(mutants, mutants::skip)]
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
