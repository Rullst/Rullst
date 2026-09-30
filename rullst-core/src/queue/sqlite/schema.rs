//! Idempotent creation and forward migration of the `rullst_jobs` table.

use crate::queue::QueueError;

/// Creates the table and indexes and adds columns missing from older tables.
pub(super) async fn prepare(pool: &sqlx::SqlitePool) -> Result<(), QueueError> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS rullst_jobs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            payload TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            error TEXT,
            attempts INTEGER NOT NULL DEFAULT 0,
            available_at_ms INTEGER NOT NULL DEFAULT 0,
            stalled_recoveries INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"#,
    )
    .execute(pool)
    .await
    .map_err(|e| QueueError::Driver(format!("Failed to create rullst_jobs table: {}", e)))?;

    add_missing_column(
        pool,
        "available_at_ms",
        "ALTER TABLE rullst_jobs ADD COLUMN available_at_ms INTEGER NOT NULL DEFAULT 0",
        "scheduling",
    )
    .await?;
    add_missing_column(
        pool,
        "stalled_recoveries",
        "ALTER TABLE rullst_jobs ADD COLUMN stalled_recoveries INTEGER NOT NULL DEFAULT 0",
        "stalled-lease counter",
    )
    .await?;

    // Add index for fast polling of pending jobs
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_rullst_jobs_status_created ON rullst_jobs(status, created_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| QueueError::Driver(format!("Failed to create rullst_jobs indexes: {}", e)))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_rullst_jobs_ready ON rullst_jobs(status, available_at_ms, created_at)",
    )
    .execute(pool)
    .await
    .map_err(|error| {
        QueueError::Driver(format!("Failed to create scheduled-job index: {error}"))
    })?;
    Ok(())
}

/// Runs the fixed `alter` statement when older tables lack `column`.
async fn add_missing_column(
    pool: &sqlx::SqlitePool,
    column: &'static str,
    alter: &'static str,
    purpose: &'static str,
) -> Result<(), QueueError> {
    let existing: Option<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('rullst_jobs') WHERE name = ?")
            .bind(column)
            .fetch_optional(pool)
            .await
            .map_err(|error| {
                QueueError::Driver(format!("Failed to inspect rullst_jobs columns: {error}"))
            })?;
    if existing.is_some() {
        return Ok(());
    }
    sqlx::query(alter).execute(pool).await.map_err(|error| {
        QueueError::Driver(format!(
            "Failed to add rullst_jobs {purpose} column: {error}"
        ))
    })?;
    Ok(())
}
