// src/queue/sqlite.rs — SQLite-backed queue driver with automatic schema migrations.

use super::{QueueDriver, QueueError, QueuedJob, QueuedJobDetail};
use async_trait::async_trait;
use serde_json::Value;

/// Queue driver backed by a SQLite database.
///
/// Uses an auto-created `rullst_jobs` table. Perfect for local development
/// and small-to-medium production workloads. Zero external dependencies.
pub struct SqliteDriver {
    pub(crate) pool: sqlx::SqlitePool,
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

        // Add index for fast polling of pending jobs
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_rullst_jobs_status_created ON rullst_jobs(status, created_at)"
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Driver(format!("Failed to create rullst_jobs indexes: {}", e)))?;

        Ok(Self { pool })
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
        sqlx::query("UPDATE rullst_jobs SET status = 'pending', attempts = 0, error = NULL, updated_at = datetime('now') WHERE id = ? AND status = 'failed'")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::Driver(e.to_string()))?;
        Ok(())
    }

    /// Purges all failed jobs from the database.
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
