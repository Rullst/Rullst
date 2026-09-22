use super::*;
use sqlx::{Any, Row, Transaction};
use std::sync::Arc;
use subtle::ConstantTimeEq;

const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS rullst_email_login_control (namespace TEXT PRIMARY KEY, binding TEXT NOT NULL, capacity BIGINT NOT NULL, last_now BIGINT NOT NULL, request_window BIGINT NOT NULL, requests BIGINT NOT NULL, consume_window BIGINT NOT NULL, consumes BIGINT NOT NULL)",
    "CREATE TABLE IF NOT EXISTS rullst_email_login_accounts (namespace TEXT NOT NULL, subject TEXT NOT NULL, enabled BIGINT NOT NULL, revision BIGINT NOT NULL, request_window BIGINT NOT NULL, requests BIGINT NOT NULL, PRIMARY KEY(namespace, subject))",
    "CREATE TABLE IF NOT EXISTS rullst_email_login_tokens (namespace TEXT NOT NULL, subject TEXT NOT NULL, token_digest TEXT NOT NULL, browser_digest TEXT NOT NULL, account_epoch BIGINT NOT NULL, policy_revision BIGINT NOT NULL, issued_at BIGINT NOT NULL, expires_at BIGINT NOT NULL, PRIMARY KEY(namespace,subject), UNIQUE(namespace,token_digest))",
    "CREATE TABLE IF NOT EXISTS rullst_email_login_outbox (id TEXT PRIMARY KEY, namespace TEXT NOT NULL, subject TEXT NOT NULL, ciphertext TEXT NOT NULL, expires_at BIGINT NOT NULL, status TEXT NOT NULL, attempts BIGINT NOT NULL, due_at BIGINT NOT NULL, lease TEXT NOT NULL, lease_until BIGINT NOT NULL)",
    "CREATE INDEX IF NOT EXISTS rullst_email_login_due ON rullst_email_login_outbox(namespace,status,due_at)",
];

impl EmailLoginService {
    /// Explicit deployment bootstrap for the account registry and email-login
    /// tables. Existing accounts are preserved and do not become opted in.
    pub async fn initialize(
        url: impl Into<String>,
        keys: RecoverySecrets,
        config: EmailLoginConfig,
    ) -> Result<Self, RecoveryError> {
        bounded(Self::open(url.into(), keys, config, true)).await
    }

    /// Connects without DDL or missing-state repair. The private pool requires
    /// certificate/hostname-verified remote PostgreSQL transport and bounded SQL.
    pub async fn connect(
        url: impl Into<String>,
        keys: RecoverySecrets,
        config: EmailLoginConfig,
    ) -> Result<Self, RecoveryError> {
        bounded(Self::open(url.into(), keys, config, false)).await
    }

    async fn open(
        url: String,
        keys: RecoverySecrets,
        config: EmailLoginConfig,
        initialize: bool,
    ) -> Result<Self, RecoveryError> {
        let (store, postgres) = connection::open(url, keys, initialize).await?;
        let service = Self {
            store,
            config,
            postgres,
        };
        let result = async {
            // Serialize concurrent explicit bootstraps before recovery DDL too.
            let bootstrap = if initialize && postgres {
                let mut connection = service.store.pool.begin().await?;
                sqlx::query("SELECT pg_catalog.pg_advisory_xact_lock(7311456720483293)").execute(&mut *connection).await?;
                Some(connection)
            } else { None };
            if initialize { service.store.migrate().await?; }
            let mut tx = service.store.pool.begin().await?;
            service.store.lock_writes(&mut tx).await?;
            if initialize {
                for statement in SCHEMA { sqlx::query(*statement).execute(&mut *tx).await?; }
                sqlx::query("INSERT INTO rullst_email_login_control (namespace,binding,capacity,last_now,request_window,requests,consume_window,consumes) VALUES ($1,$2,$3,0,0,0,0,0) ON CONFLICT(namespace) DO NOTHING")
                    .bind(&service.config.namespace).bind(service.configuration_binding()).bind(service.config.capacity as i64).execute(&mut *tx).await?;
            }
            service.durable(&mut tx).await?;
            service.metadata(&mut tx).await?;
            tx.commit().await?;
            if let Some(connection) = bootstrap { connection.commit().await?; }
            Ok::<_,RecoveryError>(())
        }.await;
        if let Err(error) = result {
            service.store.close().await;
            return Err(error);
        }
        Ok(service)
    }

    /// Existing account/password/session lifecycle, using this same private pool.
    /// Host-owned profile records and tenant membership remain separate.
    pub fn accounts(&self) -> &SqlRecoveryStore {
        &self.store
    }

    pub async fn close(self) {
        self.store.close().await;
    }

    fn configuration_binding(&self) -> String {
        let value = format!(
            "{}\n{}\n{}\n{}\n{}",
            self.config.namespace,
            self.config.landing,
            self.config.destination,
            self.config.capacity,
            self.config.development
        );
        self.store
            .keys
            .digest("email-login-configuration-v1", &value)
    }

    pub(super) async fn metadata(
        &self,
        tx: &mut Transaction<'_, Any>,
    ) -> Result<i64, RecoveryError> {
        let row = sqlx::query(
            "SELECT binding,capacity,last_now FROM rullst_email_login_control WHERE namespace = $1",
        )
        .bind(&self.config.namespace)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(RecoveryError::Configuration)?;
        let binding: String = row.try_get("binding")?;
        let expected = self.configuration_binding();
        let now: i64 = row.try_get("last_now")?;
        if !bool::from(binding.as_bytes().ct_eq(expected.as_bytes()))
            || row.try_get::<i64, _>("capacity")? != self.config.capacity as i64
            || now < 0
        {
            return Err(RecoveryError::Configuration);
        }
        Ok(now)
    }

    pub(super) async fn observe(
        &self,
        tx: &mut Transaction<'_, Any>,
        now: i64,
    ) -> Result<(), RecoveryError> {
        self.durable(tx).await?;
        if now < self.metadata(tx).await? {
            return Err(RecoveryError::InvalidAction);
        }
        sqlx::query("UPDATE rullst_email_login_control SET last_now = $1 WHERE namespace = $2")
            .bind(now)
            .bind(&self.config.namespace)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    /// Enables/disables this account only after the host's recent authentication,
    /// tenant and MFA policy. An authentication proof from another pool or an old
    /// account epoch is rejected. A change invalidates all pending login links.
    pub async fn set_account_enabled(
        &self,
        account: &AuthenticatedRecoveryAccount,
        enabled: bool,
        clock: &impl EmailLoginClock,
    ) -> Result<(), RecoveryError> {
        bounded(self.set_enabled(account, enabled, clock)).await
    }

    async fn set_enabled(
        &self,
        account: &AuthenticatedRecoveryAccount,
        enabled: bool,
        clock: &impl EmailLoginClock,
    ) -> Result<(), RecoveryError> {
        if !Arc::ptr_eq(&account.instance, &self.store.instance) {
            return Err(RecoveryError::InvalidAction);
        }
        let (mut tx, now) = self.begin(clock).await?;
        let epoch: Option<i64> = sqlx::query_scalar(
            "SELECT session_version FROM rullst_recovery_accounts WHERE subject = $1",
        )
        .bind(&account.subject)
        .fetch_optional(&mut *tx)
        .await?;
        if epoch != Some(account.version) {
            return Err(RecoveryError::InvalidAction);
        }
        let previous: Option<i64> = sqlx::query_scalar("SELECT revision FROM rullst_email_login_accounts WHERE namespace = $1 AND subject = $2")
            .bind(&self.config.namespace).bind(&account.subject).fetch_optional(&mut *tx).await?;
        if previous.is_none() {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM rullst_email_login_accounts WHERE namespace = $1",
            )
            .bind(&self.config.namespace)
            .fetch_one(&mut *tx)
            .await?;
            if count >= self.config.capacity as i64 {
                return Err(RecoveryError::Limited);
            }
        }
        if previous.is_some_and(|value| value < 1) {
            return Err(RecoveryError::Configuration);
        }
        let revision = previous
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(RecoveryError::Limited)?;
        sqlx::query("INSERT INTO rullst_email_login_accounts (namespace,subject,enabled,revision,request_window,requests) VALUES ($1,$2,$3,$4,0,0) ON CONFLICT(namespace,subject) DO UPDATE SET enabled=excluded.enabled,revision=excluded.revision")
            .bind(&self.config.namespace).bind(&account.subject).bind(i64::from(enabled)).bind(revision).execute(&mut *tx).await?;
        self.cancel_pending(&mut tx, &account.subject).await?;
        self.finish_time(&mut tx, clock, now, i64::MAX).await?;
        tx.commit().await?;
        if super::super::timestamp(clock.now()?)? < now {
            return Err(RecoveryError::InvalidAction);
        }
        Ok(())
    }

    pub(super) async fn cancel_pending(
        &self,
        tx: &mut Transaction<'_, Any>,
        subject: &str,
    ) -> Result<(), RecoveryError> {
        sqlx::query("DELETE FROM rullst_email_login_tokens WHERE namespace = $1 AND subject = $2")
            .bind(&self.config.namespace)
            .bind(subject)
            .execute(&mut **tx)
            .await?;
        sqlx::query("DELETE FROM rullst_email_login_outbox WHERE namespace = $1 AND subject = $2 AND status IN ('pending','leased')")
            .bind(&self.config.namespace).bind(subject).execute(&mut **tx).await?;
        Ok(())
    }
}
