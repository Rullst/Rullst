use super::*;
use sqlx::Row;

impl SuppressionStore for PostgresSuppressionStore {
    /// Reads authoritative state under the namespace lock; no positive cache.
    async fn lookup(&self, recipient: &str) -> Result<Option<SuppressionRecord>, SuppressionError> {
        bounded(async {
            let recipient = normalize_recipient(recipient)?;
            let (mut tx, current) = self.begin().await?;
            let result = self.fetch(&mut tx, &recipient, current).await?;
            self.commit(tx, current).await?;
            Ok(result)
        })
        .await
    }
}

impl MutableSuppressionStore for PostgresSuppressionStore {
    /// Atomically binds a verified event to its original recipient/reason/time.
    async fn record(&self, event: SuppressionEvent) -> Result<SuppressionRecord, SuppressionError> {
        bounded(async {
            let (mut tx, current) = self.begin().await?;
            let observed = i64::try_from(event.observed_at())
                .map_err(|_| SuppressionError::InvalidEvent("observation time"))?;
            if observed <= 0 || observed > current + 300 {
                return Err(SuppressionError::InvalidEvent("observation time"));
            }
            let event_tag = self.tag(
                b"event-identity",
                &[event.provider().as_bytes(), event.event_id().as_bytes()],
            )?;
            let fingerprint_parts = [
                event.recipient().as_bytes(),
                event.reason().as_str().as_bytes(),
                &observed.to_be_bytes(),
            ];
            let previous: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT fingerprint FROM rullst_mail_pg_suppression_events
                 WHERE namespace = $1 AND event_tag = $2",
            )
            .bind(&self.config.namespace)
            .bind(&event_tag)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| unavailable("lookup replay identity"))?;
            let existing = self.fetch(&mut tx, event.recipient(), current).await?;
            if let Some(previous) = previous {
                if !self.matches(b"event-contents", &fingerprint_parts, &previous)? {
                    return Err(SuppressionError::EventConflict);
                }
                let record = existing.ok_or(SuppressionError::CorruptStorage("event recipient"))?;
                self.commit(tx, current).await?;
                return Ok(record);
            }
            let (recipients, events) = self.counts(&mut tx).await?;
            if events >= self.config.max_events
                || (existing.is_none() && recipients >= self.config.max_recipients)
            {
                return Err(SuppressionError::CapacityExceeded);
            }
            let record = super::super::memory::merge_record(existing.as_ref(), &event);
            let recipient_tag = self.tag(b"recipient", &[event.recipient().as_bytes()])?;
            sqlx::query(
                "INSERT INTO rullst_mail_pg_suppression_recipients
                 (namespace,recipient_tag,reason,provider,first_seen_at,last_seen_at)
                 VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT(namespace,recipient_tag) DO UPDATE
                 SET reason = excluded.reason, provider = excluded.provider,
                 first_seen_at = excluded.first_seen_at, last_seen_at = excluded.last_seen_at",
            )
            .bind(&self.config.namespace)
            .bind(recipient_tag)
            .bind(record.reason().rank())
            .bind(record.provider())
            .bind(record.first_seen_at() as i64)
            .bind(record.last_seen_at() as i64)
            .execute(&mut *tx)
            .await
            .map_err(|_| unavailable("persist recipient"))?;
            sqlx::query(
                "INSERT INTO rullst_mail_pg_suppression_events
                 (namespace,event_tag,fingerprint,observed_at) VALUES ($1,$2,$3,$4)",
            )
            .bind(&self.config.namespace)
            .bind(event_tag)
            .bind(self.tag(b"event-contents", &fingerprint_parts)?)
            .bind(observed)
            .execute(&mut *tx)
            .await
            .map_err(|_| unavailable("persist replay identity"))?;
            self.commit(tx, current).await?;
            Ok(record)
        })
        .await
    }
}

impl PostgresSuppressionStore {
    pub(super) async fn fetch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        recipient: &str,
        current: i64,
    ) -> Result<Option<SuppressionRecord>, SuppressionError> {
        let tag = self.tag(b"recipient", &[recipient.as_bytes()])?;
        let row = sqlx::query(
            "SELECT reason,provider,first_seen_at,last_seen_at
             FROM rullst_mail_pg_suppression_recipients WHERE namespace = $1 AND recipient_tag = $2",
        ).bind(&self.config.namespace).bind(tag).fetch_optional(&mut **tx).await
            .map_err(|_| unavailable("lookup recipient"))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let reason: i64 = row
            .try_get("reason")
            .map_err(|_| unavailable("decode reason"))?;
        let provider: String = row
            .try_get("provider")
            .map_err(|_| unavailable("decode provider"))?;
        let first: i64 = row
            .try_get("first_seen_at")
            .map_err(|_| unavailable("decode time"))?;
        let last: i64 = row
            .try_get("last_seen_at")
            .map_err(|_| unavailable("decode time"))?;
        if first <= 0 || last < first || last > current + 300 {
            return Err(SuppressionError::CorruptStorage("recipient time"));
        }
        validate_identifier(&provider, MAX_PROVIDER_BYTES, "provider")
            .map_err(|_| SuppressionError::CorruptStorage("provider"))?;
        Ok(Some(SuppressionRecord {
            recipient: recipient.to_owned(),
            reason: SuppressionReason::from_rank(reason)?,
            provider,
            first_seen_at: first as u64,
            last_seen_at: last as u64,
        }))
    }
    /// Counts authoritative recipient/replay rows without revealing identities.
    pub async fn snapshot(&self) -> Result<SuppressionSnapshot, SuppressionError> {
        bounded(async {
            let (mut tx, current) = self.begin().await?;
            let (recipients, events) = self.counts(&mut tx).await?;
            self.commit(tx, current).await?;
            Ok(SuppressionSnapshot::new(
                recipients,
                events,
                self.config.max_recipients,
                self.config.max_events,
            ))
        })
        .await
    }
    /// Prunes old replay identifiers only. The host selects a cutoff after every
    /// provider's redelivery window; pruned identities can be accepted again.
    /// Recipient suppression persists independently and cannot be lifted here.
    pub async fn prune_events_before(&self, cutoff: u64) -> Result<usize, SuppressionError> {
        bounded(async {
            let cutoff = i64::try_from(cutoff).ok().filter(|value|*value>0)
                .ok_or(SuppressionError::InvalidConfiguration("event cutoff"))?;
            let (mut tx,current) = self.begin().await?;
            if cutoff > current { return Err(SuppressionError::InvalidConfiguration("event cutoff")); }
            let removed = sqlx::query(
                "DELETE FROM rullst_mail_pg_suppression_events WHERE namespace = $1 AND observed_at < $2",
            ).bind(&self.config.namespace).bind(cutoff).execute(&mut *tx).await
                .map_err(|_| unavailable("prune replay identities"))?.rows_affected();
            self.commit(tx,current).await?;
            usize::try_from(removed).map_err(|_| SuppressionError::CorruptStorage("pruned count"))
        }).await
    }
}
