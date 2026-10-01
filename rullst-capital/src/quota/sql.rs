//! Durable SQLx quota accounting for SQLite, PostgreSQL, MySQL, and MariaDB.

use super::{
    BillingSubject, QuotaError, QuotaGrant, QuotaRequest, QuotaStore, random_claim_token,
    tokens_match, validate_replay,
};
use async_trait::async_trait;
use rullst_orm::sqlx::{Any, AnyPool, Executor, Row, Transaction, any::AnyPoolOptions};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

mod mysql;
mod statements;

use statements::{
    decrement_counter_sql, delete_claim_sql, insert_claim_sql, insert_counter_sql, schema_sql,
    select_claim_sql, select_usage_sql, update_claim_usage_sql, update_counter_sql,
};

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
    // Set once the MySQL/MariaDB key columns were seen to compare case-sensitively.
    keys_verified: Arc<AtomicBool>,
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
        Ok(Self::from_pool(pool, backend))
    }

    /// Wraps an application-created pool with an explicit matching dialect.
    pub fn from_pool(pool: AnyPool, backend: SqlQuotaBackend) -> Self {
        Self {
            pool,
            backend,
            keys_verified: Arc::new(AtomicBool::new(false)),
        }
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
    ///
    /// On MySQL/MariaDB the subject, feature and event-key columns use the
    /// binary `ascii_bin` collation, so keys that differ only by letter case
    /// stay distinct as on PostgreSQL and SQLite. An existing table is never
    /// altered. If one of its key columns folds case (a table created by an
    /// earlier release), this method and every store operation return
    /// [`QuotaError::StorageUnavailable`] until the documented migration
    /// converts those columns.
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
        self.ensure_case_sensitive_keys(&self.pool).await
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
        self.ensure_case_sensitive_keys(&mut **transaction).await?;
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
        self.ensure_case_sensitive_keys(&mut **transaction).await?;
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

    /// Fails closed while a MySQL/MariaDB quota key column folds letter case.
    ///
    /// Only a successful check is remembered, so a store recovers once the
    /// operator has migrated a legacy table.
    async fn ensure_case_sensitive_keys<'e, E>(&self, executor: E) -> Result<(), QuotaError>
    where
        E: Executor<'e, Database = Any>,
    {
        if self.backend != SqlQuotaBackend::Mysql || self.keys_verified.load(Ordering::Acquire) {
            return Ok(());
        }
        let case_sensitive =
            rullst_orm::sqlx::query_scalar::<_, i64>(mysql::CASE_SENSITIVE_KEY_COLUMNS)
                .fetch_one(executor)
                .await
                .map_err(|_| QuotaError::StorageUnavailable)?;
        if case_sensitive != mysql::KEY_COLUMNS {
            return Err(QuotaError::StorageUnavailable);
        }
        self.keys_verified.store(true, Ordering::Release);
        Ok(())
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
        // Checked before the transaction starts, so the first read does not
        // open the MySQL/MariaDB snapshot ahead of the claim insert.
        self.ensure_case_sensitive_keys(&self.pool).await?;
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
        self.ensure_case_sensitive_keys(&self.pool).await?;
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
        self.ensure_case_sensitive_keys(&self.pool).await?;
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
