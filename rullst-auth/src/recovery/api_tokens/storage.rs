use super::*;
use sqlx::{Any, Row, Transaction};
use std::sync::Arc;
use subtle::ConstantTimeEq;

const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS rullst_api_token_control (namespace TEXT PRIMARY \
                KEY,binding TEXT NOT NULL,capacity BIGINT NOT NULL CHECK(capacity BETWEEN 1 AND \
                100000),last_now BIGINT NOT NULL CHECK(last_now >= 0))",
    "CREATE TABLE IF NOT EXISTS rullst_api_tokens (namespace TEXT NOT NULL,id TEXT NOT \
                NULL,subject TEXT NOT NULL,token_digest TEXT NOT NULL,account_epoch BIGINT NOT NULL \
                CHECK(account_epoch > 0),scopes TEXT NOT NULL,label TEXT NOT NULL,revision BIGINT NOT \
                NULL CHECK(revision > 0),created_at BIGINT NOT NULL CHECK(created_at >= 0),issued_at \
                BIGINT NOT NULL,expires_at BIGINT NOT NULL,PRIMARY \
                KEY(namespace,id),UNIQUE(namespace,token_digest),CHECK(issued_at >= \
                created_at),CHECK(expires_at > issued_at))",
    "CREATE INDEX IF NOT EXISTS rullst_api_token_accounts ON \
                rullst_api_tokens(namespace,subject,expires_at)",
];

impl ApiTokenService {
    /// Explicit deployment bootstrap. No existing account receives an API token.
    pub async fn initialize(
        url: impl Into<String>,
        keys: RecoverySecrets,
        config: ApiTokenConfig,
    ) -> Result<Self, RecoveryError> {
        bounded(Self::open(url.into(), keys, config, true)).await
    }
    /// Opens existing namespace/configuration without DDL or missing-state repair.
    pub async fn connect(
        url: impl Into<String>,
        keys: RecoverySecrets,
        config: ApiTokenConfig,
    ) -> Result<Self, RecoveryError> {
        bounded(Self::open(url.into(), keys, config, false)).await
    }
    async fn open(
        url: String,
        keys: RecoverySecrets,
        config: ApiTokenConfig,
        initialize: bool,
    ) -> Result<Self, RecoveryError> {
        let (store, postgres) = super::super::connection::open(url, keys, initialize).await?;
        let service = Self {
            store,
            config,
            postgres,
        };
        let result = async {
            let bootstrap = if initialize && postgres {
                let mut tx = service.store.pool.begin().await?;
                sqlx::query("SELECT pg_catalog.pg_advisory_xact_lock(7311456720483293)")
                    .execute(&mut *tx)
                    .await?;
                Some(tx)
            } else {
                None
            };
            if initialize {
                service.store.migrate().await?;
            }
            let mut tx = service.store.pool.begin().await?;
            service.store.lock_writes(&mut tx).await?;
            if initialize {
                for ddl in SCHEMA {
                    sqlx::query(*ddl).execute(&mut *tx).await?;
                }
                sqlx::query(
                    "INSERT INTO rullst_api_token_control (namespace,binding,capacity,last_now)
                     VALUES ($1,$2,$3,0) ON CONFLICT(namespace) DO NOTHING",
                )
                .bind(&service.config.namespace)
                .bind(service.binding())
                .bind(service.config.capacity as i64)
                .execute(&mut *tx)
                .await?;
            }
            service.durable(&mut tx).await?;
            service.metadata(&mut tx).await?;
            tx.commit().await?;
            if let Some(tx) = bootstrap {
                tx.commit().await?;
            }
            Ok::<_, RecoveryError>(())
        }
        .await;
        if let Err(error) = result {
            service.store.close().await;
            return Err(error);
        }
        Ok(service)
    }
    /// The authoritative account registry. Management requires a proof returned
    /// by recent password authentication here plus the host's tenant/MFA approval.
    pub fn accounts(&self) -> &SqlRecoveryStore {
        &self.store
    }
    /// Closes this pool and every clone; independently opened pools are unaffected.
    pub async fn close(self) {
        self.store.close().await;
    }

    fn binding(&self) -> String {
        self.store.keys.digest(
            "api-token-configuration-v1",
            &format!(
                "{}\n{}\n{}\n{}",
                self.config.namespace,
                self.config.scopes.encode(),
                self.config.capacity,
                self.config.lifetime
            ),
        )
    }
    pub(super) fn digest(&self, bearer: &str) -> String {
        self.store.keys.digest(
            "api-token-v1",
            &format!("{}:{bearer}", self.config.namespace),
        )
    }
    async fn metadata(&self, tx: &mut Transaction<'_, Any>) -> Result<i64, RecoveryError> {
        let row = sqlx::query(
            "SELECT binding,capacity,last_now FROM rullst_api_token_control WHERE namespace = $1",
        )
        .bind(&self.config.namespace)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(RecoveryError::Configuration)?;
        let stored: String = row.try_get("binding")?;
        let expected = self.binding();
        let last: i64 = row.try_get("last_now")?;
        if !bool::from(stored.as_bytes().ct_eq(expected.as_bytes()))
            || row.try_get::<i64, _>("capacity")? != self.config.capacity as i64
            || last < 0
        {
            return Err(RecoveryError::Configuration);
        }
        Ok(last)
    }
    pub(super) async fn begin(
        &self,
        clock: &impl AuthClock,
    ) -> Result<(Transaction<'_, Any>, i64), RecoveryError> {
        let started = now(clock)?;
        let mut tx = self.store.pool.begin().await?;
        self.store.lock_writes(&mut tx).await?;
        let current = now(clock)?;
        if current < started {
            return Err(RecoveryError::InvalidAction);
        }
        self.observe(&mut tx, current).await?;
        Ok((tx, current))
    }
    pub(super) async fn observe(
        &self,
        tx: &mut Transaction<'_, Any>,
        current: i64,
    ) -> Result<(), RecoveryError> {
        self.durable(tx).await?;
        if current < self.metadata(tx).await? {
            return Err(RecoveryError::InvalidAction);
        }
        sqlx::query("UPDATE rullst_api_token_control SET last_now = $1 WHERE namespace = $2")
            .bind(current)
            .bind(&self.config.namespace)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
    pub(super) async fn finish(
        &self,
        tx: &mut Transaction<'_, Any>,
        clock: &impl AuthClock,
        started: i64,
        expiry: i64,
    ) -> Result<i64, RecoveryError> {
        let current = now(clock)?;
        if current < started || current >= expiry {
            return Err(RecoveryError::InvalidAction);
        }
        self.observe(tx, current).await?;
        Ok(current)
    }
    pub(super) async fn owner(
        &self,
        tx: &mut Transaction<'_, Any>,
        account: &AuthenticatedRecoveryAccount,
    ) -> Result<(), RecoveryError> {
        if !Arc::ptr_eq(&account.instance, &self.store.instance) {
            return Err(RecoveryError::InvalidAction);
        }
        let current: Option<i64> = sqlx::query_scalar(
            "SELECT session_version FROM rullst_recovery_accounts WHERE subject = $1",
        )
        .bind(&account.subject)
        .fetch_optional(&mut **tx)
        .await?;
        if current != Some(account.version) || account.version <= 0 {
            return Err(RecoveryError::InvalidAction);
        }
        Ok(())
    }
    pub(super) async fn purge(
        &self,
        tx: &mut Transaction<'_, Any>,
        current: i64,
    ) -> Result<(), RecoveryError> {
        sqlx::query("DELETE FROM rullst_api_tokens WHERE namespace = $1 AND (expires_at <= $2 OR NOT \
                EXISTS (SELECT 1 FROM rullst_recovery_accounts a WHERE a.subject = \
                rullst_api_tokens.subject AND a.session_version = rullst_api_tokens.account_epoch))")
            .bind(&self.config.namespace).bind(current).execute(&mut **tx).await?;
        Ok(())
    }
    /// Trusted retention task for expired credentials and obsolete account epochs.
    pub async fn purge_expired(&self, clock: &impl AuthClock) -> Result<(), RecoveryError> {
        bounded(async {
            let (mut tx, current) = self.begin(clock).await?;
            self.purge(&mut tx, current).await?;
            tx.commit().await?;
            Ok(())
        })
        .await
    }
    pub(super) async fn durable(&self, tx: &mut Transaction<'_, Any>) -> Result<(), RecoveryError> {
        if self.postgres {
            let durable: i64 = sqlx::query_scalar(
                "SELECT CASE WHEN pg_catalog.current_setting('fsync') = 'on' AND \
                pg_catalog.current_setting('full_page_writes') = 'on' AND \
                pg_catalog.current_setting('synchronous_commit') IN ('on','remote_apply') AND NOT \
                pg_catalog.pg_is_in_recovery() THEN 1::bigint ELSE 0::bigint END",
            )
            .fetch_one(&mut **tx)
            .await?;

            let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = \
                c.relnamespace WHERE n.nspname = 'public' AND c.relname IN \
                ('rullst_recovery_control','rullst_recovery_accounts','rullst_api_token_control','rullst_api_tokens') \
                AND c.relkind = 'r' AND c.relpersistence = 'p'").fetch_one(&mut **tx).await?;

            if durable != 1 || count != 4 {
                return Err(RecoveryError::Configuration);
            }
        } else {
            let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
                .fetch_one(&mut **tx)
                .await?;
            let sync: i64 = sqlx::query_scalar("PRAGMA synchronous")
                .fetch_one(&mut **tx)
                .await?;
            if !mode.eq_ignore_ascii_case("wal") || sync != 2 {
                return Err(RecoveryError::Configuration);
            }
        }
        Ok(())
    }
    pub(super) fn row(&self, row: &sqlx::any::AnyRow) -> Result<ApiTokenMetadata, RecoveryError> {
        let metadata = ApiTokenMetadata {
            id: ApiTokenId::new(row.try_get::<String, _>("id")?)
                .map_err(|_| RecoveryError::Configuration)?,
            label: SessionLabel::new(row.try_get::<String, _>("label")?)
                .map_err(|_| RecoveryError::Configuration)?,
            scopes: ApiScopes::decode(&row.try_get::<String, _>("scopes")?)?,
            revision: row.try_get("revision")?,
            created: row.try_get("created_at")?,
            issued: row.try_get("issued_at")?,
            expires: row.try_get("expires_at")?,
        };
        if metadata.revision < 1
            || metadata.created < 0
            || metadata.issued < metadata.created
            || metadata.expires <= metadata.issued
            || metadata.expires - metadata.issued > i64::from(self.config.lifetime)
            || !self.config.scopes.includes(&metadata.scopes)
        {
            return Err(RecoveryError::Configuration);
        }
        Ok(metadata)
    }
}
