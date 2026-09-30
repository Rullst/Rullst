//! Stalled-lease recovery with a per-job ceiling.

use crate::queue::{QueueError, unix_timestamp_millis_floor};
use std::time::{Duration, SystemTime};

/// A processing lease is stalled once its recorded claim lease has passed,
/// or, for a claim without one, once it is older than the caller's age.
/// Binds: current Unix milliseconds, then the `datetime` age modifier.
macro_rules! stalled {
    () => {
        "status = 'processing' AND CASE WHEN lease_expires_at_ms > 0 \
         THEN lease_expires_at_ms <= ? ELSE updated_at <= datetime('now', ?) END"
    };
}

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

    let now_ms = i64::try_from(unix_timestamp_millis_floor(SystemTime::now())?).map_err(|_| {
        QueueError::Driver("current timestamp exceeds SQLite integer range".to_string())
    })?;

    let mut transaction = pool.begin().await.map_err(driver_error)?;
    let failed = sqlx::query(concat!(
        "UPDATE rullst_jobs SET status = 'failed', error = ?, updated_at = datetime('now') WHERE ",
        stalled!(),
        " AND stalled_recoveries + 1 >= ?"
    ))
    .bind(&reason)
    .bind(now_ms)
    .bind(&modifier)
    .bind(i64::from(max_stalled_leases))
    .execute(&mut *transaction)
    .await
    .map_err(driver_error)?;
    let requeued = sqlx::query(concat!(
        "UPDATE rullst_jobs SET status = 'pending', error = 'recovered after worker interruption', \
         available_at_ms = 0, stalled_recoveries = stalled_recoveries + 1, \
         lease_expires_at_ms = 0, updated_at = datetime('now') WHERE ",
        stalled!()
    ))
    .bind(now_ms)
    .bind(&modifier)
    .execute(&mut *transaction)
    .await
    .map_err(driver_error)?;
    transaction.commit().await.map_err(driver_error)?;
    Ok(failed
        .rows_affected()
        .saturating_add(requeued.rows_affected()))
}
