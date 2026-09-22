//! Bounded account-owned inventory and authoritative opaque-session revocation.
mod inventory;
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
mod types;

pub use types::{ActiveSession, SessionId, SessionLabel};

use super::{
    AuthenticatedRecoveryAccount, RecoveryError, SecretToken, SqlRecoveryStore, timestamp,
};
use sqlx::{Any, Transaction};
use std::{future::Future, sync::Arc, time::Duration};

pub(super) const MAX_SESSIONS: i64 = 20;

// Cancelling a database future may race a commit. Errors never authenticate or
// assert rollback; callers reconcile authoritative state after an uncertain write.
async fn bounded<T>(
    operation: impl Future<Output = Result<T, RecoveryError>>,
) -> Result<T, RecoveryError> {
    tokio::time::timeout(Duration::from_secs(10), operation)
        .await
        .map_err(|_| RecoveryError::Storage)?
}

impl SqlRecoveryStore {
    /// Mints an opaque session after password authentication. Set it only in a
    /// Secure, HttpOnly, SameSite cookie; verify it on every authenticated request.
    /// `now` must come from the trusted application clock, never request data.
    pub async fn create_session(
        &self,
        account: &AuthenticatedRecoveryAccount,
        now: u64,
        lifetime_seconds: u32,
    ) -> Result<SecretToken, RecoveryError> {
        bounded(self.issue_session(account, now, lifetime_seconds, None)).await
    }

    /// Creates a session with an explicit, bounded display label. No IP address,
    /// user agent, device fingerprint or activity history is collected.
    pub async fn create_session_with_label(
        &self,
        account: &AuthenticatedRecoveryAccount,
        now: u64,
        lifetime_seconds: u32,
        label: SessionLabel,
    ) -> Result<SecretToken, RecoveryError> {
        bounded(self.issue_session(account, now, lifetime_seconds, Some(label))).await
    }

    async fn issue_session(
        &self,
        account: &AuthenticatedRecoveryAccount,
        now: u64,
        lifetime_seconds: u32,
        label: Option<SessionLabel>,
    ) -> Result<SecretToken, RecoveryError> {
        if !Arc::ptr_eq(&account.instance, &self.instance) {
            return Err(RecoveryError::InvalidAction);
        }
        if lifetime_seconds == 0 || lifetime_seconds > 30 * 86400 {
            return Err(RecoveryError::InvalidInput);
        }
        let now = timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        self.lock_writes(&mut tx).await?;
        let token = self
            .insert_session(&mut tx, account, now, lifetime_seconds, label)
            .await?;
        tx.commit().await?;
        Ok(token)
    }

    // Caller holds lock_writes and validates its authentication proof, time and
    // lifetime. Email login consumes its one-use challenge in this same transaction.
    pub(super) async fn insert_session(
        &self,
        tx: &mut Transaction<'_, Any>,
        account: &AuthenticatedRecoveryAccount,
        now: i64,
        lifetime_seconds: u32,
        label: Option<SessionLabel>,
    ) -> Result<SecretToken, RecoveryError> {
        let token = SecretToken::generate()?;
        let digest = self.keys.digest("session", token.expose());
        let current: Option<i64> = sqlx::query_scalar(
            "SELECT session_version FROM rullst_recovery_accounts WHERE subject = $1",
        )
        .bind(&account.subject)
        .fetch_optional(&mut **tx)
        .await?;
        if current != Some(account.version) {
            return Err(RecoveryError::InvalidAction);
        }
        sqlx::query("DELETE FROM rullst_recovery_session_details WHERE token_digest IN (SELECT token_digest FROM rullst_recovery_sessions WHERE subject = $1 AND expires_at <= $2)")
            .bind(&account.subject).bind(now).execute(&mut **tx).await?;
        sqlx::query("DELETE FROM rullst_recovery_sessions WHERE subject = $1 AND expires_at <= $2")
            .bind(&account.subject)
            .bind(now)
            .execute(&mut **tx)
            .await?;
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM rullst_recovery_sessions WHERE subject = $1")
                .bind(&account.subject)
                .fetch_one(&mut **tx)
                .await?;
        if count >= MAX_SESSIONS {
            return Err(RecoveryError::Limited);
        }
        sqlx::query("INSERT INTO rullst_recovery_sessions (token_digest, subject, session_version, expires_at) VALUES ($1, $2, $3, $4)")
            .bind(&digest).bind(&account.subject).bind(account.version).bind(now + i64::from(lifetime_seconds))
            .execute(&mut **tx).await?;
        sqlx::query("INSERT INTO rullst_recovery_session_details (token_digest, created_at, label) VALUES ($1, $2, $3)")
            .bind(&digest).bind(now).bind(label.as_ref().map_or("", SessionLabel::as_str))
            .execute(&mut **tx).await?;
        Ok(token)
    }

    /// Reads authoritative revocation state; storage errors never authenticate.
    /// `now` is trusted server time. Previously authorized in-flight work is not recalled.
    pub async fn verify_session(
        &self,
        token: &str,
        now: u64,
    ) -> Result<Option<String>, RecoveryError> {
        if token.len() != 43 {
            return Ok(None);
        }
        bounded(async {
            let subject = sqlx::query_scalar("SELECT s.subject FROM rullst_recovery_sessions s JOIN rullst_recovery_accounts a ON a.subject = s.subject AND a.session_version = s.session_version WHERE s.token_digest = $1 AND s.expires_at > $2")
                .bind(self.keys.digest("session", token)).bind(timestamp(now)?).fetch_optional(&self.pool).await?;
            Ok(subject)
        }).await
    }

    /// Logout revokes this opaque session across every verifier sharing the store.
    /// The operation is idempotent and removes its display metadata atomically.
    pub async fn revoke_session(&self, token: &str) -> Result<(), RecoveryError> {
        if token.len() != 43 {
            return Ok(());
        }
        bounded(async {
            let mut tx = self.pool.begin().await?;
            self.lock_writes(&mut tx).await?;
            delete_session(&mut tx, &self.keys.digest("session", token)).await?;
            tx.commit().await?;
            Ok(())
        })
        .await
    }
}

impl SqlRecoveryStore {
    /// Operator-owned retention task: atomically removes at most 100 expired
    /// sessions and their display metadata. Schedule this with trusted server
    /// time; never expose it as an account-owned unauthenticated HTTP operation.
    pub async fn purge_expired_sessions(
        &self,
        now: u64,
        limit: u32,
    ) -> Result<usize, RecoveryError> {
        if !(1..=100).contains(&limit) {
            return Err(RecoveryError::InvalidInput);
        }
        let now = timestamp(now)?;
        bounded(async {
            let mut tx = self.pool.begin().await?;
            self.lock_writes(&mut tx).await?;
            let digests: Vec<String> = sqlx::query_scalar("SELECT token_digest FROM rullst_recovery_sessions WHERE expires_at <= $1 ORDER BY expires_at, token_digest LIMIT $2")
                .bind(now).bind(i64::from(limit)).fetch_all(&mut *tx).await?;
            for digest in &digests {
                delete_session(&mut tx, digest).await?;
            }
            tx.commit().await?;
            Ok(digests.len())
        }).await
    }
}

pub(super) async fn delete_session(
    tx: &mut Transaction<'_, Any>,
    digest: &str,
) -> Result<u64, RecoveryError> {
    sqlx::query("DELETE FROM rullst_recovery_session_details WHERE token_digest = $1")
        .bind(digest)
        .execute(&mut **tx)
        .await?;
    let count = sqlx::query("DELETE FROM rullst_recovery_sessions WHERE token_digest = $1")
        .bind(digest)
        .execute(&mut **tx)
        .await?
        .rows_affected();
    Ok(count)
}
