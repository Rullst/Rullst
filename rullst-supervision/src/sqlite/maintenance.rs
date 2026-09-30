use super::{SqliteSupervision, storage, transaction::Operation};
use crate::{Clock, Operator, SupervisionError as Error};

/// Rows past retention that one admission removes when a ceiling is reached.
const SWEEP_BATCH: i64 = 32;

/// Logical row counts only. Deletion does not erase pages, WAL or backups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PurgeReceipt {
    pub events: u64,
    pub sessions: u64,
    pub grants: u64,
}

impl<C: Clock> SqliteSupervision<C> {
    /// Deletes at most `limit` records in the operator's tenant, events first.
    /// Session rows are eligible only after every event is removed. Active
    /// parental enrollment/policies and unexpired authority are never evicted.
    pub async fn purge_expired(
        &self,
        operator: &Operator,
        limit: u32,
    ) -> Result<PurgeReceipt, Error> {
        if !(1..=1000).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        let mut op = self.begin().await?;
        let tenant = operator.context().tenant().as_str();
        let events = sqlx::query("DELETE FROM rullst_supervision_events WHERE rowid IN (SELECT e.rowid FROM rullst_supervision_events e JOIN rullst_supervision_sessions s ON s.id=e.session_id WHERE s.tenant=? AND e.expires_at<=? ORDER BY e.expires_at,e.rowid LIMIT ?)")
            .bind(tenant).bind(op.now).bind(limit).execute(&mut *op.tx).await.map_err(storage)?.rows_affected();
        let remaining = u64::from(limit)
            .checked_sub(events)
            .ok_or(Error::Configuration)?;
        let sessions = sqlx::query("DELETE FROM rullst_supervision_sessions WHERE id IN (SELECT s.id FROM rullst_supervision_sessions s WHERE s.tenant=? AND s.retain_until<=? AND NOT EXISTS (SELECT 1 FROM rullst_supervision_events e WHERE e.session_id=s.id) ORDER BY s.retain_until,s.id LIMIT ?)")
            .bind(tenant).bind(op.now).bind(i64::try_from(remaining).map_err(|_| Error::Configuration)?)
            .execute(&mut *op.tx).await.map_err(storage)?.rows_affected();
        let remaining = remaining
            .checked_sub(sessions)
            .ok_or(Error::Configuration)?;
        let grants = sqlx::query("DELETE FROM rullst_supervision_grants WHERE rowid IN (SELECT rowid FROM rullst_supervision_grants WHERE tenant=? AND expires_at<=? ORDER BY expires_at,rowid LIMIT ?)")
            .bind(tenant).bind(op.now).bind(i64::try_from(remaining).map_err(|_| Error::Configuration)?)
            .execute(&mut *op.tx).await.map_err(storage)?.rows_affected();
        if events + sessions + grants > 0 {
            op.next_revision()?;
        }
        op.finish().await?;
        Ok(PurgeReceipt {
            events,
            sessions,
            grants,
        })
    }
}

impl<C: Clock> Operation<'_, C> {
    /// Removes a bounded batch of sessions already past retention, in any
    /// tenant, so logically deleted rows never block a new session. Their
    /// events expired before `retain_until` and cascade with them.
    pub(super) async fn sweep_retired_sessions(&mut self) -> Result<i64, Error> {
        let removed = sqlx::query("DELETE FROM rullst_supervision_sessions WHERE id IN (SELECT id FROM rullst_supervision_sessions WHERE retain_until<=? ORDER BY retain_until,id LIMIT ?)")
            .bind(self.now).bind(SWEEP_BATCH)
            .execute(&mut *self.tx).await.map_err(storage)?.rows_affected();
        i64::try_from(removed).map_err(|_| Error::Configuration)
    }

    /// Removes a bounded batch of already expired events, which reads exclude.
    pub(super) async fn sweep_expired_events(&mut self) -> Result<i64, Error> {
        let removed = sqlx::query("DELETE FROM rullst_supervision_events WHERE rowid IN (SELECT rowid FROM rullst_supervision_events WHERE expires_at<=? ORDER BY expires_at,rowid LIMIT ?)")
            .bind(self.now).bind(SWEEP_BATCH)
            .execute(&mut *self.tx).await.map_err(storage)?.rows_affected();
        i64::try_from(removed).map_err(|_| Error::Configuration)
    }
}
