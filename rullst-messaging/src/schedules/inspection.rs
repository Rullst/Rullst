use super::*;
use sqlx::{Row, postgres::PgRow};

pub(super) fn occurrence(row: &PgRow) -> Result<OccurrenceMetadata> {
    let state: String = row.try_get("state").map_err(|_| RecurringError::Storage)?;
    let state = match state.as_str() {
        "pending" => OccurrenceState::Pending,
        "leased" => OccurrenceState::Leased,
        "published" => OccurrenceState::Published,
        "dead" => OccurrenceState::DeadLetter,
        "cancelled" => OccurrenceState::Cancelled,
        _ => return Err(RecurringError::Configuration),
    };
    let attempts: i64 = row
        .try_get("attempts")
        .map_err(|_| RecurringError::Storage)?;
    Ok(OccurrenceMetadata {
        id: row.try_get("id").map_err(|_| RecurringError::Storage)?,
        schedule: row
            .try_get("schedule")
            .map_err(|_| RecurringError::Storage)?,
        due: row.try_get("due").map_err(|_| RecurringError::Storage)?,
        created: row
            .try_get("created")
            .map_err(|_| RecurringError::Storage)?,
        expires: row
            .try_get("expires")
            .map_err(|_| RecurringError::Storage)?,
        attempts: u32::try_from(attempts).map_err(|_| RecurringError::Configuration)?,
        state,
    })
}
pub(super) fn id(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(RecurringError::InvalidInput("occurrence ID"));
    }
    Ok(())
}
impl<C: Clock> PostgresRecurringStore<C> {
    /// Metadata page ordered by schedule name, excluding the optional cursor.
    pub async fn schedules(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecurringMetadata>> {
        let limit = tick::batch(limit)?;
        if let Some(after) = after {
            management::name(after)?;
        }
        bounded(async {
            let (mut tx, now) = self.begin().await?;
            let rows = sqlx::query("SELECT name,generation,created,next_due,cancelled FROM rullst_recurring_definitions
                WHERE namespace=$1 AND ($2::TEXT IS NULL OR name > $2) ORDER BY name LIMIT $3")
                .bind(self.config.namespace()).bind(after).bind(limit).fetch_all(&mut *tx).await.map_err(|_| RecurringError::Storage)?;
            let output = rows.iter().map(management::metadata).collect::<Result<Vec<_>>>()?;
            self.commit(tx, now, None).await?; Ok(output)
        }).await
    }
    /// Metadata page ordered by occurrence ID, excluding the optional cursor.
    pub async fn occurrences(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<OccurrenceMetadata>> {
        let limit = tick::batch(limit)?;
        if let Some(after) = after {
            id(after)?;
        }
        bounded(async {
            let (mut tx, now) = self.begin().await?;
            self.expire(&mut tx, now).await?;
            let rows = sqlx::query("SELECT id,schedule,due,created,expires,attempts,state FROM rullst_recurring_occurrences
                WHERE namespace=$1 AND ($2::TEXT IS NULL OR id > $2) ORDER BY id LIMIT $3")
                .bind(self.config.namespace()).bind(after).bind(limit).fetch_all(&mut *tx).await.map_err(|_| RecurringError::Storage)?;
            let output = rows.iter().map(occurrence).collect::<Result<Vec<_>>>()?;
            self.commit(tx, now, None).await?; Ok(output)
        }).await
    }
    /// Explicit operator retry, preserving content/key and the original delivery window.
    /// Starts a new bounded attempt budget; never revives cancelled/published work.
    pub async fn retry_failed(&self, occurrence_id: impl Into<String>) -> Result<()> {
        let occurrence_id = occurrence_id.into();
        id(&occurrence_id)?;
        bounded(async {
            let (mut tx, now) = self.begin().await?;
            let expires: Option<i64> = sqlx::query_scalar(
                "UPDATE rullst_recurring_occurrences SET state='pending',attempts=0,
                available=$1,terminal_at=NULL WHERE namespace=$2 AND id=$3 AND state='dead'
                AND expires > $1 AND content IS NOT NULL RETURNING expires",
            )
            .bind(now)
            .bind(self.config.namespace())
            .bind(&occurrence_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| RecurringError::Storage)?;
            let expires = expires.ok_or(RecurringError::InvalidLease)?;
            self.commit(tx, now, Some(expires)).await
        })
        .await
    }
    /// Deletes at most 100 terminal occurrence rows before an explicit retention cutoff.
    /// Broker and consumer deduplication retention must exceed the delivery/retry window.
    /// Schedule definitions/names remain reserved until the namespace is retired by its owner.
    pub async fn purge_terminal(&self, before_ms: i64, limit: usize) -> Result<u64> {
        let limit = tick::batch(limit)?;
        bounded(async {
            let (mut tx, now) = self.begin().await?;
            if !(1..=now).contains(&before_ms) {
                return Err(RecurringError::InvalidInput("retention cutoff"));
            }
            self.expire(&mut tx, now).await?;
            let removed = sqlx::query(
                "DELETE FROM rullst_recurring_occurrences WHERE namespace=$1 AND id IN
                (SELECT id FROM rullst_recurring_occurrences WHERE namespace=$1 AND terminal_at < $2
                 AND state IN ('published','dead','cancelled') ORDER BY terminal_at,id LIMIT $3)",
            )
            .bind(self.config.namespace())
            .bind(before_ms)
            .bind(limit)
            .execute(&mut *tx)
            .await
            .map_err(|_| RecurringError::Storage)?
            .rows_affected();
            self.commit(tx, now, None).await?;
            Ok(removed)
        })
        .await
    }
}
