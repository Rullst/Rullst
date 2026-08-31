//! Durable SQLx quota accounting for SQLite, PostgreSQL, MySQL, and MariaDB.

use super::{
    BillingSubject, QuotaError, QuotaGrant, QuotaRequest, QuotaStore, random_claim_token,
    tokens_match, validate_replay,
};
use async_trait::async_trait;
use rullst_orm::sqlx::{Any, AnyPool, Row, Transaction, any::AnyPoolOptions};
use std::time::Duration;

/// SQL dialect used by [`SqlQuotaStore`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqlQuotaBackend {
    /// PostgreSQL wire protocol.
    Postgres,
    /// MySQL wire protocol, including MariaDB.
    Mysql,
    /// Local or file-backed SQLite.
    Sqlite,
}

/// Durable shared quota store backed by a dedicated SQLx `AnyPool`.
#[derive(Clone)]
#[non_exhaustive]
pub struct SqlQuotaStore {
    pool: AnyPool,
    backend: SqlQuotaBackend,
}

impl std::fmt::Debug for SqlQuotaStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqlQuotaStore")
            .field("backend", &self.backend)
            .finish_non_exhaustive()
    }
}

impl SqlQuotaStore {
    /// Connects to SQLite, PostgreSQL, MySQL, or MariaDB.
    ///
    /// Call [`Self::prepare_schema`] explicitly before serving traffic.
    pub async fn connect(database_url: impl Into<String>) -> Result<Self, QuotaError> {
        let database_url = database_url.into();
        let backend = backend_from_url(&database_url)?;
        rullst_orm::sqlx::any::install_default_drivers();
        let max_connections =
            if database_url.contains(":memory:") || database_url.contains("mode=memory") {
                1
            } else {
                5
            };
        let pool = AnyPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&database_url)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        Ok(Self { pool, backend })
    }

    /// Wraps an application-created pool with an explicit matching dialect.
    pub fn from_pool(pool: AnyPool, backend: SqlQuotaBackend) -> Self {
        Self { pool, backend }
    }

    /// Returns the selected SQL dialect.
    pub fn backend(&self) -> SqlQuotaBackend {
        self.backend
    }

    /// Returns the dedicated pool for health checks and caller-owned transactions.
    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    /// Creates the fixed-name quota counter and idempotency-claim tables.
    ///
    /// Release migrations should normally own this DDL. It is never run
    /// implicitly by a request path.
    pub async fn prepare_schema(&self) -> Result<(), QuotaError> {
        let (counters, claims) = schema_sql(self.backend);
        rullst_orm::sqlx::query(counters)
            .execute(&self.pool)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        rullst_orm::sqlx::query(claims)
            .execute(&self.pool)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        Ok(())
    }

    /// Reserves quota inside a caller-owned transaction.
    ///
    /// Insert the application resource through this same transaction and only
    /// then commit it. On any error, the caller must roll the transaction back.
    pub async fn reserve_with_transaction(
        &self,
        transaction: &mut Transaction<'_, Any>,
        request: &QuotaRequest,
    ) -> Result<QuotaGrant, QuotaError> {
        let claim_token = random_claim_token()?;
        rullst_orm::sqlx::query(insert_claim_sql(self.backend))
            .bind(request.subject.kind())
            .bind(request.subject.id())
            .bind(request.feature())
            .bind(request.event_key())
            .bind(to_i64(request.units())?)
            .bind(to_i64(request.limit())?)
            .bind(0_i64)
            .bind(&claim_token)
            .execute(&mut **transaction)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;

        let row = rullst_orm::sqlx::query(select_claim_sql(self.backend))
            .bind(request.subject.kind())
            .bind(request.subject.id())
            .bind(request.feature())
            .bind(request.event_key())
            .fetch_one(&mut **transaction)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        let stored_units = read_u64(&row, "units")?;
        let stored_limit = read_u64(&row, "limit_at_claim")?;
        let used_after = read_u64(&row, "used_after")?;
        let stored_token = row
            .try_get::<String, _>("claim_token")
            .map_err(|_| QuotaError::CorruptState)?;
        if !tokens_match(&stored_token, &claim_token) {
            validate_replay(stored_units, stored_limit, request)?;
            if used_after == 0 {
                return Err(QuotaError::CorruptState);
            }
            return Ok(QuotaGrant::replay(
                request.clone(),
                used_after,
                stored_token,
            ));
        }

        rullst_orm::sqlx::query(insert_counter_sql(self.backend))
            .bind(request.subject.kind())
            .bind(request.subject.id())
            .bind(request.feature())
            .bind(0_i64)
            .execute(&mut **transaction)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        let remaining_before =
            request
                .limit()
                .checked_sub(request.units())
                .ok_or(QuotaError::LimitExceeded {
                    used: self
                        .usage_with_transaction(transaction, request.subject(), request.feature())
                        .await?,
                    requested: request.units(),
                    limit: request.limit(),
                })?;
        let updated = rullst_orm::sqlx::query(update_counter_sql(self.backend))
            .bind(to_i64(request.units())?)
            .bind(request.subject.kind())
            .bind(request.subject.id())
            .bind(request.feature())
            .bind(to_i64(remaining_before)?)
            .execute(&mut **transaction)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        if updated.rows_affected() != 1 {
            let used = self
                .usage_with_transaction(transaction, request.subject(), request.feature())
                .await?;
            self.delete_claim_with_transaction(transaction, request, &claim_token)
                .await?;
            return Err(QuotaError::LimitExceeded {
                used,
                requested: request.units(),
                limit: request.limit(),
            });
        }
        let used_after = self
            .usage_with_transaction(transaction, request.subject(), request.feature())
            .await?;
        let written = rullst_orm::sqlx::query(update_claim_usage_sql(self.backend))
            .bind(to_i64(used_after)?)
            .bind(request.subject.kind())
            .bind(request.subject.id())
            .bind(request.feature())
            .bind(request.event_key())
            .bind(&claim_token)
            .execute(&mut **transaction)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        if written.rows_affected() != 1 {
            return Err(QuotaError::CorruptState);
        }
        Ok(QuotaGrant::fresh(request.clone(), used_after, claim_token))
    }

    /// Releases a grant inside a caller-owned transaction.
    pub async fn release_with_transaction(
        &self,
        transaction: &mut Transaction<'_, Any>,
        grant: &QuotaGrant,
    ) -> Result<bool, QuotaError> {
        let request = grant.request();
        let row = rullst_orm::sqlx::query(select_claim_sql(self.backend))
            .bind(request.subject.kind())
            .bind(request.subject.id())
            .bind(request.feature())
            .bind(request.event_key())
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        let Some(row) = row else {
            return Ok(false);
        };
        let token = row
            .try_get::<String, _>("claim_token")
            .map_err(|_| QuotaError::CorruptState)?;
        if !tokens_match(&token, &grant.claim_token) {
            return Err(QuotaError::GrantMismatch);
        }
        let units = read_u64(&row, "units")?;
        let deleted = self
            .delete_claim_with_transaction(transaction, request, &token)
            .await?;
        if !deleted {
            return Err(QuotaError::CorruptState);
        }
        let decremented = rullst_orm::sqlx::query(decrement_counter_sql(self.backend))
            .bind(to_i64(units)?)
            .bind(request.subject.kind())
            .bind(request.subject.id())
            .bind(request.feature())
            .bind(to_i64(units)?)
            .execute(&mut **transaction)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        if decremented.rows_affected() != 1 {
            return Err(QuotaError::CorruptState);
        }
        Ok(true)
    }

    async fn usage_with_transaction(
        &self,
        transaction: &mut Transaction<'_, Any>,
        subject: &BillingSubject,
        feature: &str,
    ) -> Result<u64, QuotaError> {
        let value = rullst_orm::sqlx::query_scalar::<_, i64>(select_usage_sql(self.backend))
            .bind(subject.kind())
            .bind(subject.id())
            .bind(feature)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?
            .unwrap_or(0);
        u64::try_from(value).map_err(|_| QuotaError::CorruptState)
    }

    async fn delete_claim_with_transaction(
        &self,
        transaction: &mut Transaction<'_, Any>,
        request: &QuotaRequest,
        claim_token: &str,
    ) -> Result<bool, QuotaError> {
        let deleted = rullst_orm::sqlx::query(delete_claim_sql(self.backend))
            .bind(request.subject.kind())
            .bind(request.subject.id())
            .bind(request.feature())
            .bind(request.event_key())
            .bind(claim_token)
            .execute(&mut **transaction)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        Ok(deleted.rows_affected() == 1)
    }
}

#[async_trait]
impl QuotaStore for SqlQuotaStore {
    async fn reserve(&self, request: &QuotaRequest) -> Result<QuotaGrant, QuotaError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        match self
            .reserve_with_transaction(&mut transaction, request)
            .await
        {
            Ok(grant) => {
                transaction
                    .commit()
                    .await
                    .map_err(|_| QuotaError::StorageUnavailable)?;
                Ok(grant)
            }
            Err(error) => {
                transaction
                    .rollback()
                    .await
                    .map_err(|_| QuotaError::StorageUnavailable)?;
                Err(error)
            }
        }
    }

    async fn release(&self, grant: &QuotaGrant) -> Result<bool, QuotaError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?;
        match self.release_with_transaction(&mut transaction, grant).await {
            Ok(released) => {
                transaction
                    .commit()
                    .await
                    .map_err(|_| QuotaError::StorageUnavailable)?;
                Ok(released)
            }
            Err(error) => {
                transaction
                    .rollback()
                    .await
                    .map_err(|_| QuotaError::StorageUnavailable)?;
                Err(error)
            }
        }
    }

    async fn usage(&self, subject: &BillingSubject, feature: &str) -> Result<u64, QuotaError> {
        super::validate_identifier("quota feature", feature, super::MAX_FEATURE_BYTES)?;
        let value = rullst_orm::sqlx::query_scalar::<_, i64>(select_usage_sql(self.backend))
            .bind(subject.kind())
            .bind(subject.id())
            .bind(feature)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| QuotaError::StorageUnavailable)?
            .unwrap_or(0);
        u64::try_from(value).map_err(|_| QuotaError::CorruptState)
    }
}

fn backend_from_url(database_url: &str) -> Result<SqlQuotaBackend, QuotaError> {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        Ok(SqlQuotaBackend::Postgres)
    } else if database_url.starts_with("mysql://") {
        Ok(SqlQuotaBackend::Mysql)
    } else if database_url.starts_with("sqlite:") {
        Ok(SqlQuotaBackend::Sqlite)
    } else {
        Err(QuotaError::InvalidRequest(
            "SQL quota requires a PostgreSQL, MySQL/MariaDB, or SQLite URL".to_string(),
        ))
    }
}

fn read_u64(row: &rullst_orm::sqlx::any::AnyRow, column: &str) -> Result<u64, QuotaError> {
    let value = row
        .try_get::<i64, _>(column)
        .map_err(|_| QuotaError::CorruptState)?;
    u64::try_from(value).map_err(|_| QuotaError::CorruptState)
}

fn to_i64(value: u64) -> Result<i64, QuotaError> {
    i64::try_from(value).map_err(|_| QuotaError::InvalidRequest("quota quantity overflow".into()))
}

fn schema_sql(backend: SqlQuotaBackend) -> (&'static str, &'static str) {
    match backend {
        SqlQuotaBackend::Postgres => (POSTGRES_COUNTERS, POSTGRES_CLAIMS),
        SqlQuotaBackend::Mysql => (MYSQL_COUNTERS, MYSQL_CLAIMS),
        SqlQuotaBackend::Sqlite => (SQLITE_COUNTERS, SQLITE_CLAIMS),
    }
}

fn insert_claim_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_INSERT_CLAIM,
        SqlQuotaBackend::Mysql => MYSQL_INSERT_CLAIM,
        SqlQuotaBackend::Sqlite => SQLITE_INSERT_CLAIM,
    }
}

fn insert_counter_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_INSERT_COUNTER,
        SqlQuotaBackend::Mysql => MYSQL_INSERT_COUNTER,
        SqlQuotaBackend::Sqlite => SQLITE_INSERT_COUNTER,
    }
}

fn select_claim_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_SELECT_CLAIM,
        _ => PORTABLE_SELECT_CLAIM,
    }
}

fn update_counter_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_UPDATE_COUNTER,
        _ => PORTABLE_UPDATE_COUNTER,
    }
}

fn update_claim_usage_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_UPDATE_CLAIM_USAGE,
        _ => PORTABLE_UPDATE_CLAIM_USAGE,
    }
}

fn select_usage_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_SELECT_USAGE,
        _ => PORTABLE_SELECT_USAGE,
    }
}

fn delete_claim_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_DELETE_CLAIM,
        _ => PORTABLE_DELETE_CLAIM,
    }
}

fn decrement_counter_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_DECREMENT_COUNTER,
        _ => PORTABLE_DECREMENT_COUNTER,
    }
}

const POSTGRES_COUNTERS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_counters (subject_kind VARCHAR(32) NOT NULL, subject_id VARCHAR(128) NOT NULL, feature VARCHAR(128) NOT NULL, used_units BIGINT NOT NULL DEFAULT 0 CHECK (used_units >= 0), PRIMARY KEY (subject_kind, subject_id, feature))";
const POSTGRES_CLAIMS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_claims (subject_kind VARCHAR(32) NOT NULL, subject_id VARCHAR(128) NOT NULL, feature VARCHAR(128) NOT NULL, event_key VARCHAR(128) NOT NULL, units BIGINT NOT NULL CHECK (units > 0), limit_at_claim BIGINT NOT NULL CHECK (limit_at_claim > 0), used_after BIGINT NOT NULL CHECK (used_after >= 0), claim_token VARCHAR(32) NOT NULL, PRIMARY KEY (subject_kind, subject_id, feature, event_key))";
const MYSQL_COUNTERS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_counters (subject_kind VARCHAR(32) NOT NULL, subject_id VARCHAR(128) NOT NULL, feature VARCHAR(128) NOT NULL, used_units BIGINT NOT NULL DEFAULT 0 CHECK (used_units >= 0), PRIMARY KEY (subject_kind, subject_id, feature)) ENGINE=InnoDB";
const MYSQL_CLAIMS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_claims (subject_kind VARCHAR(32) NOT NULL, subject_id VARCHAR(128) NOT NULL, feature VARCHAR(128) NOT NULL, event_key VARCHAR(128) NOT NULL, units BIGINT NOT NULL CHECK (units > 0), limit_at_claim BIGINT NOT NULL CHECK (limit_at_claim > 0), used_after BIGINT NOT NULL CHECK (used_after >= 0), claim_token VARCHAR(32) NOT NULL, PRIMARY KEY (subject_kind, subject_id, feature, event_key)) ENGINE=InnoDB";
const SQLITE_COUNTERS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_counters (subject_kind TEXT NOT NULL, subject_id TEXT NOT NULL, feature TEXT NOT NULL, used_units INTEGER NOT NULL DEFAULT 0 CHECK (used_units >= 0), PRIMARY KEY (subject_kind, subject_id, feature))";
const SQLITE_CLAIMS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_claims (subject_kind TEXT NOT NULL, subject_id TEXT NOT NULL, feature TEXT NOT NULL, event_key TEXT NOT NULL, units INTEGER NOT NULL CHECK (units > 0), limit_at_claim INTEGER NOT NULL CHECK (limit_at_claim > 0), used_after INTEGER NOT NULL CHECK (used_after >= 0), claim_token TEXT NOT NULL, PRIMARY KEY (subject_kind, subject_id, feature, event_key))";

const POSTGRES_INSERT_CLAIM: &str = "INSERT INTO rullst_capital_quota_claims (subject_kind, subject_id, feature, event_key, units, limit_at_claim, used_after, claim_token) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (subject_kind, subject_id, feature, event_key) DO NOTHING";
const MYSQL_INSERT_CLAIM: &str = "INSERT INTO rullst_capital_quota_claims (subject_kind, subject_id, feature, event_key, units, limit_at_claim, used_after, claim_token) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE subject_id = VALUES(subject_id)";
const SQLITE_INSERT_CLAIM: &str = "INSERT INTO rullst_capital_quota_claims (subject_kind, subject_id, feature, event_key, units, limit_at_claim, used_after, claim_token) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT (subject_kind, subject_id, feature, event_key) DO NOTHING";
const POSTGRES_INSERT_COUNTER: &str = "INSERT INTO rullst_capital_quota_counters (subject_kind, subject_id, feature, used_units) VALUES ($1, $2, $3, $4) ON CONFLICT (subject_kind, subject_id, feature) DO NOTHING";
const MYSQL_INSERT_COUNTER: &str = "INSERT INTO rullst_capital_quota_counters (subject_kind, subject_id, feature, used_units) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE subject_id = VALUES(subject_id)";
const SQLITE_INSERT_COUNTER: &str = "INSERT INTO rullst_capital_quota_counters (subject_kind, subject_id, feature, used_units) VALUES (?, ?, ?, ?) ON CONFLICT (subject_kind, subject_id, feature) DO NOTHING";

const POSTGRES_SELECT_CLAIM: &str = "SELECT units, limit_at_claim, used_after, claim_token FROM rullst_capital_quota_claims WHERE subject_kind = $1 AND subject_id = $2 AND feature = $3 AND event_key = $4";
const PORTABLE_SELECT_CLAIM: &str = "SELECT units, limit_at_claim, used_after, claim_token FROM rullst_capital_quota_claims WHERE subject_kind = ? AND subject_id = ? AND feature = ? AND event_key = ?";
const POSTGRES_UPDATE_COUNTER: &str = "UPDATE rullst_capital_quota_counters SET used_units = used_units + $1 WHERE subject_kind = $2 AND subject_id = $3 AND feature = $4 AND used_units <= $5";
const PORTABLE_UPDATE_COUNTER: &str = "UPDATE rullst_capital_quota_counters SET used_units = used_units + ? WHERE subject_kind = ? AND subject_id = ? AND feature = ? AND used_units <= ?";
const POSTGRES_UPDATE_CLAIM_USAGE: &str = "UPDATE rullst_capital_quota_claims SET used_after = $1 WHERE subject_kind = $2 AND subject_id = $3 AND feature = $4 AND event_key = $5 AND claim_token = $6";
const PORTABLE_UPDATE_CLAIM_USAGE: &str = "UPDATE rullst_capital_quota_claims SET used_after = ? WHERE subject_kind = ? AND subject_id = ? AND feature = ? AND event_key = ? AND claim_token = ?";
const POSTGRES_SELECT_USAGE: &str = "SELECT used_units FROM rullst_capital_quota_counters WHERE subject_kind = $1 AND subject_id = $2 AND feature = $3";
const PORTABLE_SELECT_USAGE: &str = "SELECT used_units FROM rullst_capital_quota_counters WHERE subject_kind = ? AND subject_id = ? AND feature = ?";
const POSTGRES_DELETE_CLAIM: &str = "DELETE FROM rullst_capital_quota_claims WHERE subject_kind = $1 AND subject_id = $2 AND feature = $3 AND event_key = $4 AND claim_token = $5";
const PORTABLE_DELETE_CLAIM: &str = "DELETE FROM rullst_capital_quota_claims WHERE subject_kind = ? AND subject_id = ? AND feature = ? AND event_key = ? AND claim_token = ?";
const POSTGRES_DECREMENT_COUNTER: &str = "UPDATE rullst_capital_quota_counters SET used_units = used_units - $1 WHERE subject_kind = $2 AND subject_id = $3 AND feature = $4 AND used_units >= $5";
const PORTABLE_DECREMENT_COUNTER: &str = "UPDATE rullst_capital_quota_counters SET used_units = used_units - ? WHERE subject_kind = ? AND subject_id = ? AND feature = ? AND used_units >= ?";
