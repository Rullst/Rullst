//! Durable relational webhook replay claims.

use super::{event_key_hash, payload_key, validate_replay_event_key, validate_replay_provider};
use crate::CapitalError;
use rullst_orm::sqlx::{Any, AnyPool, Row, Transaction, any::AnyPoolOptions};
use std::time::Duration;

mod statements;

use statements::{
    active_count_sql, claim_expiry_index_sql, claim_schema_sql, config_schema_sql,
    contains_claim_sql, delete_expired_sql, insert_claim_sql, insert_config_sql,
    lock_sqlite_config_sql, select_config_for_update_sql, select_config_sql, timestamp_sql,
};

/// SQL dialect used by [`SqlWebhookReplayStore`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqlWebhookBackend {
    /// PostgreSQL wire protocol.
    Postgres,
    /// MySQL wire protocol, including MariaDB.
    Mysql,
    /// Local or file-backed SQLite.
    Sqlite,
}

/// Bounded durable webhook replay ledger for SQLite, PostgreSQL, MySQL, and MariaDB.
///
/// The fixed configuration row serializes claims and prevents two processes from
/// accepting the same provider/payload digest concurrently. Active claims are
/// retained until their configured TTL expires; reaching capacity fails closed
/// instead of evicting a still-active replay proof. Expiry uses the selected
/// database's transaction-time clock rather than an individual process clock.
#[derive(Clone)]
#[non_exhaustive]
pub struct SqlWebhookReplayStore {
    pool: AnyPool,
    backend: SqlWebhookBackend,
    max_entries: usize,
    ttl: Duration,
}

impl std::fmt::Debug for SqlWebhookReplayStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqlWebhookReplayStore")
            .field("backend", &self.backend)
            .field("max_entries", &self.max_entries)
            .field("ttl", &self.ttl)
            .finish_non_exhaustive()
    }
}

impl SqlWebhookReplayStore {
    /// Connects to a supported relational database without mutating its schema.
    pub async fn connect(
        database_url: impl Into<String>,
        max_entries: usize,
        ttl: Duration,
    ) -> Result<Self, CapitalError> {
        validate_profile(max_entries, ttl)?;
        let database_url = database_url.into();
        let backend = backend_from_url(&database_url)?;
        rullst_orm::sqlx::any::install_default_drivers();
        let options = AnyPoolOptions::new().acquire_timeout(Duration::from_secs(10));
        let options = if database_url.contains(":memory:") || database_url.contains("mode=memory") {
            // A replacement connection would open a fresh empty database, so
            // keep the single connection instead of retiring it when idle or old.
            options
                .max_connections(1)
                .min_connections(1)
                .idle_timeout(None)
                .max_lifetime(None)
        } else {
            options.max_connections(5)
        };
        let pool = options
            .connect(&database_url)
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
        Ok(Self {
            pool,
            backend,
            max_entries,
            ttl,
        })
    }

    /// Wraps a caller-created pool after validating the immutable store profile.
    pub fn from_pool(
        pool: AnyPool,
        backend: SqlWebhookBackend,
        max_entries: usize,
        ttl: Duration,
    ) -> Result<Self, CapitalError> {
        validate_profile(max_entries, ttl)?;
        Ok(Self {
            pool,
            backend,
            max_entries,
            ttl,
        })
    }

    /// Returns the selected SQL dialect.
    pub fn backend(&self) -> SqlWebhookBackend {
        self.backend
    }

    /// Returns the pool for health checks and caller-owned transactions.
    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    /// Closes the underlying pool and waits for its connections to finish.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Creates the fixed replay tables and validates existing immutable metadata.
    ///
    /// Release migrations should normally own this DDL. It is never executed by
    /// a webhook request path.
    pub async fn prepare_schema(&self) -> Result<(), CapitalError> {
        rullst_orm::sqlx::query(config_schema_sql(self.backend))
            .execute(&self.pool)
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
        rullst_orm::sqlx::query(claim_schema_sql(self.backend))
            .execute(&self.pool)
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
        if let Some(index_sql) = claim_expiry_index_sql(self.backend) {
            rullst_orm::sqlx::query(index_sql)
                .execute(&self.pool)
                .await
                .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
        }
        rullst_orm::sqlx::query(insert_config_sql(self.backend))
            .bind(to_i64(self.max_entries)?)
            .bind(to_i64(self.ttl.as_secs())?)
            .execute(&self.pool)
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
        self.validate_persisted_profile().await
    }

    /// Atomically persists a provider-scoped payload digest in its own transaction.
    pub async fn check_and_record_payload(
        &self,
        provider: &str,
        payload: &[u8],
    ) -> Result<(), CapitalError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
        let accepted_at = self.database_timestamp(&mut transaction).await?;
        let result = self
            .check_and_record_at(&mut transaction, provider, payload, accepted_at)
            .await;
        finish_transaction(transaction, result).await
    }

    /// Persists a replay claim in a caller-owned transaction.
    ///
    /// The transaction must originate from this store's pool and use its selected
    /// dialect. An application may write its domain side effects through the same
    /// transaction before committing, which removes the claim/side-effect crash
    /// window for one relational database. Cross-system effects still require an
    /// outbox and reconciliation.
    pub async fn check_and_record_payload_with_transaction(
        &self,
        transaction: &mut Transaction<'_, Any>,
        provider: &str,
        payload: &[u8],
    ) -> Result<(), CapitalError> {
        let accepted_at = self.database_timestamp(transaction).await?;
        self.check_and_record_at(transaction, provider, payload, accepted_at)
            .await
    }

    /// Atomically persists a provider's stable semantic event identifier.
    ///
    /// Prefer this over payload-only replay detection when the verified provider
    /// protocol supplies a stable event ID.
    pub async fn check_and_record_event_key(
        &self,
        provider: &str,
        event_key: &str,
    ) -> Result<(), CapitalError> {
        validate_replay_provider(provider)?;
        validate_replay_event_key(event_key)?;
        let key = event_key_hash(provider, event_key);
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
        let accepted_at = self.database_timestamp(&mut transaction).await?;
        let result = self
            .check_and_record_key_at(&mut transaction, provider, &key, accepted_at)
            .await;
        finish_transaction(transaction, result).await
    }

    /// Persists a stable provider event ID with domain effects in one transaction.
    pub async fn check_and_record_event_key_with_transaction(
        &self,
        transaction: &mut Transaction<'_, Any>,
        provider: &str,
        event_key: &str,
    ) -> Result<(), CapitalError> {
        validate_replay_provider(provider)?;
        validate_replay_event_key(event_key)?;
        let accepted_at = self.database_timestamp(transaction).await?;
        self.check_and_record_key_at(
            transaction,
            provider,
            &event_key_hash(provider, event_key),
            accepted_at,
        )
        .await
    }

    async fn check_and_record_at(
        &self,
        transaction: &mut Transaction<'_, Any>,
        provider: &str,
        payload: &[u8],
        accepted_at: u64,
    ) -> Result<(), CapitalError> {
        validate_replay_provider(provider)?;
        if payload.is_empty() || payload.len() > super::MAX_WEBHOOK_PAYLOAD_BYTES {
            return Err(CapitalError::ConfigurationError(
                "Webhook replay payload must contain 1 byte through the configured body limit"
                    .to_string(),
            ));
        }
        self.check_and_record_key_at(
            transaction,
            provider,
            &payload_key(provider, payload),
            accepted_at,
        )
        .await
    }

    async fn check_and_record_key_at(
        &self,
        transaction: &mut Transaction<'_, Any>,
        provider: &str,
        key: &str,
        accepted_at: u64,
    ) -> Result<(), CapitalError> {
        self.lock_and_validate_profile(transaction).await?;

        let accepted_at = to_i64(accepted_at)?;
        let expires_at = accepted_at
            .checked_add(to_i64(self.ttl.as_secs())?)
            .ok_or(CapitalError::WebhookReplayCorruptState)?;
        rullst_orm::sqlx::query(delete_expired_sql(self.backend))
            .bind(accepted_at)
            .execute(&mut **transaction)
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;

        let already_present =
            rullst_orm::sqlx::query_scalar::<_, i64>(contains_claim_sql(self.backend))
                .bind(provider)
                .bind(key)
                .fetch_optional(&mut **transaction)
                .await
                .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?
                .is_some();
        if already_present {
            return Err(CapitalError::WebhookReplay(key.to_string()));
        }

        let active = rullst_orm::sqlx::query_scalar::<_, i64>(active_count_sql())
            .fetch_one(&mut **transaction)
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
        let active =
            usize::try_from(active).map_err(|_| CapitalError::WebhookReplayCorruptState)?;
        if active >= self.max_entries {
            return Err(CapitalError::WebhookReplayStoreFull);
        }

        let inserted = match rullst_orm::sqlx::query(insert_claim_sql(self.backend))
            .bind(provider)
            .bind(key)
            .bind(accepted_at)
            .bind(expires_at)
            .execute(&mut **transaction)
            .await
        {
            Ok(inserted) => inserted,
            // A caller-owned MySQL/MariaDB REPEATABLE READ transaction can miss
            // a claim committed after its snapshot in the SELECT above; the
            // plain INSERT then reports it as a duplicate key.
            Err(rullst_orm::sqlx::Error::Database(error)) if error.is_unique_violation() => {
                return Err(CapitalError::WebhookReplay(key.to_string()));
            }
            Err(_) => return Err(CapitalError::WebhookReplayStoreUnavailable),
        };
        if inserted.rows_affected() != 1 {
            return Err(CapitalError::WebhookReplay(key.to_string()));
        }
        Ok(())
    }

    async fn lock_and_validate_profile(
        &self,
        transaction: &mut Transaction<'_, Any>,
    ) -> Result<(), CapitalError> {
        if self.backend == SqlWebhookBackend::Sqlite {
            let locked = rullst_orm::sqlx::query(lock_sqlite_config_sql())
                .execute(&mut **transaction)
                .await
                .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
            if locked.rows_affected() != 1 {
                return Err(CapitalError::WebhookReplayCorruptState);
            }
        }
        let query = if self.backend == SqlWebhookBackend::Sqlite {
            select_config_sql()
        } else {
            select_config_for_update_sql()
        };
        let row = rullst_orm::sqlx::query(query)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?
            .ok_or(CapitalError::WebhookReplayCorruptState)?;
        validate_profile_row(&row, self.max_entries, self.ttl)
    }

    async fn validate_persisted_profile(&self) -> Result<(), CapitalError> {
        let row = rullst_orm::sqlx::query(select_config_sql())
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?
            .ok_or(CapitalError::WebhookReplayCorruptState)?;
        validate_profile_row(&row, self.max_entries, self.ttl)
    }

    async fn database_timestamp(
        &self,
        transaction: &mut Transaction<'_, Any>,
    ) -> Result<u64, CapitalError> {
        let timestamp = rullst_orm::sqlx::query_scalar::<_, i64>(timestamp_sql(self.backend))
            .fetch_one(&mut **transaction)
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
        u64::try_from(timestamp).map_err(|_| CapitalError::WebhookReplayCorruptState)
    }
}

async fn finish_transaction(
    transaction: Transaction<'_, Any>,
    result: Result<(), CapitalError>,
) -> Result<(), CapitalError> {
    match result {
        Ok(()) => transaction
            .commit()
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable),
        Err(error) => {
            transaction
                .rollback()
                .await
                .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
            Err(error)
        }
    }
}

fn validate_profile(max_entries: usize, ttl: Duration) -> Result<(), CapitalError> {
    if max_entries == 0 || max_entries > super::MAX_REPLAY_CAPACITY {
        return Err(CapitalError::ConfigurationError(format!(
            "Webhook replay capacity must be between 1 and {}",
            super::MAX_REPLAY_CAPACITY
        )));
    }
    if ttl.is_zero() || ttl > super::MAX_REPLAY_TTL {
        return Err(CapitalError::ConfigurationError(
            "Webhook replay TTL must be between 1 second and 30 days".to_string(),
        ));
    }
    Ok(())
}

fn validate_profile_row(
    row: &rullst_orm::sqlx::any::AnyRow,
    max_entries: usize,
    ttl: Duration,
) -> Result<(), CapitalError> {
    let stored_capacity = row
        .try_get::<i64, _>("max_entries")
        .map_err(|_| CapitalError::WebhookReplayCorruptState)?;
    let stored_ttl = row
        .try_get::<i64, _>("ttl_seconds")
        .map_err(|_| CapitalError::WebhookReplayCorruptState)?;
    if stored_capacity <= 0 || stored_ttl <= 0 {
        return Err(CapitalError::WebhookReplayCorruptState);
    }
    if stored_capacity != to_i64(max_entries)? || stored_ttl != to_i64(ttl.as_secs())? {
        return Err(CapitalError::WebhookReplayConfigurationDrift);
    }
    Ok(())
}

fn backend_from_url(database_url: &str) -> Result<SqlWebhookBackend, CapitalError> {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        Ok(SqlWebhookBackend::Postgres)
    } else if database_url.starts_with("mysql://") {
        Ok(SqlWebhookBackend::Mysql)
    } else if database_url.starts_with("sqlite:") {
        Ok(SqlWebhookBackend::Sqlite)
    } else {
        Err(CapitalError::ConfigurationError(
            "Webhook replay database URL must use postgres://, postgresql://, mysql://, or sqlite:"
                .to_string(),
        ))
    }
}

fn to_i64(value: impl TryInto<i64>) -> Result<i64, CapitalError> {
    value
        .try_into()
        .map_err(|_| CapitalError::WebhookReplayCorruptState)
}

#[cfg(test)]
#[path = "sql_tests.rs"]
mod tests;
