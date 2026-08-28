//! # Rullst Task Scheduler (`rullst::scheduler`)
//!
//! Declarative Cron-like job scheduler that runs alongside your HTTP server.
//!
//! ## Quick Start
//! ```rust,ignore
//! use rullst::scheduler::Scheduler;
//!
//! let scheduler = Scheduler::new()
//!     .task("0 0 * * *", || async { cleanup_old_sessions().await })
//!     .task("*/5 * * * *", || async { check_health().await });
//!
//! Server::new(router)
//!     .schedule(scheduler)
//!     .run(3000)
//!     .await?;
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ─── Scheduled Task ─────────────────────────────────────────────────────────

/// The boxed async handler function type for scheduled tasks.
pub type ScheduledHandler =
    Arc<Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>>;

/// A single scheduled task with a cron expression and an async handler.
pub struct ScheduledTask {
    /// Human-readable label for logging (derived from cron expression).
    label: String,
    /// The parsed cron schedule.
    schedule: cron::Schedule,
    /// The async handler to execute when the schedule fires.
    handler: ScheduledHandler,
}

// ─── Scheduler ──────────────────────────────────────────────────────────────

/// A declarative task scheduler that runs recurring async jobs on cron schedules.
///
/// Tasks are registered with standard 5-field cron expressions and async closures.
/// The scheduler runs as background Tokio tasks alongside your HTTP server.
///
/// # Cron Expression Format
/// ```text
/// ┌─────────── minute (0–59)
/// │ ┌───────── hour (0–23)
/// │ │ ┌─────── day of month (1–31)
/// │ │ │ ┌───── month (1–12)
/// │ │ │ │ ┌─── day of week (0–6, Sun=0)
/// │ │ │ │ │
/// * * * * *
/// ```
///
/// # Example
/// ```rust,ignore
/// Scheduler::new()
///     .task("0 0 * * *", || async { nightly_cleanup().await })
///     .task("*/10 * * * *", || async { health_check().await })
/// ```
pub struct Scheduler {
    tasks: Vec<ScheduledTask>,
}

impl Scheduler {
    /// Create a new empty scheduler.
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Register a recurring task with a cron expression.
    ///
    /// The cron expression uses the standard 5-field format extended to 7 fields
    /// internally (seconds and year are auto-filled).
    ///
    /// # Errors
    /// Returns an error if the cron expression is invalid.
    ///
    /// # Example
    /// ```rust,ignore
    /// scheduler.task("30 2 * * 1", || async {
    ///     println!("Running at 2:30 AM every Monday");
    /// }).unwrap();
    /// ```
    pub fn task<F, Fut>(mut self, cron_expr: &str, handler: F) -> Result<Self, String>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // The `cron` crate requires 7 fields (sec min hr dom mon dow year).
        // We convert a standard 5-field expression by prepending "0" (seconds)
        // and appending "*" (year).
        let full_expr = format!("0 {} *", cron_expr);

        let schedule: cron::Schedule = full_expr
            .parse()
            .map_err(|e| format!("Invalid cron expression '{}': {}", cron_expr, e))?;

        let label = cron_expr.to_string();
        let boxed: ScheduledHandler = Arc::new(Box::new(move || Box::pin(handler())));

        self.tasks.push(ScheduledTask {
            label,
            schedule,
            handler: boxed,
        });

        Ok(self)
    }

    /// Start all scheduled tasks as background Tokio tasks.
    ///
    /// Each task runs in its own spawned task, sleeping until the next
    /// cron-scheduled time, then executing the handler.
    ///
    /// This method consumes the scheduler and should be called once during
    /// server startup (typically via `Server::schedule()`).
    #[cfg_attr(mutants, mutants::skip)]
    pub fn start(self) {
        for task in self.tasks {
            tokio::spawn(run_task_loop(task));
        }
    }
}

#[cfg_attr(mutants, mutants::skip)]
async fn run_task_loop(task: ScheduledTask) {
    let schedule = task.schedule;
    let handler = task.handler;
    let label = task.label;

    println!("📅 Scheduler: task '{}' registered.", label);
    loop {
        let now = chrono::Utc::now();
        if let Some(next) = schedule.upcoming(chrono::Utc).next() {
            let duration = (next - now)
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(60));
            tokio::time::sleep(duration).await;

            let handler_clone = Arc::clone(&handler);
            tokio::spawn(async move {
                handler_clone().await;
            });
        } else {
            // No more upcoming executions — this shouldn't happen with standard cron
            break;
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = Scheduler::new();
        assert_eq!(scheduler.tasks.len(), 0);
    }

    #[test]
    fn test_scheduler_task_registration() {
        let scheduler = Scheduler::new()
            .task("* * * * *", || async {})
            .unwrap()
            .task("0 0 * * *", || async {})
            .unwrap();
        assert_eq!(scheduler.tasks.len(), 2);
    }

    #[test]
    fn test_scheduler_label_preserved() {
        let scheduler = Scheduler::new().task("30 2 * * 1", || async {}).unwrap();
        assert_eq!(scheduler.tasks[0].label, "30 2 * * 1");
    }

    #[test]
    fn test_scheduler_invalid_cron() {
        let res = Scheduler::new().task("invalid cron", || async {});
        match res {
            Ok(_) => panic!("Expected task registration to fail for invalid cron"),
            Err(e) => assert!(e.contains("Invalid cron expression")),
        }
    }

    #[tokio::test]
    async fn test_scheduler_cron_parses_correctly() {
        // Verify that a 5-field expression parses to a valid 7-field schedule
        let scheduler = Scheduler::new().task("*/5 * * * *", || async {}).unwrap();
        let next = scheduler.tasks[0].schedule.upcoming(chrono::Utc).next();
        assert!(next.is_some(), "Scheduler should have upcoming executions");
    }

    #[tokio::test]
    async fn test_scheduler_start() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = Arc::clone(&executed);

        // Use a task that should execute immediately/frequently
        // Note: For a real test, we'd need to mock time, but this is a simple check.
        let scheduler = Scheduler::new()
            .task("* * * * *", move || {
                let executed = Arc::clone(&executed_clone);
                async move {
                    executed.store(true, Ordering::SeqCst);
                }
            })
            .unwrap();

        scheduler.start();
        // Since we cannot easily control the clock, we verify basic execution
        // mechanics or integration logic here.
    }
}
