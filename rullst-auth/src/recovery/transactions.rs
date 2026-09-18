use super::{
    RecoveryError, RecoveryNotice, RecoveryNoticeKind, ResetRequestAccepted, SecretToken,
    SqlRecoveryStore, normalized_email, timestamp,
};
use sqlx::{Any, Row, Transaction};

impl SqlRecoveryStore {
    /// Always returns the same acknowledgement for absent accounts and request
    /// throttling. Enqueues without contacting a mail provider and applies a
    /// 250 ms minimum response time. Ingress limits and deployment timing tests
    /// remain required; a minimum delay alone cannot hide arbitrary DB outages.
    pub async fn request_password_reset(
        &self,
        email: impl Into<String>,
        now: u64,
    ) -> Result<ResetRequestAccepted, RecoveryError> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(250);
        let result = self.request_reset_inner(email.into(), now).await;
        tokio::time::sleep_until(deadline).await;
        // Capacity rejection is publicly indistinguishable from absent or
        // throttled accounts; the transaction has already rolled back.
        match result {
            Err(RecoveryError::Limited) => Ok(ResetRequestAccepted),
            other => other,
        }
    }

    async fn request_reset_inner(
        &self,
        email: String,
        now: u64,
    ) -> Result<ResetRequestAccepted, RecoveryError> {
        let Ok(email) = normalized_email(&email) else {
            return Ok(ResetRequestAccepted);
        };
        let now = timestamp(now)?;
        let token = SecretToken::generate()?;
        let mut tx = self.pool.begin().await?;
        self.lock_writes(&mut tx).await?;
        if !take_limit(&mut tx, "request", now, 120).await? {
            tx.commit().await?;
            return Ok(ResetRequestAccepted);
        }
        let row = sqlx::query("SELECT subject, email_ciphertext, reset_window, reset_count, suppressed, locale FROM rullst_recovery_accounts WHERE email_key = $1")
            .bind(self.keys.digest("email", &email)).fetch_optional(&mut *tx).await?;
        if let Some(row) = row {
            let subject: String = row.try_get("subject")?;
            let window: i64 = row.try_get("reset_window")?;
            let count: i64 = row.try_get("reset_count")?;
            let suppressed: i64 = row.try_get("suppressed")?;
            let (window, count) = if now >= window + 900 {
                (now, 0)
            } else {
                (window, count)
            };
            if suppressed == 0 && count < 3 && now >= window {
                sqlx::query("UPDATE rullst_recovery_accounts SET reset_window = $1, reset_count = $2 WHERE subject = $3")
                    .bind(window).bind(count + 1).bind(&subject).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO rullst_recovery_tokens (subject, token_digest, expires_at) VALUES ($1, $2, $3) ON CONFLICT(subject) DO UPDATE SET token_digest = excluded.token_digest, expires_at = excluded.expires_at")
                    .bind(&subject).bind(self.keys.digest("password-reset", token.expose())).bind(now + 1200).execute(&mut *tx).await?;
                // A replacement invalidates the old code and cancels its unsent mail.
                sqlx::query("DELETE FROM rullst_recovery_outbox WHERE subject = $1 AND kind = 'reset' AND status IN ('pending', 'leased')")
                    .bind(&subject).execute(&mut *tx).await?;
                let ciphertext: String = row.try_get("email_ciphertext")?;
                let recipient = self.recipient(&subject, &ciphertext)?;
                self.enqueue(
                    &mut tx,
                    &subject,
                    RecoveryNotice {
                        recipient,
                        token: Some(token),
                        kind: RecoveryNoticeKind::PasswordReset,
                        expires_at: now + 1200,
                        locale: super::RecoveryLocale::parse(&row.try_get::<String, _>("locale")?)?,
                    },
                    now,
                )
                .await?;
            }
        }
        tx.commit().await?;
        Ok(ResetRequestAccepted)
    }

    /// Atomically replaces the password, consumes all reset tokens, revokes every
    /// session and persists the encrypted password-change notice. Never logs in
    /// automatically. Independent consume limits run before Argon2 work.
    pub async fn complete_password_reset(
        &self,
        token: &str,
        new_password: impl Into<String>,
        now: u64,
    ) -> Result<(), RecoveryError> {
        let now = timestamp(now)?;
        let mut limit = self.pool.begin().await?;
        self.lock_writes(&mut limit).await?;
        let allowed = take_limit(&mut limit, "consume", now, 60).await?;
        limit.commit().await?;
        if !allowed {
            return Err(RecoveryError::Limited);
        }
        if token.len() != 43 {
            return Err(RecoveryError::InvalidAction);
        }
        let hash = super::store::password_hash(new_password.into()).await?;
        let mut tx = self.pool.begin().await?;
        self.lock_writes(&mut tx).await?;
        let row = sqlx::query("SELECT a.subject, a.email_ciphertext, a.session_version, a.suppressed, a.locale FROM rullst_recovery_tokens t JOIN rullst_recovery_accounts a ON a.subject = t.subject WHERE t.token_digest = $1 AND t.expires_at > $2")
            .bind(self.keys.digest("password-reset", token)).bind(now).fetch_optional(&mut *tx).await?
            .ok_or(RecoveryError::InvalidAction)?;
        let subject: String = row.try_get("subject")?;
        let version: i64 = row.try_get("session_version")?;
        let new_version = version.checked_add(1).ok_or(RecoveryError::Limited)?;
        sqlx::query("UPDATE rullst_recovery_accounts SET password_hash = $1, session_version = $2 WHERE subject = $3")
            .bind(hash).bind(new_version).bind(&subject).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM rullst_recovery_tokens WHERE subject = $1")
            .bind(&subject)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM rullst_recovery_sessions WHERE subject = $1")
            .bind(&subject)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM rullst_recovery_outbox WHERE subject = $1 AND kind = 'reset' AND status IN ('pending', 'leased')")
            .bind(&subject).execute(&mut *tx).await?;
        if row.try_get::<i64, _>("suppressed")? == 0 {
            let ciphertext: String = row.try_get("email_ciphertext")?;
            self.enqueue(
                &mut tx,
                &subject,
                RecoveryNotice {
                    recipient: self.recipient(&subject, &ciphertext)?,
                    token: None,
                    kind: RecoveryNoticeKind::PasswordChanged,
                    expires_at: now + 86400,
                    locale: super::RecoveryLocale::parse(&row.try_get::<String, _>("locale")?)?,
                },
                now,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

async fn take_limit(
    tx: &mut Transaction<'_, Any>,
    id: &str,
    now: i64,
    maximum: i64,
) -> Result<bool, RecoveryError> {
    let row =
        sqlx::query("SELECT window_start, attempts FROM rullst_recovery_control WHERE id = $1")
            .bind(id)
            .fetch_one(&mut **tx)
            .await?;
    let start: i64 = row.try_get("window_start")?;
    let attempts: i64 = row.try_get("attempts")?;
    if now < start {
        return Ok(false);
    }
    let (start, attempts) = if now >= start + 60 {
        (now, 0)
    } else {
        (start, attempts)
    };
    if attempts >= maximum {
        return Ok(false);
    }
    sqlx::query(
        "UPDATE rullst_recovery_control SET window_start = $1, attempts = $2 WHERE id = $3",
    )
    .bind(start)
    .bind(attempts + 1)
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(true)
}
