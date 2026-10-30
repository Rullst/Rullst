//! Bounded job listing for the SQLite queue driver.

use super::SqliteDriver;
use crate::queue::preview::{field_prefix, optional_field_prefix, stored_prefix_bytes};
use crate::queue::{QueueError, QueuedJobDetail, QueuedJobPreview};

impl SqliteDriver {
    /// Retrieves a list of all jobs up to the specified limit, sorted by creation time.
    pub async fn list_all_jobs(&self, limit: u32) -> Result<Vec<QueuedJobDetail>, QueueError> {
        // id, name, payload, status, error, attempts, created_at, updated_at.
        // A tuple keeps the crate free of SQLx's derive macros.
        type JobRow = (
            String,
            String,
            String,
            String,
            Option<String>,
            i32,
            String,
            String,
        );

        let rows: Vec<JobRow> = sqlx::query_as(
            "SELECT id, name, payload, status, error, attempts, created_at, updated_at FROM rullst_jobs ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QueueError::Driver(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(id, name, payload, status, error, attempts, created_at, updated_at)| {
                    QueuedJobDetail {
                        id,
                        name,
                        payload,
                        status,
                        error,
                        attempts,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect())
    }

    /// Lists the records of [`Self::list_all_jobs`] with each payload and error
    /// cut to at most `max_field_bytes` bytes. SQLite returns only the leading
    /// `max_field_bytes + 1` bytes of each value, so complete payloads are
    /// never copied out of the database. Added in v13.
    pub async fn list_job_previews(
        &self,
        limit: u32,
        max_field_bytes: u32,
    ) -> Result<Vec<QueuedJobPreview>, QueueError> {
        // id, name, payload_head, status, error_head, attempts, created_at,
        // updated_at, as a tuple like `list_all_jobs`.
        type PreviewRow = (
            String,
            String,
            Vec<u8>,
            String,
            Option<Vec<u8>>,
            i32,
            String,
            String,
        );

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
            .map(
                |(id, name, payload_head, status, error_head, attempts, created_at, updated_at)| {
                    let (payload, payload_truncated) = field_prefix(&payload_head, max_field_bytes);
                    let (error, error_truncated) =
                        optional_field_prefix(error_head.as_deref(), max_field_bytes);
                    QueuedJobPreview {
                        id,
                        name,
                        payload,
                        payload_truncated,
                        status,
                        error,
                        error_truncated,
                        attempts,
                        created_at,
                        updated_at,
                    }
                },
            )
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
