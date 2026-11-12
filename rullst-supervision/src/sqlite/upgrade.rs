//! Opt-in rebuild of the events table of stores created before
//! 13.0.0-alpha.5. Opening never changes a store; this method does, once.
use super::{SqliteSupervision, establish, schema, storage};
use crate::{Clock, SupervisionError as Error};

/// Rebuild steps, in order: the old table (and its index) is renamed away,
/// the new table is created from the exact current definition, every row is
/// copied unchanged, then the old table goes and the index is recreated.
const REBUILD: &[&str] = &[
    "ALTER TABLE rullst_supervision_events RENAME TO rullst_supervision_events_legacy",
    schema::EVENTS,
    "INSERT INTO rullst_supervision_events (session_id,sequence,kind,source,adapter_id,adapter_version,received_at,expires_at) SELECT session_id,sequence,kind,source,adapter_id,adapter_version,received_at,expires_at FROM rullst_supervision_events_legacy",
    "DROP TABLE rullst_supervision_events_legacy",
    schema::EVENT_INDEX,
];

impl<C: Clock> SqliteSupervision<C> {
    /// Lets a store created before 13.0.0-alpha.5 record
    /// `PresenceObservation::Obscured` (v13): rebuilds its events table, in
    /// one transaction, with the wider range of observation codes. Rows,
    /// sessions, configuration and retention are unchanged. Returns `true`
    /// when it rebuilt the table and `false` when the store was already
    /// current. A rebuilt store no longer opens with 13.0.0-alpha.4 or
    /// earlier, so upgrade every process sharing the file first and keep a
    /// backup. Other stores are refused with `Configuration`.
    pub async fn upgrade_event_schema(&self) -> Result<bool, Error> {
        let mut tx = establish::retry(|| self.pool.begin_with("BEGIN IMMEDIATE"))
            .await
            .map_err(storage)?;
        let definition: Option<String> = sqlx::query_scalar(
            "SELECT substr(sql,1,2049) FROM sqlite_schema WHERE type='table' AND name=?",
        )
        .bind(schema::EVENTS_TABLE)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?;
        match definition.as_deref() {
            Some(current) if current == schema::EVENTS => {
                tx.commit().await.map_err(|_| Error::UncertainCommit)?;
                return Ok(false);
            }
            Some(legacy) if legacy == schema::LEGACY_EVENTS => {}
            _ => return Err(Error::Configuration),
        }
        for statement in REBUILD {
            sqlx::query(*statement)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
        }
        tx.commit().await.map_err(|_| Error::UncertainCommit)?;
        self.validate_schema().await?;
        Ok(true)
    }
}
