// src/queue/sqlite.rs — SQLite-backed queue driver with automatic schema migrations.

use super::{
    QueueDriver, QueueError, QueuedJob, QueuedJobDetail, unix_timestamp_millis_ceil,
    unix_timestamp_millis_floor,
};
use async_trait::async_trait;
use std::time::{Duration, SystemTime};

const MAX_COMPLETED_HISTORY: usize = 100_000;

/// Queue driver backed by a SQLite database.
///
/// Uses an auto-created `rullst_jobs` table. Perfect for local development
/// and small-to-medium production workloads. Zero external dependencies.
pub struct SqliteDriver {
    pub(crate) pool: sqlx::SqlitePool,
    completed_history_limit: usize,
}

impl SqliteDriver {
    /// Create a new SQLite queue driver. Automatically creates the `rullst_jobs`
    /// table if it doesn't exist.
    pub async fn new(database_url: impl Into<String>) -> Result<Self, QueueError> {
        let database_url = database_url.into();
        let pool = sqlx::SqlitePool::connect(&database_url)
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
                available_at_ms INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )"#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Driver(format!("Failed to create rullst_jobs table: {}", e)))?;

        let scheduled_column: Option<String> = sqlx::query_scalar(
            "SELECT name FROM pragma_table_info('rullst_jobs') WHERE name = 'available_at_ms'",
        )
        .fetch_optional(&pool)
        .await
        .map_err(|error| {
            QueueError::Driver(format!("Failed to inspect rullst_jobs columns: {error}"))
        })?;
        if scheduled_column.is_none() {
            sqlx::query(
                "ALTER TABLE rullst_jobs ADD COLUMN available_at_ms INTEGER NOT NULL DEFAULT 0",
            )
            .execute(&pool)
            .await
            .map_err(|error| {
                QueueError::Driver(format!(
                    "Failed to add rullst_jobs scheduling column: {error}"
                ))
            })?;
        }

        // Add index for fast polling of pending jobs
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_rullst_jobs_status_created ON rullst_jobs(status, created_at)"
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Driver(format!("Failed to create rullst_jobs indexes: {}", e)))?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_rullst_jobs_ready ON rullst_jobs(status, available_at_ms, created_at)",
        )
        .execute(&pool)
        .await
        .map_err(|error| {
            QueueError::Driver(format!("Failed to create scheduled-job index: {error}"))
        })?;

        Ok(Self {
            pool,
            completed_history_limit: 0,
        })
    }

    /// Enables bounded retention of successful jobs for monitoring.
    ///
    /// Retention is disabled by default so successful payloads are deleted. When enabled, the
    /// completion transition and pruning are committed atomically.
    pub fn try_with_completed_history_limit(
        mut self,
        retained_jobs: usize,
    ) -> Result<Self, QueueError> {
        if !(1..=MAX_COMPLETED_HISTORY).contains(&retained_jobs) {
            return Err(QueueError::InvalidConfiguration(format!(
                "completed job history must retain between 1 and {MAX_COMPLETED_HISTORY} records"
            )));
        }
        self.completed_history_limit = retained_jobs;
        Ok(self)
    }

    /// Returns a reference to the internal SQLite pool.
    pub fn get_pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }

    /// Retrieves a list of all jobs up to the specified limit, sorted by creation time.
    pub async fn list_all_jobs(&self, limit: u32) -> Result<Vec<QueuedJobDetail>, QueueError> {
        #[derive(sqlx::FromRow)]
        struct JobRow {
            id: String,
            name: String,
            payload: String,
            status: String,
            error: Option<String>,
            attempts: i32,
            created_at: String,
            updated_at: String,
        }

        let rows: Vec<JobRow> = sqlx::query_as(
            "SELECT id, name, payload, status, error, attempts, created_at, updated_at FROM rullst_jobs ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QueueError::Driver(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| QueuedJobDetail {
                id: row.id,
                name: row.name,
                payload: row.payload,
                status: row.status,
                error: row.error,
                attempts: row.attempts,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    /// Retries a failed job by resetting its status to 'pending' and clearing error details.
    pub async fn retry_failed_job(&self, job_id: &str) -> Result<(), QueueError> {
        let result = sqlx::query("UPDATE rullst_jobs SET status = 'pending', attempts = 0, error = NULL, available_at_ms = 0, updated_at = datetime('now') WHERE id = ? AND status = 'failed'")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::Driver(e.to_string()))?;
        ensure_transition(result.rows_affected(), job_id, "retry_failed")?;
        Ok(())
    }

    /// Purges all failed jobs from the database.
    pub async fn purge_failed_jobs(&self) -> Result<(), QueueError> {
        sqlx::query("DELETE FROM rullst_jobs WHERE status = 'failed'")
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::Driver(e.to_string()))?;
        Ok(())
    }

    /// Purges successful jobs retained by the explicit history policy.
    pub async fn purge_completed_history(&self) -> Result<(), QueueError> {
        sqlx::query("DELETE FROM rullst_jobs WHERE status = 'completed'")
            .execute(&self.pool)
            .await
            .map_err(|error| {
                QueueError::Driver(format!("Failed to purge completed job history: {error}"))
            })?;
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
            .map_err(|error| QueueError::Driver(format!("Failed to push job: {error}")))?;
        Ok(())
    }

    async fn push_at(
        &self,
        id: &str,
        job_name: &str,
        payload: &str,
        available_at: SystemTime,
    ) -> Result<(), QueueError> {
        let available_at_ms =
            i64::try_from(unix_timestamp_millis_ceil(available_at)?).map_err(|_| {
                QueueError::InvalidConfiguration(
                    "scheduled timestamp exceeds SQLite integer range".to_string(),
                )
            })?;
        sqlx::query(
            "INSERT INTO rullst_jobs (id, name, payload, available_at_ms) VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(job_name)
        .bind(payload)
        .bind(available_at_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Driver(format!("Failed to push job: {}", e)))?;
        Ok(())
    }

    async fn pop(&self) -> Result<Option<QueuedJob>, QueueError> {
        let now_ms =
            i64::try_from(unix_timestamp_millis_floor(SystemTime::now())?).map_err(|_| {
                QueueError::Driver("current timestamp exceeds SQLite integer range".to_string())
            })?;
        // Atomically select and mark the oldest pending job as 'processing'
        let row: Option<(String, String, String, i32)> = sqlx::query_as(
            r#"UPDATE rullst_jobs
               SET status = 'processing', attempts = attempts + 1, updated_at = datetime('now')
               WHERE id = (
                   SELECT id FROM rullst_jobs
                   WHERE status = 'pending' AND available_at_ms <= ?
                   ORDER BY available_at_ms ASC, created_at ASC LIMIT 1
               )
               RETURNING id, name, payload, attempts"#,
        )
        .bind(now_ms)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::Driver(format!("Failed to pop job: {}", e)))?;

        let Some((id, name, payload_str, attempts)) = row else {
            return Ok(None);
        };

        let payload = match serde_json::from_str(&payload_str) {
            Ok(payload) => payload,
            Err(error) => {
                let message = format!("invalid JSON payload: {error}");
                self.mark_failed(&id, &message)
                    .await
                    .map_err(|transition| QueueError::StateTransition {
                        job_id: id.clone(),
                        operation: "reject_invalid_payload",
                        message: format!("{message}; {transition}"),
                    })?;
                return Err(QueueError::Serialization(format!(
                    "job '{id}' contains invalid JSON: {error}"
                )));
            }
        };
        let attempts = match u32::try_from(attempts) {
            Ok(attempts) => attempts,
            Err(_) => {
                let message = "job attempts counter is negative";
                self.mark_failed(&id, message).await.map_err(|transition| {
                    QueueError::StateTransition {
                        job_id: id.clone(),
                        operation: "reject_invalid_attempts",
                        message: transition.to_string(),
                    }
                })?;
                return Err(QueueError::Driver(format!("job '{id}' has {message}")));
            }
        };

        Ok(Some(QueuedJob {
            id,
            name,
            payload,
            attempts,
        }))
    }

    async fn mark_complete(&self, job_id: &str) -> Result<(), QueueError> {
        if self.completed_history_limit == 0 {
            let result =
                sqlx::query("DELETE FROM rullst_jobs WHERE id = ? AND status = 'processing'")
                    .bind(job_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|error| {
                        QueueError::Driver(format!("Failed to mark job complete: {error}"))
                    })?;
            ensure_transition(result.rows_affected(), job_id, "mark_complete")?;
            return Ok(());
        }

        let retained_jobs = i64::try_from(self.completed_history_limit).map_err(|_| {
            QueueError::InvalidConfiguration(
                "completed job history exceeds SQLite integer range".to_string(),
            )
        })?;
        let mut transaction = self.pool.begin().await.map_err(|error| {
            QueueError::Driver(format!(
                "Failed to begin job completion transaction: {error}"
            ))
        })?;
        let result = sqlx::query(
            "UPDATE rullst_jobs SET status = 'completed', error = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ? AND status = 'processing'",
        )
        .bind(job_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| QueueError::Driver(format!("Failed to retain completed job: {error}")))?;
        ensure_transition(result.rows_affected(), job_id, "mark_complete")?;
        sqlx::query(
            "DELETE FROM rullst_jobs WHERE status = 'completed' AND id NOT IN (SELECT id FROM rullst_jobs WHERE status = 'completed' ORDER BY updated_at DESC, rowid DESC LIMIT ?)",
        )
        .bind(retained_jobs)
        .execute(&mut *transaction)
        .await
        .map_err(|error| {
            QueueError::Driver(format!("Failed to prune completed job history: {error}"))
        })?;
        transaction.commit().await.map_err(|error| {
            QueueError::Driver(format!("Failed to commit completed job history: {error}"))
        })?;
        Ok(())
    }

    async fn mark_failed(&self, job_id: &str, error: &str) -> Result<(), QueueError> {
        let result = sqlx::query(
            "UPDATE rullst_jobs SET status = 'failed', error = ?, updated_at = datetime('now') WHERE id = ? AND status = 'processing'",
        )
        .bind(error)
        .bind(job_id)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Driver(format!("Failed to mark job failed: {}", e)))?;
        ensure_transition(result.rows_affected(), job_id, "mark_failed")?;
        Ok(())
    }

    async fn requeue(&self, job_id: &str, reason: &str) -> Result<(), QueueError> {
        let result = sqlx::query(
            "UPDATE rullst_jobs SET status = 'pending', error = ?, available_at_ms = 0, updated_at = datetime('now') WHERE id = ? AND status = 'processing'",
        )
        .bind(reason)
        .bind(job_id)
        .execute(&self.pool)
        .await
        .map_err(|error| QueueError::StateTransition {
            job_id: job_id.to_string(),
            operation: "requeue",
            message: error.to_string(),
        })?;
        ensure_transition(result.rows_affected(), job_id, "requeue")?;
        Ok(())
    }

    async fn recover_stalled(&self, stale_after: Duration) -> Result<u64, QueueError> {
        let stale_seconds = stale_after.as_secs().max(1);
        let modifier = format!("-{stale_seconds} seconds");
        let result = sqlx::query(
            "UPDATE rullst_jobs SET status = 'pending', error = 'recovered after worker interruption', available_at_ms = 0, updated_at = datetime('now') WHERE status = 'processing' AND updated_at <= datetime('now', ?)",
        )
        .bind(modifier)
        .execute(&self.pool)
        .await
        .map_err(|error| QueueError::Driver(format!("Failed to recover stalled jobs: {error}")))?;
        Ok(result.rows_affected())
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
        SqliteDriver::purge_failed_jobs(self).await
    }

    async fn purge_failed_jobs(&self) -> Result<(), QueueError> {
        SqliteDriver::purge_failed_jobs(self).await
    }

    async fn purge_completed_history(&self) -> Result<(), QueueError> {
        SqliteDriver::purge_completed_history(self).await
    }
}

fn ensure_transition(
    rows_affected: u64,
    job_id: &str,
    operation: &'static str,
) -> Result<(), QueueError> {
    if rows_affected == 1 {
        Ok(())
    } else {
        Err(QueueError::StateTransition {
            job_id: job_id.to_string(),
            operation,
            message: format!("expected one processing job, affected {rows_affected}"),
        })
    }
}
