// src/queue/sqlite.rs — SQLite-backed queue driver with automatic schema migrations.

use super::{
    QueueDriver, QueueError, QueuedJob, QueuedJobDetail, unix_timestamp_millis_ceil,
    unix_timestamp_millis_floor,
};
use async_trait::async_trait;
use std::time::{Duration, SystemTime};

const MAX_COMPLETED_HISTORY: usize = 100_000;

mod recovery;
mod schema;

/// Queue driver backed by a SQLite database.
///
/// Uses an auto-created `rullst_jobs` table. Perfect for local development
/// and small-to-medium production workloads. Zero external dependencies.
pub struct SqliteDriver {
    pub(crate) pool: sqlx::SqlitePool,
    completed_history_limit: usize,
    max_stalled_leases: u32,
}

impl SqliteDriver {
    /// Create a new SQLite queue driver. Automatically creates the `rullst_jobs`
    /// table if it doesn't exist.
    pub async fn new(database_url: impl Into<String>) -> Result<Self, QueueError> {
        let database_url = database_url.into();
        let pool = sqlx::SqlitePool::connect(&database_url)
            .await
            .map_err(|e| QueueError::Driver(format!("Failed to connect to SQLite: {}", e)))?;
        schema::prepare(&pool).await?;

        Ok(Self {
            pool,
            completed_history_limit: 0,
            max_stalled_leases: super::DEFAULT_MAX_STALLED_LEASES,
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

    /// Sets how many times a job's lease may stall before recovery fails the
    /// job instead of requeuing it (default
    /// [`super::DEFAULT_MAX_STALLED_LEASES`]; `1` fails on the first stall).
    ///
    /// Unpublished v13 API.
    ///
    /// # Errors
    /// Returns [`QueueError::InvalidConfiguration`] outside
    /// `1..=`[`super::MAX_STALLED_LEASES_LIMIT`].
    pub fn try_with_max_stalled_leases(mut self, leases: u32) -> Result<Self, QueueError> {
        self.max_stalled_leases = super::validate_max_stalled_leases(leases)?;
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
        let result = sqlx::query("UPDATE rullst_jobs SET status = 'pending', attempts = 0, stalled_recoveries = 0, error = NULL, available_at_ms = 0, updated_at = datetime('now') WHERE id = ? AND status = 'failed'")
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

    /// Claims the oldest due job. With `lease`, the claim stalls only after
    /// that lease; without one, after the recovering worker's age.
    async fn claim(&self, lease: Option<Duration>) -> Result<Option<QueuedJob>, QueueError> {
        let now_ms = current_unix_millis()?;
        let lease_expires_at_ms = lease.map_or(0, |lease| {
            let lease_ms = i64::try_from(lease.as_millis()).unwrap_or(i64::MAX).max(1);
            now_ms.saturating_add(lease_ms)
        });
        // Atomically select and mark the oldest pending job as 'processing'
        let row: Option<(String, String, String, i32)> = sqlx::query_as(
            r#"UPDATE rullst_jobs
               SET status = 'processing', attempts = attempts + 1,
                   lease_expires_at_ms = ?, updated_at = datetime('now')
               WHERE id = (
                   SELECT id FROM rullst_jobs
                   WHERE status = 'pending' AND available_at_ms <= ?
                   ORDER BY available_at_ms ASC, created_at ASC LIMIT 1
               )
               RETURNING id, name, payload, attempts"#,
        )
        .bind(lease_expires_at_ms)
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

    /// Completes a processing job. With `attempt`, only the claim with that
    /// attempt number matches, so a recovered and re-claimed lease is fenced.
    async fn complete_claim(&self, job_id: &str, attempt: Option<u32>) -> Result<(), QueueError> {
        let attempt = attempt.map(i64::from);
        if self.completed_history_limit == 0 {
            let result = sqlx::query(
                "DELETE FROM rullst_jobs WHERE id = ? AND status = 'processing' AND attempts = COALESCE(?, attempts)",
            )
            .bind(job_id)
            .bind(attempt)
            .execute(&self.pool)
            .await
            .map_err(|error| QueueError::Driver(format!("Failed to mark job complete: {error}")))?;
            ensure_claim_transition(result.rows_affected(), job_id, "mark_complete", attempt)?;
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
            "UPDATE rullst_jobs SET status = 'completed', error = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ? AND status = 'processing' AND attempts = COALESCE(?, attempts)",
        )
        .bind(job_id)
        .bind(attempt)
        .execute(&mut *transaction)
        .await
        .map_err(|error| QueueError::Driver(format!("Failed to retain completed job: {error}")))?;
        ensure_claim_transition(result.rows_affected(), job_id, "mark_complete", attempt)?;
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

    async fn fail_claim(
        &self,
        job_id: &str,
        attempt: Option<u32>,
        error: &str,
    ) -> Result<(), QueueError> {
        let attempt = attempt.map(i64::from);
        let result = sqlx::query(
            "UPDATE rullst_jobs SET status = 'failed', error = ?, updated_at = datetime('now') WHERE id = ? AND status = 'processing' AND attempts = COALESCE(?, attempts)",
        )
        .bind(error)
        .bind(job_id)
        .bind(attempt)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Driver(format!("Failed to mark job failed: {}", e)))?;
        ensure_claim_transition(result.rows_affected(), job_id, "mark_failed", attempt)
    }

    /// Returns a processing job to pending, claimable from `available_at_ms`
    /// (`0` means immediately).
    async fn requeue_claim(
        &self,
        job_id: &str,
        attempt: Option<u32>,
        reason: &str,
        available_at_ms: i64,
    ) -> Result<(), QueueError> {
        let attempt = attempt.map(i64::from);
        let result = sqlx::query(
            "UPDATE rullst_jobs SET status = 'pending', error = ?, available_at_ms = ?, updated_at = datetime('now') WHERE id = ? AND status = 'processing' AND attempts = COALESCE(?, attempts)",
        )
        .bind(reason)
        .bind(available_at_ms)
        .bind(job_id)
        .bind(attempt)
        .execute(&self.pool)
        .await
        .map_err(|error| QueueError::StateTransition {
            job_id: job_id.to_string(),
            operation: "requeue",
            message: error.to_string(),
        })?;
        ensure_claim_transition(result.rows_affected(), job_id, "requeue", attempt)
    }
}

#[async_trait]
impl QueueDriver for SqliteDriver {
    /// Stores the enqueue time as the job's due time, so claims follow the
    /// effective due time: a scheduled or deferred job that became due before
    /// an immediate job was pushed is claimed first.
    async fn push(&self, id: &str, job_name: &str, payload: &str) -> Result<(), QueueError> {
        sqlx::query(
            "INSERT INTO rullst_jobs (id, name, payload, available_at_ms) VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(job_name)
        .bind(payload)
        .bind(current_unix_millis()?)
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
        self.claim(None).await
    }

    async fn pop_with_lease(&self, lease: Duration) -> Result<Option<QueuedJob>, QueueError> {
        self.claim(Some(lease)).await
    }

    async fn mark_complete(&self, job_id: &str) -> Result<(), QueueError> {
        self.complete_claim(job_id, None).await
    }

    async fn mark_failed(&self, job_id: &str, error: &str) -> Result<(), QueueError> {
        self.fail_claim(job_id, None, error).await
    }

    async fn requeue(&self, job_id: &str, reason: &str) -> Result<(), QueueError> {
        self.requeue_claim(job_id, None, reason, 0).await
    }

    async fn mark_complete_attempt(&self, job_id: &str, attempt: u32) -> Result<(), QueueError> {
        self.complete_claim(job_id, Some(attempt)).await
    }

    async fn mark_failed_attempt(
        &self,
        job_id: &str,
        attempt: u32,
        error: &str,
    ) -> Result<(), QueueError> {
        self.fail_claim(job_id, Some(attempt), error).await
    }

    async fn requeue_attempt(
        &self,
        job_id: &str,
        attempt: u32,
        reason: &str,
    ) -> Result<(), QueueError> {
        self.requeue_claim(job_id, Some(attempt), reason, 0).await
    }

    async fn requeue_attempt_after(
        &self,
        job_id: &str,
        attempt: u32,
        reason: &str,
        delay: Duration,
    ) -> Result<(), QueueError> {
        let due = SystemTime::now().checked_add(delay).ok_or_else(|| {
            QueueError::InvalidConfiguration("requeue delay exceeds the clock range".to_string())
        })?;
        let available_at_ms = i64::try_from(unix_timestamp_millis_ceil(due)?).map_err(|_| {
            QueueError::InvalidConfiguration(
                "requeue timestamp exceeds SQLite integer range".to_string(),
            )
        })?;
        self.requeue_claim(job_id, Some(attempt), reason, available_at_ms)
            .await
    }

    async fn recover_stalled(&self, stale_after: Duration) -> Result<u64, QueueError> {
        recovery::recover_stalled(&self.pool, stale_after, self.max_stalled_leases).await
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

/// Current Unix time in whole milliseconds, rounded down.
fn current_unix_millis() -> Result<i64, QueueError> {
    i64::try_from(unix_timestamp_millis_floor(SystemTime::now())?).map_err(|_| {
        QueueError::Driver("current timestamp exceeds SQLite integer range".to_string())
    })
}

/// Like [`ensure_transition`], naming the claim attempt when a fenced
/// transition matched nothing, which usually means a stale lease.
fn ensure_claim_transition(
    rows_affected: u64,
    job_id: &str,
    operation: &'static str,
    attempt: Option<i64>,
) -> Result<(), QueueError> {
    match attempt {
        Some(attempt) if rows_affected != 1 => Err(QueueError::StateTransition {
            job_id: job_id.to_string(),
            operation,
            message: format!(
                "expected one processing job at claim attempt {attempt}, affected {rows_affected}; \
                 the lease may have been recovered and claimed again"
            ),
        }),
        _ => ensure_transition(rows_affected, job_id, operation),
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
