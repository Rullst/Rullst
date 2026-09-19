use super::{
    RecoveryError, RecoveryNotice, RecoveryNoticeKind, SecretToken, SqlRecoveryStore, timestamp,
};
use sqlx::{Any, Row, Transaction};

/// Redacted delivery outcome. Provider payloads must never be persisted here.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum RecoveryDeliveryFailure {
    Transient,
    Permanent,
    PermanentBounce,
    Complaint,
}

/// Aggregate operations data safe to expose only through an authorized dashboard.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RecoveryOutboxSnapshot {
    pub pending: i64,
    pub leased: i64,
    pub delivered: i64,
    pub failed: i64,
}

/// Fenced delivery lease and decrypted notice. Never put the notice in logs.
pub struct ClaimedRecoveryNotice {
    id: String,
    lease: String,
    notice: RecoveryNotice,
}

impl std::fmt::Debug for ClaimedRecoveryNotice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaimedRecoveryNotice")
            .field("id", &self.id)
            .field("notice", &self.notice)
            .finish_non_exhaustive()
    }
}

impl ClaimedRecoveryNotice {
    /// Stable across worker retries. Pass it to transports that support idempotency.
    pub fn delivery_id(&self) -> &str {
        &self.id
    }
    pub fn notice(&self) -> &RecoveryNotice {
        &self.notice
    }
}

impl SqlRecoveryStore {
    pub(super) async fn enqueue(
        &self,
        tx: &mut Transaction<'_, Any>,
        subject: &str,
        notice: RecoveryNotice,
        now: i64,
    ) -> Result<(), RecoveryError> {
        sqlx::query("DELETE FROM rullst_recovery_outbox WHERE expires_at <= $1")
            .bind(now)
            .execute(&mut **tx)
            .await?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_recovery_outbox")
            .fetch_one(&mut **tx)
            .await?;
        if count >= 10000 {
            return Err(RecoveryError::Limited);
        }
        let id = SecretToken::generate()?.expose().to_owned();
        let ciphertext = notice.seal(&self.keys, &id)?;
        let kind = match notice.kind() {
            RecoveryNoticeKind::Welcome => "welcome",
            RecoveryNoticeKind::PasswordReset => "reset",
            RecoveryNoticeKind::PasswordChanged => "changed",
        };
        sqlx::query("INSERT INTO rullst_recovery_outbox (id, subject, kind, ciphertext, expires_at, status, attempts, due_at, lease, lease_until) VALUES ($1, $2, $3, $4, $5, 'pending', 0, $6, '', 0)")
            .bind(id).bind(subject).bind(kind).bind(ciphertext).bind(notice.expires_at).bind(now).execute(&mut **tx).await?;
        Ok(())
    }

    /// Claims one due notice for 60 seconds, at most six attempts, with restart
    /// recovery. Delivery is at-least-once: a crash after provider acceptance can
    /// duplicate a message unless the provider honors the stable delivery ID.
    pub async fn claim_notice(
        &self,
        now: u64,
    ) -> Result<Option<ClaimedRecoveryNotice>, RecoveryError> {
        let now = timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        self.lock_writes(&mut tx).await?;
        sqlx::query("DELETE FROM rullst_recovery_outbox WHERE expires_at <= $1")
            .bind(now)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE rullst_recovery_outbox SET status = 'failed', ciphertext = '', lease = '', lease_until = 0 WHERE attempts >= 6 AND status = 'leased' AND lease_until <= $1").bind(now).execute(&mut *tx).await?;
        let row = sqlx::query("SELECT o.id, o.ciphertext FROM rullst_recovery_outbox o JOIN rullst_recovery_accounts a ON a.subject = o.subject WHERE a.suppressed = 0 AND o.attempts < 6 AND o.due_at <= $1 AND (o.status = 'pending' OR (o.status = 'leased' AND o.lease_until <= $1)) ORDER BY o.due_at, o.id LIMIT 1")
            .bind(now).fetch_optional(&mut *tx).await?;
        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        let id: String = row.try_get("id")?;
        let ciphertext: String = row.try_get("ciphertext")?;
        let notice = RecoveryNotice::open(&self.keys, &id, &ciphertext)?;
        let lease = SecretToken::generate()?.expose().to_owned();
        sqlx::query("UPDATE rullst_recovery_outbox SET status = 'leased', attempts = attempts + 1, lease = $1, lease_until = $2 WHERE id = $3")
            .bind(&lease).bind(now + 60).bind(&id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some(ClaimedRecoveryNotice { id, lease, notice }))
    }

    /// Acknowledges only the current, unexpired lease. Old workers cannot mark a
    /// newer retry as delivered or overwrite its result.
    pub async fn complete_notice(
        &self,
        claim: &ClaimedRecoveryNotice,
        now: u64,
    ) -> Result<(), RecoveryError> {
        let changed = sqlx::query("UPDATE rullst_recovery_outbox SET status = 'delivered', ciphertext = '', lease = '', lease_until = 0 WHERE id = $1 AND lease = $2 AND status = 'leased' AND lease_until > $3")
            .bind(&claim.id).bind(&claim.lease).bind(timestamp(now)?).execute(&self.pool).await?.rows_affected();
        if changed != 1 {
            return Err(RecoveryError::InvalidAction);
        }
        Ok(())
    }

    pub async fn fail_notice(
        &self,
        claim: &ClaimedRecoveryNotice,
        failure: RecoveryDeliveryFailure,
        now: u64,
    ) -> Result<(), RecoveryError> {
        let now = timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        self.lock_writes(&mut tx).await?;
        let row = sqlx::query("SELECT subject, attempts FROM rullst_recovery_outbox WHERE id = $1 AND lease = $2 AND status = 'leased' AND lease_until > $3")
            .bind(&claim.id).bind(&claim.lease).bind(now).fetch_optional(&mut *tx).await?.ok_or(RecoveryError::InvalidAction)?;
        let attempts: i64 = row.try_get("attempts")?;
        let terminal = !matches!(failure, RecoveryDeliveryFailure::Transient) || attempts >= 6;
        let delay = 15i64 * (1i64 << attempts.clamp(0, 6));
        sqlx::query("UPDATE rullst_recovery_outbox SET status = $1, due_at = $2, lease = '', lease_until = 0, ciphertext = CASE WHEN $3 = 1 THEN '' ELSE ciphertext END WHERE id = $4")
            .bind(if terminal { "failed" } else { "pending" }).bind(now + delay).bind(if terminal { 1i64 } else { 0 })
            .bind(&claim.id).execute(&mut *tx).await?;
        if matches!(
            failure,
            RecoveryDeliveryFailure::PermanentBounce | RecoveryDeliveryFailure::Complaint
        ) {
            let subject: String = row.try_get("subject")?;
            sqlx::query("UPDATE rullst_recovery_accounts SET suppressed = 1 WHERE subject = $1")
                .bind(subject)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn outbox_snapshot(&self) -> Result<RecoveryOutboxSnapshot, RecoveryError> {
        let mut snapshot = RecoveryOutboxSnapshot {
            pending: 0,
            leased: 0,
            delivered: 0,
            failed: 0,
        };
        for row in sqlx::query(
            "SELECT status, COUNT(*) AS total FROM rullst_recovery_outbox GROUP BY status",
        )
        .fetch_all(&self.pool)
        .await?
        {
            let count: i64 = row.try_get("total")?;
            match row.try_get::<String, _>("status")?.as_str() {
                "pending" => snapshot.pending = count,
                "leased" => snapshot.leased = count,
                "delivered" => snapshot.delivered = count,
                "failed" => snapshot.failed = count,
                _ => return Err(RecoveryError::Storage),
            }
        }
        Ok(snapshot)
    }
}
