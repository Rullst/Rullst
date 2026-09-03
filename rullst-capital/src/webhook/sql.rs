//! Durable relational webhook replay claims.

use super::{event_key_hash, payload_key, validate_replay_event_key, validate_replay_provider};
use crate::CapitalError;
use rullst_orm::sqlx::{Any, AnyPool, Row, Transaction, any::AnyPoolOptions};
use std::time::Duration;

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

        let inserted = rullst_orm::sqlx::query(insert_claim_sql(self.backend))
            .bind(provider)
            .bind(key)
            .bind(accepted_at)
            .bind(expires_at)
            .execute(&mut **transaction)
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
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

fn timestamp_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "SELECT CAST(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP) AS BIGINT)"
        }
        SqlWebhookBackend::Mysql => "SELECT UNIX_TIMESTAMP()",
        SqlWebhookBackend::Sqlite => "SELECT CAST(strftime('%s', 'now') AS INTEGER)",
    }
}

fn config_schema_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "CREATE TABLE IF NOT EXISTS rullst_webhook_replay_config (singleton SMALLINT PRIMARY KEY CHECK (singleton = 1), max_entries BIGINT NOT NULL, ttl_seconds BIGINT NOT NULL)"
        }
        SqlWebhookBackend::Mysql => {
            "CREATE TABLE IF NOT EXISTS rullst_webhook_replay_config (singleton SMALLINT PRIMARY KEY, max_entries BIGINT NOT NULL, ttl_seconds BIGINT NOT NULL)"
        }
        SqlWebhookBackend::Sqlite => {
            "CREATE TABLE IF NOT EXISTS rullst_webhook_replay_config (singleton INTEGER PRIMARY KEY CHECK (singleton = 1), max_entries INTEGER NOT NULL CHECK (max_entries > 0), ttl_seconds INTEGER NOT NULL CHECK (ttl_seconds > 0))"
        }
    }
}

fn claim_schema_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "CREATE TABLE IF NOT EXISTS rullst_webhook_replay_claims (provider VARCHAR(64) NOT NULL, replay_hash VARCHAR(64) NOT NULL, accepted_at BIGINT NOT NULL, expires_at BIGINT NOT NULL, PRIMARY KEY (provider, replay_hash))"
        }
        SqlWebhookBackend::Mysql => {
            "CREATE TABLE IF NOT EXISTS rullst_webhook_replay_claims (provider VARCHAR(64) NOT NULL, replay_hash VARCHAR(64) NOT NULL, accepted_at BIGINT NOT NULL, expires_at BIGINT NOT NULL, PRIMARY KEY (provider, replay_hash), INDEX rullst_webhook_replay_expiry (expires_at))"
        }
        SqlWebhookBackend::Sqlite => {
            "CREATE TABLE IF NOT EXISTS rullst_webhook_replay_claims (provider TEXT NOT NULL, replay_hash TEXT NOT NULL, accepted_at INTEGER NOT NULL, expires_at INTEGER NOT NULL, PRIMARY KEY (provider, replay_hash), CHECK (length(provider) BETWEEN 1 AND 64), CHECK (length(replay_hash) = 64), CHECK (accepted_at >= 0), CHECK (expires_at > accepted_at))"
        }
    }
}

fn claim_expiry_index_sql(backend: SqlWebhookBackend) -> Option<&'static str> {
    match backend {
        SqlWebhookBackend::Postgres | SqlWebhookBackend::Sqlite => Some(
            "CREATE INDEX IF NOT EXISTS rullst_webhook_replay_expiry ON rullst_webhook_replay_claims (expires_at)",
        ),
        SqlWebhookBackend::Mysql => None,
    }
}

fn insert_config_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres | SqlWebhookBackend::Sqlite => {
            if backend == SqlWebhookBackend::Postgres {
                "INSERT INTO rullst_webhook_replay_config (singleton, max_entries, ttl_seconds) VALUES (1, $1, $2) ON CONFLICT (singleton) DO NOTHING"
            } else {
                "INSERT INTO rullst_webhook_replay_config (singleton, max_entries, ttl_seconds) VALUES (1, ?, ?) ON CONFLICT (singleton) DO NOTHING"
            }
        }
        SqlWebhookBackend::Mysql => {
            "INSERT INTO rullst_webhook_replay_config (singleton, max_entries, ttl_seconds) VALUES (1, ?, ?) ON DUPLICATE KEY UPDATE singleton = VALUES(singleton)"
        }
    }
}

fn insert_claim_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "INSERT INTO rullst_webhook_replay_claims (provider, replay_hash, accepted_at, expires_at) VALUES ($1, $2, $3, $4) ON CONFLICT (provider, replay_hash) DO NOTHING"
        }
        SqlWebhookBackend::Sqlite => {
            "INSERT INTO rullst_webhook_replay_claims (provider, replay_hash, accepted_at, expires_at) VALUES (?, ?, ?, ?) ON CONFLICT (provider, replay_hash) DO NOTHING"
        }
        SqlWebhookBackend::Mysql => {
            "INSERT INTO rullst_webhook_replay_claims (provider, replay_hash, accepted_at, expires_at) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE replay_hash = VALUES(replay_hash)"
        }
    }
}

fn select_config_sql() -> &'static str {
    "SELECT max_entries, ttl_seconds FROM rullst_webhook_replay_config WHERE singleton = 1"
}

fn select_config_for_update_sql() -> &'static str {
    "SELECT max_entries, ttl_seconds FROM rullst_webhook_replay_config WHERE singleton = 1 FOR UPDATE"
}

fn lock_sqlite_config_sql() -> &'static str {
    "UPDATE rullst_webhook_replay_config SET singleton = singleton WHERE singleton = 1"
}

fn delete_expired_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "DELETE FROM rullst_webhook_replay_claims WHERE expires_at <= $1"
        }
        _ => "DELETE FROM rullst_webhook_replay_claims WHERE expires_at <= ?",
    }
}

fn contains_claim_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "SELECT expires_at FROM rullst_webhook_replay_claims WHERE provider = $1 AND replay_hash = $2"
        }
        _ => {
            "SELECT expires_at FROM rullst_webhook_replay_claims WHERE provider = ? AND replay_hash = ?"
        }
    }
}

fn active_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM rullst_webhook_replay_claims"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn sqlite_url(label: &str) -> (String, std::path::PathBuf) {
        let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rullst-capital-webhook-{label}-{}-{sequence}.sqlite",
            std::process::id()
        ));
        (format!("sqlite://{}?mode=rwc", path.display()), path)
    }

    async fn record_at(
        store: &SqlWebhookReplayStore,
        provider: &str,
        payload: &[u8],
        accepted_at: u64,
    ) -> Result<(), CapitalError> {
        let mut transaction = store
            .pool
            .begin()
            .await
            .map_err(|_| CapitalError::WebhookReplayStoreUnavailable)?;
        let result = store
            .check_and_record_at(&mut transaction, provider, payload, accepted_at)
            .await;
        finish_transaction(transaction, result).await
    }

    #[test]
    fn validates_profiles_and_backend_urls() {
        assert!(validate_profile(1, Duration::from_secs(1)).is_ok());
        assert!(validate_profile(0, Duration::from_secs(1)).is_err());
        assert!(
            validate_profile(
                super::super::MAX_REPLAY_CAPACITY + 1,
                Duration::from_secs(1)
            )
            .is_err()
        );
        assert!(validate_profile(1, Duration::ZERO).is_err());
        assert!(
            validate_profile(1, super::super::MAX_REPLAY_TTL + Duration::from_secs(1)).is_err()
        );
        assert_eq!(
            backend_from_url("postgres://localhost/db"),
            Ok(SqlWebhookBackend::Postgres)
        );
        assert_eq!(
            backend_from_url("mysql://localhost/db"),
            Ok(SqlWebhookBackend::Mysql)
        );
        assert_eq!(
            backend_from_url("sqlite::memory:"),
            Ok(SqlWebhookBackend::Sqlite)
        );
        assert!(backend_from_url("mongodb://localhost/db").is_err());
        assert_eq!(
            timestamp_sql(SqlWebhookBackend::Postgres),
            "SELECT CAST(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP) AS BIGINT)"
        );
        assert_eq!(
            timestamp_sql(SqlWebhookBackend::Mysql),
            "SELECT UNIX_TIMESTAMP()"
        );
        assert_eq!(
            timestamp_sql(SqlWebhookBackend::Sqlite),
            "SELECT CAST(strftime('%s', 'now') AS INTEGER)"
        );
    }

    #[tokio::test]
    async fn sqlite_claims_survive_restart_and_fail_closed_at_capacity() {
        let (url, path) = sqlite_url("restart");
        let first = SqlWebhookReplayStore::connect(&url, 2, Duration::from_secs(60))
            .await
            .expect("first SQLite replay store");
        first.prepare_schema().await.expect("replay schema");
        first
            .check_and_record_payload("stripe", b"event-one")
            .await
            .expect("first claim");
        first.close().await;

        let reopened = SqlWebhookReplayStore::connect(&url, 2, Duration::from_secs(60))
            .await
            .expect("reopened SQLite replay store");
        reopened.prepare_schema().await.expect("existing schema");
        assert!(matches!(
            reopened
                .check_and_record_payload("stripe", b"event-one")
                .await,
            Err(CapitalError::WebhookReplay(_))
        ));
        reopened
            .check_and_record_payload("stripe", b"event-two")
            .await
            .expect("second claim");
        assert_eq!(
            reopened
                .check_and_record_payload("stripe", b"event-three")
                .await,
            Err(CapitalError::WebhookReplayStoreFull)
        );
        reopened.close().await;
        std::fs::remove_file(path).expect("remove closed SQLite fixture");
    }

    #[tokio::test]
    async fn sqlite_expiry_and_configuration_drift_are_explicit() {
        let store = SqlWebhookReplayStore::connect("sqlite::memory:", 1, Duration::from_secs(10))
            .await
            .expect("SQLite replay store");
        store.prepare_schema().await.expect("replay schema");
        record_at(&store, "stripe", b"same-event", 100)
            .await
            .expect("initial deterministic claim");
        assert!(matches!(
            record_at(&store, "stripe", b"same-event", 109).await,
            Err(CapitalError::WebhookReplay(_))
        ));
        record_at(&store, "stripe", b"same-event", 110)
            .await
            .expect("claim after exact TTL boundary");

        let drifted = SqlWebhookReplayStore::from_pool(
            store.pool.clone(),
            SqlWebhookBackend::Sqlite,
            2,
            Duration::from_secs(10),
        )
        .expect("valid alternate local profile");
        assert_eq!(
            drifted.prepare_schema().await,
            Err(CapitalError::WebhookReplayConfigurationDrift)
        );
        store.close().await;
    }

    #[tokio::test]
    async fn sqlite_concurrent_duplicate_has_one_winner() {
        let (url, path) = sqlite_url("concurrency");
        let store = std::sync::Arc::new(
            SqlWebhookReplayStore::connect(&url, 32, Duration::from_secs(60))
                .await
                .expect("SQLite replay store"),
        );
        store.prepare_schema().await.expect("replay schema");

        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..8 {
            let store = store.clone();
            tasks.spawn(async move {
                store
                    .check_and_record_payload("stripe", b"concurrent-event")
                    .await
            });
        }
        let mut accepted = 0;
        let mut replayed = 0;
        while let Some(result) = tasks.join_next().await {
            match result.expect("replay task") {
                Ok(()) => accepted += 1,
                Err(CapitalError::WebhookReplay(_)) => replayed += 1,
                Err(error) => panic!("unexpected replay-store error: {error}"),
            }
        }
        assert_eq!(accepted, 1);
        assert_eq!(replayed, 7);

        store.close().await;
        std::fs::remove_file(path).expect("remove closed SQLite fixture");
    }

    #[tokio::test]
    async fn caller_transaction_commits_or_rolls_back_claim_with_domain_effect() {
        let store = SqlWebhookReplayStore::connect("sqlite::memory:", 8, Duration::from_secs(60))
            .await
            .expect("SQLite replay store");
        store.prepare_schema().await.expect("replay schema");
        rullst_orm::sqlx::query(
            "CREATE TABLE webhook_effects (event_name TEXT PRIMARY KEY NOT NULL)",
        )
        .execute(store.pool())
        .await
        .expect("domain fixture schema");

        let mut rolled_back = store.pool().begin().await.expect("rollback transaction");
        store
            .check_and_record_event_key_with_transaction(
                &mut rolled_back,
                "stripe",
                "evt_transaction_1",
            )
            .await
            .expect("transactional claim");
        rullst_orm::sqlx::query("INSERT INTO webhook_effects (event_name) VALUES (?)")
            .bind("rolled-back")
            .execute(&mut *rolled_back)
            .await
            .expect("transactional domain effect");
        rolled_back.rollback().await.expect("explicit rollback");

        let mut committed = store.pool().begin().await.expect("commit transaction");
        store
            .check_and_record_event_key_with_transaction(
                &mut committed,
                "stripe",
                "evt_transaction_1",
            )
            .await
            .expect("claim was rolled back with effect");
        rullst_orm::sqlx::query("INSERT INTO webhook_effects (event_name) VALUES (?)")
            .bind("committed")
            .execute(&mut *committed)
            .await
            .expect("committed domain effect");
        committed.commit().await.expect("atomic commit");

        assert!(matches!(
            store
                .check_and_record_event_key("stripe", "evt_transaction_1")
                .await,
            Err(CapitalError::WebhookReplay(_))
        ));
        let effects = rullst_orm::sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM webhook_effects WHERE event_name = ?",
        )
        .bind("committed")
        .fetch_one(store.pool())
        .await
        .expect("domain effect count");
        assert_eq!(effects, 1);
        store.close().await;
    }
}
