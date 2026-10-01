//! Bounded job listing for the SQLite queue driver.

use super::SqliteDriver;
use crate::queue::preview::{field_prefix, optional_field_prefix, stored_prefix_bytes};
use crate::queue::{QueueError, QueuedJobDetail, QueuedJobPreview};

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

    /// Lists the records of [`Self::list_all_jobs`] with each payload and error
    /// cut to at most `max_field_bytes` bytes. SQLite returns only the leading
    /// `max_field_bytes + 1` bytes of each value, so complete payloads are
    /// never copied out of the database. Unpublished v13 API.
    pub async fn list_job_previews(
        &self,
        limit: u32,
        max_field_bytes: u32,
    ) -> Result<Vec<QueuedJobPreview>, QueueError> {
        #[derive(sqlx::FromRow)]
        struct PreviewRow {
            id: String,
            name: String,
            payload_head: Vec<u8>,
            status: String,
            error_head: Option<Vec<u8>>,
            attempts: i32,
            created_at: String,
            updated_at: String,
        }

        // CAST ... AS BLOB makes substr count bytes instead of characters.
        let rows: Vec<PreviewRow> = sqlx::query_as(
            "SELECT id, name, substr(CAST(payload AS BLOB), 1, ?2) AS payload_head, status, \
             substr(CAST(error AS BLOB), 1, ?2) AS error_head, attempts, created_at, updated_at \
             FROM rullst_jobs ORDER BY created_at DESC LIMIT ?1",
        )
        .bind(i64::from(limit))
        .bind(stored_prefix_bytes(max_field_bytes))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QueueError::Driver(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let (payload, payload_truncated) = field_prefix(&row.payload_head, max_field_bytes);
                let (error, error_truncated) =
                    optional_field_prefix(row.error_head.as_deref(), max_field_bytes);
                QueuedJobPreview {
                    id: row.id,
                    name: row.name,
                    payload,
                    payload_truncated,
                    status: row.status,
                    error,
                    error_truncated,
                    attempts: row.attempts,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                }
            })
            .collect())
    }
}

#[cfg(all(test, not(miri)))]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::queue::QueueDriver;

    #[tokio::test]
    async fn previews_cut_large_payloads_and_errors_inside_sqlite() {
        let driver = SqliteDriver::new("sqlite::memory:").await.unwrap();
        let large = format!("{{\"blob\":\"{}\"}}", "é".repeat(1_000_000));
        driver.push("large", "report", &large).await.unwrap();
        let claimed = driver.pop().await.unwrap().unwrap();
        driver
            .mark_failed(&claimed.id, &"ü".repeat(500_000))
            .await
            .unwrap();
        driver.push("small", "mail", r#"{"to":1}"#).await.unwrap();
        // Invalid UTF-8 written by another process is replaced, not rejected.
        sqlx::query(
            "INSERT INTO rullst_jobs (id, name, payload) VALUES ('raw', 'raw', CAST(X'7BFF7D' AS TEXT))",
        )
        .execute(&driver.pool)
        .await
        .unwrap();

        let previews = QueueDriver::list_job_previews(&driver, 10, 64)
            .await
            .unwrap();
        assert_eq!(previews.len(), 3);
        let by_id = |id: &str| previews.iter().find(|job| job.id == id).unwrap();

        let large = by_id("large");
        assert_eq!(large.status, "failed");
        assert!(large.payload.len() <= 64 && large.payload.starts_with("{\"blob\":\"é"));
        assert!(large.payload_truncated);
        let error = large.error.as_deref().unwrap();
        assert!(error.len() <= 64 && error.chars().all(|character| character == 'ü'));
        assert!(large.error_truncated);
        assert_eq!(large.attempts, 1);

        let small = by_id("small");
        assert_eq!(small.payload, r#"{"to":1}"#);
        assert!(!small.payload_truncated && small.error.is_none() && !small.error_truncated);
        assert_eq!(by_id("raw").payload, "{\u{fffd}}");

        assert!(driver.list_job_previews(1, 64).await.unwrap().len() == 1);
        assert!(driver.list_job_previews(0, 64).await.unwrap().is_empty());
        let empty = driver.list_job_previews(10, 0).await.unwrap();
        assert!(empty.iter().all(|job| job.payload.is_empty()));
        assert!(empty.iter().all(|job| job.payload_truncated));
    }
}
