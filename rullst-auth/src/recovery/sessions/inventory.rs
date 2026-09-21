use super::{ActiveSession, MAX_SESSIONS, SessionId, SessionLabel, bounded, delete_session};
use crate::recovery::{RecoveryError, SqlRecoveryStore, timestamp};
use sqlx::{Any, Row, Transaction};
use subtle::ConstantTimeEq;

struct Current {
    subject: String,
    digest: String,
    version: i64,
}

impl SqlRecoveryStore {
    /// Lists at most 20 active sessions of the authenticated account.
    /// The caller supplies trusted server time and must protect the response with
    /// no-store headers. No account/tenant selector comes from the client.
    pub async fn active_sessions(
        &self,
        current_token: &str,
        now: u64,
    ) -> Result<Vec<ActiveSession>, RecoveryError> {
        bounded(self.active_sessions_inner(current_token, now)).await
    }

    async fn active_sessions_inner(
        &self,
        current_token: &str,
        now: u64,
    ) -> Result<Vec<ActiveSession>, RecoveryError> {
        let now = timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        self.lock_writes(&mut tx).await?;
        let current = self.current_session(&mut tx, current_token, now).await?;
        let rows = self.session_inventory(&mut tx, &current, now).await?;
        tx.commit().await?;
        Ok(rows.into_iter().map(|(_, session)| session).collect())
    }

    /// Revokes one sibling of the currently authenticated session. Missing,
    /// expired, foreign-account and current-session targets are rejected alike.
    /// Mount this mutation behind CSRF and authenticated tenant selection.
    pub async fn revoke_other_session(
        &self,
        current_token: &str,
        target: &SessionId,
        now: u64,
    ) -> Result<(), RecoveryError> {
        bounded(self.revoke_other_session_inner(current_token, target, now)).await
    }

    async fn revoke_other_session_inner(
        &self,
        current_token: &str,
        target: &SessionId,
        now: u64,
    ) -> Result<(), RecoveryError> {
        let now = timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        self.lock_writes(&mut tx).await?;
        let current = self.current_session(&mut tx, current_token, now).await?;
        let rows = self.session_inventory(&mut tx, &current, now).await?;
        let digest = rows
            .into_iter()
            .find_map(|(digest, session)| {
                (bool::from(session.id.0.as_bytes().ct_eq(target.0.as_bytes())) && !session.current)
                    .then_some(digest)
            })
            .ok_or(RecoveryError::InvalidAction)?;
        if delete_session(&mut tx, &digest).await? != 1 {
            return Err(RecoveryError::InvalidAction);
        }
        tx.commit().await?;
        Ok(())
    }

    /// Revokes every sibling, preserves the current bearer token and fences old
    /// password-authentication proofs by advancing the account session version.
    /// Returns the number of active siblings revoked. Repeat calls are safe.
    pub async fn revoke_other_sessions(
        &self,
        current_token: &str,
        now: u64,
    ) -> Result<usize, RecoveryError> {
        bounded(self.revoke_other_sessions_inner(current_token, now)).await
    }

    async fn revoke_other_sessions_inner(
        &self,
        current_token: &str,
        now: u64,
    ) -> Result<usize, RecoveryError> {
        let now = timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        self.lock_writes(&mut tx).await?;
        let current = self.current_session(&mut tx, current_token, now).await?;
        let active = self.session_inventory(&mut tx, &current, now).await?;
        let removed = active
            .iter()
            .filter(|(_, session)| !session.current)
            .count();
        let version = current
            .version
            .checked_add(1)
            .ok_or(RecoveryError::Storage)?;
        sqlx::query("DELETE FROM rullst_recovery_session_details WHERE token_digest IN (SELECT token_digest FROM rullst_recovery_sessions WHERE subject = $1 AND token_digest <> $2)")
            .bind(&current.subject).bind(&current.digest).execute(&mut *tx).await?;
        sqlx::query(
            "DELETE FROM rullst_recovery_sessions WHERE subject = $1 AND token_digest <> $2",
        )
        .bind(&current.subject)
        .bind(&current.digest)
        .execute(&mut *tx)
        .await?;
        let account = sqlx::query("UPDATE rullst_recovery_accounts SET session_version = $1 WHERE subject = $2 AND session_version = $3")
            .bind(version).bind(&current.subject).bind(current.version).execute(&mut *tx).await?;
        let retained = sqlx::query("UPDATE rullst_recovery_sessions SET session_version = $1 WHERE token_digest = $2 AND subject = $3 AND session_version = $4")
            .bind(version).bind(&current.digest).bind(&current.subject).bind(current.version).execute(&mut *tx).await?;
        if account.rows_affected() != 1 || retained.rows_affected() != 1 {
            return Err(RecoveryError::InvalidAction);
        }
        tx.commit().await?;
        Ok(removed)
    }

    async fn current_session(
        &self,
        tx: &mut Transaction<'_, Any>,
        token: &str,
        now: i64,
    ) -> Result<Current, RecoveryError> {
        if token.len() != 43 {
            return Err(RecoveryError::InvalidAction);
        }
        let digest = self.keys.digest("session", token);
        let row = sqlx::query("SELECT s.subject, s.session_version FROM rullst_recovery_sessions s JOIN rullst_recovery_accounts a ON a.subject = s.subject AND a.session_version = s.session_version WHERE s.token_digest = $1 AND s.expires_at > $2")
            .bind(&digest).bind(now).fetch_optional(&mut **tx).await?
            .ok_or(RecoveryError::InvalidAction)?;
        let version = row.try_get("session_version")?;
        if version <= 0 {
            return Err(RecoveryError::Storage);
        }
        Ok(Current {
            subject: row.try_get("subject")?,
            digest,
            version,
        })
    }

    async fn session_inventory(
        &self,
        tx: &mut Transaction<'_, Any>,
        current: &Current,
        now: i64,
    ) -> Result<Vec<(String, ActiveSession)>, RecoveryError> {
        let rows = sqlx::query("SELECT s.token_digest, s.expires_at, COALESCE(d.created_at, -1) AS created_at, COALESCE(d.label, '') AS label FROM rullst_recovery_sessions s LEFT JOIN rullst_recovery_session_details d ON d.token_digest = s.token_digest WHERE s.subject = $1 AND s.session_version = $2 AND s.expires_at > $3 ORDER BY s.expires_at, s.token_digest LIMIT 21")
            .bind(&current.subject).bind(current.version).bind(now).fetch_all(&mut **tx).await?;
        if rows.len() > MAX_SESSIONS as usize {
            return Err(RecoveryError::Storage);
        }
        rows.into_iter()
            .map(|row| {
                let digest: String = row.try_get("token_digest")?;
                let label: String = row.try_get("label")?;
                let created: i64 = row.try_get("created_at")?;
                let expires: i64 = row.try_get("expires_at")?;
                if created < -1 || created > expires || expires <= 0 {
                    return Err(RecoveryError::Storage);
                }
                let session = ActiveSession {
                    id: SessionId(self.keys.digest("session-management", &digest)),
                    created_at: (created >= 0).then_some(created as u64),
                    expires_at: expires as u64,
                    label: if label.is_empty() {
                        None
                    } else {
                        Some(SessionLabel::new(label).map_err(|_| RecoveryError::Storage)?)
                    },
                    current: bool::from(digest.as_bytes().ct_eq(current.digest.as_bytes())),
                };
                Ok((digest, session))
            })
            .collect()
    }
}
