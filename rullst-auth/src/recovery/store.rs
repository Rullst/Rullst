use super::{
    RecoveryError, RecoveryNotice, RecoveryNoticeKind, RecoverySecrets, SecretToken,
    normalized_email, timestamp, valid_subject,
};
use sqlx::{Any, AnyPool, Row, Transaction};
use std::sync::Arc;
use zeroize::Zeroizing;

/// SQL-backed authoritative account registry. PostgreSQL is shared across hosts;
/// SQLite is durable on one shared local file, not a distributed database.
#[derive(Clone)]
pub struct SqlRecoveryStore {
    pub(super) pool: AnyPool,
    pub(super) keys: Arc<RecoverySecrets>,
    instance: Arc<()>,
}

/// Proof of successful password verification, bound to the current session version.
pub struct AuthenticatedRecoveryAccount {
    subject: String,
    version: i64,
    instance: Arc<()>,
}

impl std::fmt::Debug for AuthenticatedRecoveryAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AuthenticatedRecoveryAccount([REDACTED])")
    }
}

impl AuthenticatedRecoveryAccount {
    pub fn subject(&self) -> &str {
        &self.subject
    }
}

impl SqlRecoveryStore {
    /// Connects without changing schema. Call `migrate` explicitly during deployment.
    pub async fn connect(
        url: impl Into<String>,
        secrets: RecoverySecrets,
    ) -> Result<Self, RecoveryError> {
        let url = url.into();
        let sqlite = url.starts_with("sqlite:");
        let postgres = url.starts_with("postgres:") || url.starts_with("postgresql:");
        if (!sqlite && !postgres)
            || (sqlite && !cfg!(feature = "recovery-sqlite"))
            || (postgres && !cfg!(feature = "recovery-postgres"))
        {
            return Err(RecoveryError::Configuration);
        }
        sqlx::any::install_default_drivers();
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(if sqlite { 1 } else { 5 })
            .acquire_timeout(std::time::Duration::from_secs(5))
            .after_connect(move |connection, _| {
                Box::pin(async move {
                    if sqlite {
                        sqlx::query("PRAGMA busy_timeout = 5000")
                            .execute(&mut *connection)
                            .await?;
                    }
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        Ok(Self {
            pool,
            keys: Arc::new(secrets),
            instance: Arc::new(()),
        })
    }

    /// Creates a fixed schema and binds it to the deployment's digest/encryption keys.
    pub async fn migrate(&self) -> Result<(), RecoveryError> {
        let mut tx = self.pool.begin().await?;
        for ddl in [
            "CREATE TABLE IF NOT EXISTS rullst_recovery_control (id TEXT PRIMARY KEY, window_start BIGINT NOT NULL, attempts BIGINT NOT NULL, binding TEXT NOT NULL)",
            "CREATE TABLE IF NOT EXISTS rullst_recovery_accounts (subject TEXT PRIMARY KEY, email_key TEXT NOT NULL UNIQUE, email_ciphertext TEXT NOT NULL, password_hash TEXT NOT NULL, session_version BIGINT NOT NULL, reset_window BIGINT NOT NULL, reset_count BIGINT NOT NULL, suppressed BIGINT NOT NULL, locale TEXT NOT NULL)",
            "CREATE TABLE IF NOT EXISTS rullst_recovery_tokens (subject TEXT PRIMARY KEY, token_digest TEXT NOT NULL UNIQUE, expires_at BIGINT NOT NULL)",
            "CREATE TABLE IF NOT EXISTS rullst_recovery_sessions (token_digest TEXT PRIMARY KEY, subject TEXT NOT NULL, session_version BIGINT NOT NULL, expires_at BIGINT NOT NULL)",
            "CREATE TABLE IF NOT EXISTS rullst_recovery_outbox (id TEXT PRIMARY KEY, subject TEXT NOT NULL, kind TEXT NOT NULL, ciphertext TEXT NOT NULL, expires_at BIGINT NOT NULL, status TEXT NOT NULL, attempts BIGINT NOT NULL, due_at BIGINT NOT NULL, lease TEXT NOT NULL, lease_until BIGINT NOT NULL)",
            "CREATE INDEX IF NOT EXISTS rullst_recovery_outbox_due ON rullst_recovery_outbox(status, due_at)",
            "CREATE INDEX IF NOT EXISTS rullst_recovery_sessions_subject ON rullst_recovery_sessions(subject)",
        ] {
            sqlx::query(ddl).execute(&mut *tx).await?;
        }
        let binding = self.keys.digest("configuration", "v1");
        let sealed = self
            .keys
            .seal("rullst.auth.config.v1", binding.as_bytes())?;
        for id in ["request", "consume", "write"] {
            sqlx::query("INSERT INTO rullst_recovery_control (id, window_start, attempts, binding) VALUES ($1, 0, 0, $2) ON CONFLICT(id) DO NOTHING")
                .bind(id).bind(&sealed).execute(&mut *tx).await?;
            let value: String =
                sqlx::query_scalar("SELECT binding FROM rullst_recovery_control WHERE id = $1")
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
            if self.keys.open("rullst.auth.config.v1", &value)?.as_slice() != binding.as_bytes() {
                return Err(RecoveryError::Configuration);
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// Creates an account and encrypted welcome notice in one transaction.
    /// Application-specific profile rows must be linked through this opaque subject.
    pub async fn register_account(
        &self,
        subject: impl Into<String>,
        email: impl Into<String>,
        password: impl Into<String>,
        now: u64,
    ) -> Result<(), RecoveryError> {
        self.register_account_with_locale(subject, email, password, None, now)
            .await
    }

    /// Registers the user's explicit language preference with the account event.
    pub async fn register_account_with_locale(
        &self,
        subject: impl Into<String>,
        email: impl Into<String>,
        password: impl Into<String>,
        locale: Option<super::RecoveryLocale>,
        now: u64,
    ) -> Result<(), RecoveryError> {
        let subject = subject.into();
        let email = Zeroizing::new(normalized_email(&email.into())?);
        if !valid_subject(&subject) {
            return Err(RecoveryError::InvalidInput);
        }
        let now = timestamp(now)?;
        let hash = password_hash(password.into()).await?;
        let email_key = self.keys.digest("email", &email);
        let email_ciphertext = self.keys.seal(
            &format!("rullst.auth.account.v1:{subject}"),
            email.as_bytes(),
        )?;
        let mut tx = self.pool.begin().await?;
        self.lock_writes(&mut tx).await?;
        sqlx::query("INSERT INTO rullst_recovery_accounts (subject, email_key, email_ciphertext, password_hash, session_version, reset_window, reset_count, suppressed, locale) VALUES ($1, $2, $3, $4, 1, 0, 0, 0, $5)")
            .bind(&subject).bind(email_key).bind(email_ciphertext).bind(hash).bind(locale.map_or("", |value| value.as_str())).execute(&mut *tx).await?;
        self.enqueue(
            &mut tx,
            &subject,
            RecoveryNotice {
                recipient: email,
                token: None,
                kind: RecoveryNoticeKind::Welcome,
                expires_at: now + 86400,
                locale,
            },
            now,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Checks the authoritative password. Hosts must independently throttle login.
    pub async fn authenticate(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Option<AuthenticatedRecoveryAccount>, RecoveryError> {
        let email = normalized_email(&email.into())?;
        let password = password.into();
        if password.len() > 1024 {
            return Err(RecoveryError::InvalidInput);
        }
        let row = sqlx::query("SELECT subject, password_hash, session_version FROM rullst_recovery_accounts WHERE email_key = $1")
            .bind(self.keys.digest("email", &email)).fetch_optional(&self.pool).await?;
        let Some(row) = row else {
            // Match the current Argon2 work factor without a fast unknown-account branch.
            let _dummy = crate::hash_password_async(password)
                .await
                .map_err(|_| RecoveryError::Crypto)?;
            return Ok(None);
        };
        let hash: String = row.try_get("password_hash")?;
        if !crate::verify_password_async(password, hash).await {
            return Ok(None);
        }
        Ok(Some(AuthenticatedRecoveryAccount {
            subject: row.try_get("subject")?,
            version: row.try_get("session_version")?,
            instance: self.instance.clone(),
        }))
    }

    /// Mints an opaque session after password authentication. Set it only in a
    /// Secure, HttpOnly, SameSite cookie; verify it on every authenticated request.
    pub async fn create_session(
        &self,
        account: &AuthenticatedRecoveryAccount,
        now: u64,
        lifetime_seconds: u32,
    ) -> Result<SecretToken, RecoveryError> {
        if !Arc::ptr_eq(&account.instance, &self.instance) {
            return Err(RecoveryError::InvalidAction);
        }
        if lifetime_seconds == 0 || lifetime_seconds > 30 * 86400 {
            return Err(RecoveryError::InvalidInput);
        }
        let now = timestamp(now)?;
        let token = SecretToken::generate()?;
        let mut tx = self.pool.begin().await?;
        self.lock_writes(&mut tx).await?;
        let current: Option<i64> = sqlx::query_scalar(
            "SELECT session_version FROM rullst_recovery_accounts WHERE subject = $1",
        )
        .bind(&account.subject)
        .fetch_optional(&mut *tx)
        .await?;
        if current != Some(account.version) {
            return Err(RecoveryError::InvalidAction);
        }
        sqlx::query("DELETE FROM rullst_recovery_sessions WHERE subject = $1 AND expires_at <= $2")
            .bind(&account.subject)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM rullst_recovery_sessions WHERE subject = $1")
                .bind(&account.subject)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 20 {
            return Err(RecoveryError::Limited);
        }
        sqlx::query("INSERT INTO rullst_recovery_sessions (token_digest, subject, session_version, expires_at) VALUES ($1, $2, $3, $4)")
            .bind(self.keys.digest("session", token.expose())).bind(&account.subject).bind(account.version).bind(now + i64::from(lifetime_seconds))
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(token)
    }

    /// Reads authoritative revocation state; storage errors never authenticate.
    pub async fn verify_session(
        &self,
        token: &str,
        now: u64,
    ) -> Result<Option<String>, RecoveryError> {
        if token.len() != 43 {
            return Ok(None);
        }
        let subject = sqlx::query_scalar("SELECT s.subject FROM rullst_recovery_sessions s JOIN rullst_recovery_accounts a ON a.subject = s.subject AND a.session_version = s.session_version WHERE s.token_digest = $1 AND s.expires_at > $2")
            .bind(self.keys.digest("session", token)).bind(timestamp(now)?).fetch_optional(&self.pool).await?;
        Ok(subject)
    }

    /// Logout revokes this opaque session immediately across all users of the store.
    pub async fn revoke_session(&self, token: &str) -> Result<(), RecoveryError> {
        sqlx::query("DELETE FROM rullst_recovery_sessions WHERE token_digest = $1")
            .bind(self.keys.digest("session", token))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(super) async fn lock_writes(
        &self,
        tx: &mut Transaction<'_, Any>,
    ) -> Result<(), RecoveryError> {
        if sqlx::query("UPDATE rullst_recovery_control SET attempts = attempts WHERE id = 'write'")
            .execute(&mut **tx)
            .await?
            .rows_affected()
            != 1
        {
            return Err(RecoveryError::Configuration);
        }
        let binding: String =
            sqlx::query_scalar("SELECT binding FROM rullst_recovery_control WHERE id = 'write'")
                .fetch_one(&mut **tx)
                .await?;
        let expected = self.keys.digest("configuration", "v1");
        if self
            .keys
            .open("rullst.auth.config.v1", &binding)?
            .as_slice()
            != expected.as_bytes()
        {
            return Err(RecoveryError::Configuration);
        }
        Ok(())
    }

    pub(super) fn recipient(
        &self,
        subject: &str,
        encrypted: &str,
    ) -> Result<Zeroizing<String>, RecoveryError> {
        let bytes = self
            .keys
            .open(&format!("rullst.auth.account.v1:{subject}"), encrypted)?;
        std::str::from_utf8(&bytes)
            .map(|value| Zeroizing::new(value.to_owned()))
            .map_err(|_| RecoveryError::Crypto)
    }
}

pub(super) async fn password_hash(password: String) -> Result<String, RecoveryError> {
    if password.chars().count() < 12 || password.len() > 1024 {
        return Err(RecoveryError::InvalidInput);
    }
    crate::hash_password_async(password)
        .await
        .map_err(|_| RecoveryError::Crypto)
}
