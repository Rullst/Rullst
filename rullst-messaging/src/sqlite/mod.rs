//! Durable SQLite broker with transactional at-least-once delivery state.

mod admin;
mod codec;
mod consume;
mod publish;
mod schema;
mod storage;
mod transaction;

use crate::{
    AckToken, BrokerConfig, Clock, DeadLetter, DeadLetterQuery, Delivery, FailureCode,
    MessageAdmin, MessageBroker, PublishReceipt, PublishRequest, PurgeReceipt, PurgeRequest,
    ReceiveRequest, Result, RetryDisposition, SubscriptionReceipt, SubscriptionRequest,
    SystemClock,
};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use storage::StorageProfile;
pub use storage::{MessagingKeyring, MessagingStorageKey};

/// Durable local broker backed by a fixed, versioned SQLite schema.
///
/// Every mutation uses `BEGIN IMMEDIATE`, so multiple processes sharing the
/// same file serialize claims, acknowledgements and idempotent publication.
/// Delivery remains at least once and destination-side effects must still be
/// idempotent.
#[derive(Clone)]
pub struct SqliteBroker<C = SystemClock> {
    pub(super) config: BrokerConfig,
    pub(super) clock: C,
    pub(super) pool: SqlitePool,
    storage: StorageProfile,
}

impl SqliteBroker<SystemClock> {
    /// Opens or creates a durable broker using the system clock.
    pub async fn connect(database_url: impl Into<String>, config: BrokerConfig) -> Result<Self> {
        Self::connect_with_clock(database_url, config, SystemClock).await
    }

    /// Opens encrypted local storage using the system clock and supplied keyring.
    pub async fn connect_encrypted(
        database_url: impl Into<String>,
        config: BrokerConfig,
        keyring: MessagingKeyring,
    ) -> Result<Self> {
        Self::connect_encrypted_with_clock(database_url, config, keyring, SystemClock).await
    }
}

impl<C: Clock> SqliteBroker<C> {
    /// Opens or creates a durable broker with an injectable trusted clock.
    pub async fn connect_with_clock(
        database_url: impl Into<String>,
        config: BrokerConfig,
        clock: C,
    ) -> Result<Self> {
        Self::connect_with_profile(database_url, config, clock, StorageProfile::plaintext()).await
    }

    /// Opens encrypted local storage with an injectable trusted clock.
    pub async fn connect_encrypted_with_clock(
        database_url: impl Into<String>,
        config: BrokerConfig,
        keyring: MessagingKeyring,
        clock: C,
    ) -> Result<Self> {
        Self::connect_with_profile(
            database_url,
            config,
            clock,
            StorageProfile::encrypted(keyring),
        )
        .await
    }

    async fn connect_with_profile(
        database_url: impl Into<String>,
        config: BrokerConfig,
        clock: C,
        storage: StorageProfile,
    ) -> Result<Self> {
        let database_url = database_url.into();
        if !database_url.starts_with("sqlite:") {
            return Err(invalid_database_url(
                "must use the sqlite: URL scheme and identify a file",
            ));
        }
        let options = SqliteConnectOptions::from_str(&database_url)
            .map_err(|_| invalid_database_url("must be a supported SQLite file URL"))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30));
        if is_volatile_database_url(&database_url, options.get_filename()) {
            return Err(invalid_database_url("must identify a file-backed database"));
        }
        reject_existing_unsafe_target(options.get_filename())?;
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await
            .map_err(|_| transaction::storage_error("connect"))?;
        schema::prepare(&pool, &config, &storage).await?;
        Ok(Self {
            config,
            clock,
            pool,
            storage,
        })
    }

    /// Returns the persisted broker configuration.
    pub fn config(&self) -> &BrokerConfig {
        &self.config
    }
}

fn reject_existing_unsafe_target(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(
            invalid_database_url("existing target must be a regular file"),
        ),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(transaction::storage_error("inspect database target")),
    }
}

fn is_volatile_database_url(database_url: &str, filename: &Path) -> bool {
    let filename = filename.as_os_str().to_string_lossy();
    let memory_mode = database_url
        .split_once('?')
        .map(|(_, query)| {
            url::form_urlencoded::parse(query.as_bytes()).any(|(key, value)| {
                key.eq_ignore_ascii_case("mode") && value.eq_ignore_ascii_case("memory")
            })
        })
        .unwrap_or(false);
    database_url.eq_ignore_ascii_case("sqlite::memory:")
        || database_url.eq_ignore_ascii_case("sqlite://:memory:")
        || filename.is_empty()
        || filename.eq_ignore_ascii_case(":memory:")
        || filename.eq_ignore_ascii_case("file::memory:")
        || memory_mode
}

fn invalid_database_url(reason: &'static str) -> crate::MessagingError {
    crate::MessagingError::Invalid {
        field: "durable SQLite database URL",
        reason,
    }
}

impl<C: Clock> MessageBroker for SqliteBroker<C> {
    async fn publish(&self, request: PublishRequest) -> Result<PublishReceipt> {
        self.publish_inner(request).await
    }

    async fn subscribe(&self, request: SubscriptionRequest) -> Result<SubscriptionReceipt> {
        self.subscribe_inner(request).await
    }

    async fn receive(&self, request: ReceiveRequest) -> Result<Vec<Delivery>> {
        self.receive_inner(request).await
    }

    async fn ack(&self, token: &AckToken) -> Result<()> {
        self.ack_inner(token).await
    }

    async fn retry(
        &self,
        token: &AckToken,
        delay: Duration,
        failure_code: FailureCode,
    ) -> Result<RetryDisposition> {
        self.retry_inner(token, delay, failure_code).await
    }

    async fn dead_letter(&self, token: &AckToken, failure_code: FailureCode) -> Result<()> {
        self.dead_letter_inner(token, failure_code).await
    }
}

impl<C: Clock> MessageAdmin for SqliteBroker<C> {
    async fn dead_letters(&self, query: DeadLetterQuery) -> Result<Vec<DeadLetter>> {
        self.dead_letters_inner(query).await
    }

    async fn purge_terminal(&self, request: PurgeRequest) -> Result<PurgeReceipt> {
        self.purge_terminal_inner(request).await
    }
}
