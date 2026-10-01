//! Bounded job listing for the SQLite queue driver.

use super::SqliteDriver;
use crate::queue::{QueueError, QueuedJobDetail};

impl SqliteDriver {
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
}
