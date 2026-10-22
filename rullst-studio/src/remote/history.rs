//! The bounded local history of monitored values and fired alerts.
//!
//! An owner-only SQLite file in the user's data directory (`0700`), holding
//! only aggregates the monitoring document already exposes: reachability,
//! request rate, window requests, 5xx rate, p95 latency, queue pending and
//! failed totals. No token, route or other label is stored. Every write
//! deletes samples older than the retention age and beyond the per-project
//! row limit, and keeps a bounded number of alerts.

use super::client::FetchError;
use super::document::{MonitorDocument, Pending};
use super::secure_file;
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::collections::HashMap;
use std::path::Path;

/// How much history is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retention {
    /// Samples older than this many seconds are deleted.
    pub max_age_seconds: u64,
    /// At most this many samples per project.
    pub max_samples_per_project: u32,
    /// At most this many alerts in total.
    pub max_alerts: u32,
}

impl Default for Retention {
    fn default() -> Self {
        Self {
            max_age_seconds: 7 * 24 * 3_600,
            max_samples_per_project: 10_080,
            max_alerts: 500,
        }
    }
}

/// One poll of one project.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sample {
    pub(crate) at: u64,
    pub(crate) reachable: bool,
    pub(crate) requests_per_second: Option<f64>,
    pub(crate) window_requests: Option<u64>,
    pub(crate) error_rate_percent: Option<f64>,
    pub(crate) p95_ms: Option<f64>,
    pub(crate) queue_pending: Option<u64>,
    pub(crate) queue_failed_total: Option<u64>,
}

fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

impl Sample {
    /// The aggregates of a read at `at`; an unreachable sample has no values.
    pub(crate) fn from_outcome(at: u64, outcome: &Result<MonitorDocument, FetchError>) -> Self {
        let Ok(document) = outcome else {
            return Self {
                at,
                reachable: false,
                requests_per_second: None,
                window_requests: None,
                error_rate_percent: None,
                p95_ms: None,
                queue_pending: None,
                queue_failed_total: None,
            };
        };
        let window = &document.http.window;
        let error_rate_percent = (window.requests > 0)
            .then(|| window.status.server_error as f64 * 100.0 / window.requests as f64);
        let pending = document
            .queues
            .iter()
            .map(|queue| match queue.pending {
                Pending::Observed { count } => Some(count),
                Pending::Unavailable { .. } => None,
            })
            .try_fold(0_u64, |total, count| {
                count.map(|count| total.saturating_add(count))
            });
        let has_queues = !document.queues.is_empty();
        Self {
            at,
            reachable: true,
            requests_per_second: finite(window.requests_per_second),
            window_requests: Some(window.requests),
            error_rate_percent: error_rate_percent.and_then(finite),
            p95_ms: window.latency_ms.and_then(|latency| finite(latency.p95)),
            queue_pending: pending.filter(|_| has_queues),
            queue_failed_total: has_queues.then(|| {
                document.queues.iter().fold(0_u64, |total, queue| {
                    total.saturating_add(queue.failed_total)
                })
            }),
        }
    }
}

/// One fired alert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AlertRow {
    pub(crate) id: i64,
    pub(crate) project: String,
    pub(crate) at: u64,
    pub(crate) rule: String,
    pub(crate) message: String,
    pub(crate) delivery: String,
    pub(crate) acknowledged: bool,
}

/// The history database could not be used. Carries no SQL or path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HistoryError;

impl From<sqlx::Error> for HistoryError {
    fn from(_: sqlx::Error) -> Self {
        Self
    }
}

const SCHEMA: [&str; 3] = [
    "CREATE TABLE IF NOT EXISTS samples (
        project TEXT NOT NULL,
        at INTEGER NOT NULL,
        reachable INTEGER NOT NULL,
        requests_per_second REAL,
        window_requests INTEGER,
        error_rate_percent REAL,
        p95_ms REAL,
        queue_pending INTEGER,
        queue_failed_total INTEGER,
        PRIMARY KEY (project, at)
    )",
    "CREATE TABLE IF NOT EXISTS alerts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        project TEXT NOT NULL,
        at INTEGER NOT NULL,
        rule TEXT NOT NULL,
        message TEXT NOT NULL,
        delivery TEXT NOT NULL,
        acknowledged INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE INDEX IF NOT EXISTS alerts_by_rule ON alerts (project, rule, at)",
];

fn int(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn uint(value: Option<i64>) -> Option<u64> {
    value.and_then(|value| u64::try_from(value).ok())
}

/// The history database.
pub(crate) struct History {
    pool: SqlitePool,
    retention: Retention,
}

impl History {
    /// Opens (creating owner-only when absent) the database at `path`.
    pub(crate) async fn open(path: &Path, retention: Retention) -> Result<Self, HistoryError> {
        if std::fs::symlink_metadata(path).is_err() {
            secure_file::write_private(path, &[]).map_err(|_| HistoryError)?;
        } else if let Some(directory) = path.parent() {
            secure_file::create_private_dir(directory).map_err(|_| HistoryError)?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(false);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        for statement in SCHEMA {
            sqlx::query(statement).execute(&pool).await?;
        }
        Ok(Self { pool, retention })
    }

    /// Stores `sample` for `project` and applies the retention.
    pub(crate) async fn record(&self, project: &str, sample: &Sample) -> Result<(), HistoryError> {
        sqlx::query(
            "INSERT OR REPLACE INTO samples (project, at, reachable, requests_per_second,
                window_requests, error_rate_percent, p95_ms, queue_pending, queue_failed_total)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(project)
        .bind(int(sample.at))
        .bind(sample.reachable)
        .bind(sample.requests_per_second)
        .bind(sample.window_requests.map(int))
        .bind(sample.error_rate_percent)
        .bind(sample.p95_ms)
        .bind(sample.queue_pending.map(int))
        .bind(sample.queue_failed_total.map(int))
        .execute(&self.pool)
        .await?;
        sqlx::query("DELETE FROM samples WHERE at < ?1")
            .bind(int(sample
                .at
                .saturating_sub(self.retention.max_age_seconds)))
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "DELETE FROM samples WHERE project = ?1 AND at <= (
                SELECT at FROM samples WHERE project = ?1 ORDER BY at DESC LIMIT 1 OFFSET ?2)",
        )
        .bind(project)
        .bind(i64::from(self.retention.max_samples_per_project))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Samples of `project` since `since`, oldest first.
    pub(crate) async fn samples(
        &self,
        project: &str,
        since: u64,
    ) -> Result<Vec<Sample>, HistoryError> {
        let rows = sqlx::query(
            "SELECT at, reachable, requests_per_second, window_requests, error_rate_percent,
                    p95_ms, queue_pending, queue_failed_total
             FROM samples WHERE project = ?1 AND at >= ?2 ORDER BY at",
        )
        .bind(project)
        .bind(int(since))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(Sample {
                    at: uint(Some(row.try_get("at")?)).unwrap_or(0),
                    reachable: row.try_get("reachable")?,
                    requests_per_second: row.try_get("requests_per_second")?,
                    window_requests: uint(row.try_get("window_requests")?),
                    error_rate_percent: row.try_get("error_rate_percent")?,
                    p95_ms: row.try_get("p95_ms")?,
                    queue_pending: uint(row.try_get("queue_pending")?),
                    queue_failed_total: uint(row.try_get("queue_failed_total")?),
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(HistoryError::from)
    }

    /// The newest sample of `project` taken before `before`.
    pub(crate) async fn previous(
        &self,
        project: &str,
        before: u64,
    ) -> Result<Option<Sample>, HistoryError> {
        let since = before.saturating_sub(self.retention.max_age_seconds);
        let samples = self.samples(project, since).await?;
        Ok(samples.into_iter().rev().find(|sample| sample.at < before))
    }

    /// When each rule of `project` last fired.
    pub(crate) async fn last_fired(
        &self,
        project: &str,
    ) -> Result<HashMap<String, u64>, HistoryError> {
        let rows =
            sqlx::query("SELECT rule, MAX(at) AS at FROM alerts WHERE project = ?1 GROUP BY rule")
                .bind(project)
                .fetch_all(&self.pool)
                .await?;
        rows.into_iter()
            .map(|row| {
                let rule: String = row.try_get("rule")?;
                let at: i64 = row.try_get("at")?;
                Ok((rule, uint(Some(at)).unwrap_or(0)))
            })
            .collect::<Result<HashMap<_, _>, sqlx::Error>>()
            .map_err(HistoryError::from)
    }

    /// Stores an alert and keeps at most the configured number.
    pub(crate) async fn record_alert(
        &self,
        project: &str,
        at: u64,
        rule: &str,
        message: &str,
        delivery: &str,
    ) -> Result<i64, HistoryError> {
        let id = sqlx::query(
            "INSERT INTO alerts (project, at, rule, message, delivery) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(project)
        .bind(int(at))
        .bind(rule)
        .bind(message)
        .bind(delivery)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        sqlx::query(
            "DELETE FROM alerts WHERE id <= (SELECT id FROM alerts ORDER BY id DESC LIMIT 1 OFFSET ?1)",
        )
        .bind(i64::from(self.retention.max_alerts))
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    /// Records how the alert `id` was delivered.
    pub(crate) async fn set_delivery(&self, id: i64, delivery: &str) -> Result<(), HistoryError> {
        sqlx::query("UPDATE alerts SET delivery = ?1 WHERE id = ?2")
            .bind(delivery)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// The newest `limit` alerts, newest first.
    pub(crate) async fn alerts(&self, limit: u32) -> Result<Vec<AlertRow>, HistoryError> {
        let rows = sqlx::query(
            "SELECT id, project, at, rule, message, delivery, acknowledged
             FROM alerts ORDER BY id DESC LIMIT ?1",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(AlertRow {
                    id: row.try_get("id")?,
                    project: row.try_get("project")?,
                    at: uint(Some(row.try_get("at")?)).unwrap_or(0),
                    rule: row.try_get("rule")?,
                    message: row.try_get("message")?,
                    delivery: row.try_get("delivery")?,
                    acknowledged: row.try_get("acknowledged")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(HistoryError::from)
    }

    /// Alerts not acknowledged yet.
    pub(crate) async fn unacknowledged(&self) -> Result<u64, HistoryError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM alerts WHERE acknowledged = 0")
            .fetch_one(&self.pool)
            .await?;
        Ok(uint(Some(count)).unwrap_or(0))
    }

    /// Marks every alert as acknowledged.
    pub(crate) async fn acknowledge_all(&self) -> Result<(), HistoryError> {
        sqlx::query("UPDATE alerts SET acknowledged = 1")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn pool_for_tests(&self) -> &SqlitePool {
        &self.pool
    }

    /// Deletes the samples and alerts of a removed project.
    pub(crate) async fn forget(&self, project: &str) -> Result<(), HistoryError> {
        sqlx::query("DELETE FROM samples WHERE project = ?1")
            .bind(project)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM alerts WHERE project = ?1")
            .bind(project)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
