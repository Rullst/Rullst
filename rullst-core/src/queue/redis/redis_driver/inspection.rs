//! Bounded job listing, failed-job retry and failure purge for the Redis
//! queue driver.

use super::super::scripts::RETRY_FAILED_SCRIPT;
use super::RedisDriver;
use crate::queue::{QueueError, QueuedJobDetail};
use serde::Deserialize;

/// Upper bound on the rows one `list_all_jobs` call returns.
pub(super) const MAX_LISTED_JOBS: usize = 1_000;

impl RedisDriver {
    /// Returns at most `limit` (capped at [`MAX_LISTED_JOBS`]) jobs: failed
    /// jobs and dead letters (newest first), then processing, pending and
    /// scheduled jobs. The reads are not one atomic snapshot. Redis does not
    /// record creation time, so `created_at` is empty; `updated_at` is the
    /// failure, claim or due time when one is known.
    pub(super) async fn list_jobs(&self, limit: u32) -> Result<Vec<QueuedJobDetail>, QueueError> {
        let limit = usize::try_from(limit)
            .unwrap_or(MAX_LISTED_JOBS)
            .min(MAX_LISTED_JOBS);
        if limit == 0 {
            return Ok(Vec::new());
        }
        let last = isize::try_from(limit).unwrap_or(isize::MAX) - 1;
        let read_error = |error: redis::RedisError| {
            QueueError::Driver(format!("Failed to list Redis jobs: {error}"))
        };
        let mut connection = self.connection().await?;

        let failed: Vec<(String, f64)> = redis::cmd("ZREVRANGE")
            .arg(&self.failed_index_key)
            .arg(0)
            .arg(last)
            .arg("WITHSCORES")
            .query_async(&mut connection)
            .await
            .map_err(read_error)?;
        let failure_entries: Vec<Option<String>> = if failed.is_empty() {
            Vec::new()
        } else {
            redis::cmd("HMGET")
                .arg(&self.failed_key)
                .arg(failed.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>())
                .query_async(&mut connection)
                .await
                .map_err(read_error)?
        };
        let dead_letters: Vec<String> = redis::cmd("LRANGE")
            .arg(&self.dead_letter_key)
            .arg(-(last + 1))
            .arg(-1)
            .query_async(&mut connection)
            .await
            .map_err(read_error)?;
        let processing: Vec<(String, f64)> = redis::cmd("ZRANGE")
            .arg(&self.processing_key)
            .arg(0)
            .arg(last)
            .arg("WITHSCORES")
            .query_async(&mut connection)
            .await
            .map_err(read_error)?;
        let pending: Vec<String> = redis::cmd("LRANGE")
            .arg(&self.queue_key)
            .arg(0)
            .arg(last)
            .query_async(&mut connection)
            .await
            .map_err(read_error)?;
        let scheduled: Vec<(String, f64)> = redis::cmd("ZRANGE")
            .arg(&self.scheduled_key)
            .arg(0)
            .arg(last)
            .arg("WITHSCORES")
            .query_async(&mut connection)
            .await
            .map_err(read_error)?;

        let failures = failed
            .iter()
            .zip(failure_entries)
            .filter_map(|((_, failed_at), entry)| {
                entry.map(|entry| failure_detail(&entry, "failed", Some(*failed_at)))
            });
        let dead = dead_letters
            .iter()
            .rev()
            .map(|entry| failure_detail(entry, "dead-letter", None));
        let leased = processing
            .iter()
            .map(|(raw, claimed_at)| job_detail(raw, "processing", None, Some(*claimed_at)));
        let waiting = pending
            .iter()
            .map(|raw| job_detail(raw, "pending", None, None));
        let due = scheduled
            .iter()
            .map(|(raw, due_at)| job_detail(raw, "pending", None, Some(*due_at)));
        Ok(failures
            .chain(dead)
            .chain(leased)
            .chain(waiting)
            .chain(due)
            .take(limit)
            .collect())
    }

    /// Moves a failed job back to the tail of the pending list, keeping its
    /// attempt counter so older leases stay fenced.
    pub(super) async fn retry_failed(&self, job_id: &str) -> Result<(), QueueError> {
        let mut connection = self.connection().await?;
        let changed: i64 = redis::cmd("EVAL")
            .arg(RETRY_FAILED_SCRIPT)
            .arg(3)
            .arg(&self.failed_key)
            .arg(&self.failed_index_key)
            .arg(&self.queue_key)
            .arg(job_id)
            .query_async(&mut connection)
            .await
            .map_err(|error| QueueError::StateTransition {
                job_id: job_id.to_string(),
                operation: "retry_failed",
                message: error.to_string(),
            })?;
        match changed {
            1 => Ok(()),
            -1 => Err(QueueError::StateTransition {
                job_id: job_id.to_string(),
                operation: "retry_failed",
                message: "the failed job entry is not a valid failure record".to_string(),
            }),
            _ => Err(QueueError::StateTransition {
                job_id: job_id.to_string(),
                operation: "retry_failed",
                message: format!("expected one failed job, affected {changed}"),
            }),
        }
    }

    /// Deletes every failed job, the failure index and every dead letter.
    pub(super) async fn purge_failures(&self) -> Result<(), QueueError> {
        let mut connection = self.connection().await?;
        redis::cmd("DEL")
            .arg(&self.failed_key)
            .arg(&self.failed_index_key)
            .arg(&self.dead_letter_key)
            .query_async::<i64>(&mut connection)
            .await
            .map_err(|error| {
                QueueError::Driver(format!("Failed to purge Redis failed jobs: {error}"))
            })?;
        Ok(())
    }
}

/// Envelope fields read leniently, so a malformed dead letter still lists.
#[derive(Deserialize)]
struct ListedEnvelope {
    id: Option<String>,
    name: Option<String>,
    payload: Option<String>,
    attempts: Option<u64>,
}

#[derive(Deserialize)]
struct FailureEntry {
    raw: String,
    error: Option<String>,
}

fn failure_detail(entry: &str, status: &str, updated_at_ms: Option<f64>) -> QueuedJobDetail {
    match serde_json::from_str::<FailureEntry>(entry) {
        Ok(failure) => job_detail(&failure.raw, status, failure.error, updated_at_ms),
        Err(_) => job_detail(entry, status, None, updated_at_ms),
    }
}

fn job_detail(
    raw: &str,
    status: &str,
    error: Option<String>,
    updated_at_ms: Option<f64>,
) -> QueuedJobDetail {
    let envelope = serde_json::from_str::<ListedEnvelope>(raw).ok();
    let (id, name, payload, attempts) = match envelope {
        Some(envelope) => (
            envelope.id.unwrap_or_default(),
            envelope.name.unwrap_or_default(),
            envelope.payload.unwrap_or_default(),
            envelope.attempts.unwrap_or(0),
        ),
        None => (String::new(), String::new(), raw.to_string(), 0),
    };
    QueuedJobDetail {
        id,
        name,
        payload,
        status: status.to_string(),
        error,
        attempts: i32::try_from(attempts).unwrap_or(i32::MAX),
        created_at: String::new(),
        updated_at: updated_at_ms.map(format_millis).unwrap_or_default(),
    }
}

fn format_millis(millis: f64) -> String {
    // Scores are integral milliseconds; `as` saturates out-of-range values.
    chrono::DateTime::from_timestamp_millis(millis as i64)
        .map(|time| time.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_entries_expose_the_envelope_error_and_failure_time() {
        let entry = r#"{"raw":"{\"id\":\"job-1\",\"name\":\"mail\",\"payload\":\"{\\\"to\\\":1}\",\"attempts\":3}","error":"smtp down"}"#;
        let detail = failure_detail(entry, "failed", Some(1_700_000_000_123.0));
        assert_eq!(detail.id, "job-1");
        assert_eq!(detail.name, "mail");
        assert_eq!(detail.payload, r#"{"to":1}"#);
        assert_eq!(detail.status, "failed");
        assert_eq!(detail.error.as_deref(), Some("smtp down"));
        assert_eq!(detail.attempts, 3);
        assert!(detail.created_at.is_empty());
        assert_eq!(detail.updated_at, "2023-11-14T22:13:20.123Z");
    }

    #[test]
    fn malformed_entries_are_listed_verbatim_without_panicking() {
        let detail = failure_detail(
            r#"{"raw":"not-json","error":"invalid"}"#,
            "dead-letter",
            None,
        );
        assert_eq!(detail.id, "");
        assert_eq!(detail.payload, "not-json");
        assert_eq!(detail.error.as_deref(), Some("invalid"));
        assert!(detail.updated_at.is_empty());

        let detail = failure_detail("garbage", "dead-letter", None);
        assert_eq!(detail.payload, "garbage");
        assert_eq!(detail.error, None);

        let detail = job_detail(
            r#"{"id":"x","attempts":99999999999}"#,
            "pending",
            None,
            None,
        );
        assert_eq!(detail.attempts, i32::MAX);
    }
}
