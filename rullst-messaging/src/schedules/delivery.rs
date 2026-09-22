use super::*;
use sqlx::{Row, postgres::PgRow};
use subtle::ConstantTimeEq;

impl<C: Clock> PostgresRecurringStore<C> {
    pub(super) async fn expire(&self, tx: &mut Transaction<'_, Postgres>, now: i64) -> Result<()> {
        sqlx::query("UPDATE rullst_recurring_occurrences SET state='dead',terminal_at=$1,
            lease_hash=NULL,lease_until=NULL,content=CASE WHEN expires <= $1 THEN NULL ELSE content END
            WHERE namespace=$2 AND state IN ('pending','leased') AND
            (expires <= $1 OR (attempts >= $3 AND (state='pending' OR lease_until <= $1)))")
            .bind(now).bind(self.config.namespace()).bind(MAX_ATTEMPTS).execute(&mut **tx).await.map_err(|_| RecurringError::Storage)?;
        // Expired terminal retries no longer need their sensitive content either.
        sqlx::query(
            "UPDATE rullst_recurring_occurrences SET content=NULL
            WHERE namespace=$1 AND state='dead' AND expires <= $2 AND content IS NOT NULL",
        )
        .bind(self.config.namespace())
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(|_| RecurringError::Storage)?;
        Ok(())
    }
    /// Claims pending/abandoned work with a fresh, single-use, fenced capability.
    /// A lease may be shorter than configured at the end of its delivery window.
    pub async fn claim(&self, limit: usize) -> Result<Vec<OccurrenceLease>> {
        let limit = tick::batch(limit)?;
        bounded(async {
            let (mut tx, now) = self.begin().await?;
            self.expire(&mut tx, now).await?;
            let rows = sqlx::query("SELECT id,schedule,due,created,expires,attempts,state,version
                FROM rullst_recurring_occurrences WHERE namespace=$1 AND expires > $2 AND attempts < $3
                AND ((state='pending' AND available <= $2) OR (state='leased' AND lease_until <= $2))
                ORDER BY due,id LIMIT $4")
                .bind(self.config.namespace()).bind(now).bind(MAX_ATTEMPTS).bind(limit)
                .fetch_all(&mut *tx).await.map_err(|_| RecurringError::Storage)?;
            let mut output = Vec::new();
            for row in rows {
                let mut metadata = inspection::occurrence(&row)?;
                let previous: i64 = row.try_get("version").map_err(|_| RecurringError::Storage)?;
                let version = previous.checked_add(1).ok_or(RecurringError::Configuration)?;
                let expires = (now + self.config.lease_ms).min(metadata.expires);
                let token = crypto::random_token()?;
                let hash = crypto::lease_hash(self.config.namespace(), &metadata.id, version, &token);
                sqlx::query("UPDATE rullst_recurring_occurrences SET state='leased',version=$1,attempts=attempts+1,
                    lease_hash=$2,lease_until=$3 WHERE namespace=$4 AND id=$5")
                    .bind(version).bind(hash).bind(expires).bind(self.config.namespace()).bind(&metadata.id)
                    .execute(&mut *tx).await.map_err(|_| RecurringError::Storage)?;
                metadata.attempts += 1; metadata.state = OccurrenceState::Leased;
                output.push(OccurrenceLease { namespace: self.config.namespace().to_owned(), token, version, expires, metadata });
            }
            let deadline = output.iter().map(|lease| lease.expires).min();
            self.commit(tx, now, deadline).await?; Ok(output)
        }).await
    }
    pub(super) async fn verify(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        now: i64,
        lease: &OccurrenceLease,
    ) -> Result<PgRow> {
        if lease.namespace != self.config.namespace() || now >= lease.expires {
            return Err(RecurringError::InvalidLease);
        }
        let row = sqlx::query(
            "SELECT state,version,lease_hash,lease_until,expires,content,attempts
            FROM rullst_recurring_occurrences WHERE namespace=$1 AND id=$2",
        )
        .bind(self.config.namespace())
        .bind(&lease.metadata.id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| RecurringError::Storage)?
        .ok_or(RecurringError::InvalidLease)?;
        let state: String = row.try_get("state").map_err(|_| RecurringError::Storage)?;
        let version: i64 = row
            .try_get("version")
            .map_err(|_| RecurringError::Storage)?;
        let hash: Option<Vec<u8>> = row
            .try_get("lease_hash")
            .map_err(|_| RecurringError::Storage)?;
        let until: Option<i64> = row
            .try_get("lease_until")
            .map_err(|_| RecurringError::Storage)?;
        let expires: i64 = row
            .try_get("expires")
            .map_err(|_| RecurringError::Storage)?;
        let expected = crypto::lease_hash(
            self.config.namespace(),
            &lease.metadata.id,
            lease.version,
            &lease.token,
        );
        if state != "leased"
            || version != lease.version
            || until != Some(lease.expires)
            || now >= expires
            || !hash.is_some_and(|stored| bool::from(stored.ct_eq(&expected)))
        {
            return Err(RecurringError::InvalidLease);
        }
        Ok(row)
    }
    pub(super) async fn finish(&self, lease: &OccurrenceLease, success: bool) -> Result<()> {
        bounded(async {
            let (mut tx, now) = self.begin().await?;
            let row = self.verify(&mut tx, now, lease).await?;
            let attempts: i64 = row
                .try_get("attempts")
                .map_err(|_| RecurringError::Storage)?;
            let expires: i64 = row
                .try_get("expires")
                .map_err(|_| RecurringError::Storage)?;
            let retry_at = now + (1000_i64 << attempts.clamp(0, 9));
            let state = if success {
                "published"
            } else if attempts >= MAX_ATTEMPTS || retry_at >= expires {
                "dead"
            } else {
                "pending"
            };
            let terminal = if state == "pending" { None } else { Some(now) };
            sqlx::query(
                "UPDATE rullst_recurring_occurrences SET state=$1,available=$2,terminal_at=$3,
                lease_hash=NULL,lease_until=NULL,content=CASE WHEN $4 THEN NULL ELSE content END
                WHERE namespace=$5 AND id=$6",
            )
            .bind(state)
            .bind(retry_at)
            .bind(terminal)
            .bind(success)
            .bind(self.config.namespace())
            .bind(&lease.metadata.id)
            .execute(&mut *tx)
            .await
            .map_err(|_| RecurringError::Storage)?;
            self.commit(tx, now, Some(lease.expires.min(expires))).await
        })
        .await
    }
}
