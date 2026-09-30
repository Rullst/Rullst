//! Stalled-lease recovery with a per-job ceiling.

use crate::queue::QueueError;
use std::time::Duration;

/// Returns stalled processing leases to pending, or fails a job whose lease
/// has now stalled `max_stalled_leases` times, and reports how many leases
/// left the processing state.
///
/// A job that crashes, aborts or hangs its worker is otherwise reclaimed and
/// recovered forever. Both updates commit in one transaction.
pub(super) async fn recover_stalled(
    pool: &sqlx::SqlitePool,
    stale_after: Duration,
    max_stalled_leases: u32,
) -> Result<u64, QueueError> {
    let driver_error =
        |error: sqlx::Error| QueueError::Driver(format!("Failed to recover stalled jobs: {error}"));
    let stale_seconds = stale_after.as_secs().max(1);
    let modifier = format!("-{stale_seconds} seconds");
    let reason = format!(
        "lease stalled {max_stalled_leases} times without finishing; failed instead of requeued"
    );

    let mut transaction = pool.begin().await.map_err(driver_error)?;
    let failed = sqlx::query(
        "UPDATE rullst_jobs SET status = 'failed', error = ?, updated_at = datetime('now') \
         WHERE status = 'processing' AND updated_at <= datetime('now', ?) \
         AND stalled_recoveries + 1 >= ?",
    )
    .bind(&reason)
    .bind(&modifier)
    .bind(i64::from(max_stalled_leases))
    .execute(&mut *transaction)
    .await
    .map_err(driver_error)?;
    let requeued = sqlx::query(
        "UPDATE rullst_jobs SET status = 'pending', error = 'recovered after worker interruption', \
         available_at_ms = 0, stalled_recoveries = stalled_recoveries + 1, updated_at = datetime('now') \
         WHERE status = 'processing' AND updated_at <= datetime('now', ?)",
    )
    .bind(&modifier)
    .execute(&mut *transaction)
    .await
    .map_err(driver_error)?;
    transaction.commit().await.map_err(driver_error)?;
    Ok(failed
        .rows_affected()
        .saturating_add(requeued.rows_affected()))
}
