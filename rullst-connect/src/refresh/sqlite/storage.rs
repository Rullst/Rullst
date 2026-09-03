use super::super::TokenSnapshotBinding;
use super::TokenStoreError;
use sha2::{Digest as _, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Executor, Sqlite, SqlitePool, Transaction};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

const SCHEMA_VERSION: i64 = 1;
const SUBJECT_KEY_DOMAIN: &[u8] = b"rullst-connect:token-store-subject:v1";
const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS rullst_connect_token_meta (id INTEGER PRIMARY KEY CHECK (id = 1), schema_version INTEGER NOT NULL CHECK (schema_version > 0), max_entries INTEGER NOT NULL CHECK (max_entries > 0))",
    "CREATE TABLE IF NOT EXISTS rullst_connect_token_snapshots (subject_key TEXT PRIMARY KEY CHECK (length(subject_key) = 64), generation INTEGER NOT NULL CHECK (generation >= 0), key_id TEXT NOT NULL CHECK (length(key_id) BETWEEN 1 AND 128), envelope TEXT NOT NULL CHECK (length(envelope) BETWEEN 1 AND 196608))",
];

pub(super) async fn connect_pool(database_url: &str) -> Result<SqlitePool, TokenStoreError> {
    if !database_url.starts_with("sqlite:") {
        return Err(TokenStoreError::InvalidConfiguration("database URL"));
    }
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|_| unavailable("parse database URL"))?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(30));
    if volatile_database_url(database_url, options.get_filename()) {
        return Err(TokenStoreError::InvalidConfiguration(
            "database must be file-backed",
        ));
    }
    reject_existing_unsafe_target(options.get_filename())?;
    SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .map_err(|_| unavailable("connect database"))
}

pub(super) async fn prepare_schema(
    pool: &SqlitePool,
    max_entries: usize,
) -> Result<(), TokenStoreError> {
    let mut connection = begin_write(pool, "begin schema preparation").await?;
    let result = async {
        for statement in SCHEMA {
            connection
                .execute(*statement)
                .await
                .map_err(|_| unavailable("prepare schema"))?;
        }
        let max_entries = usize_as_i64(max_entries, "entry limit")?;
        sqlx::query("INSERT OR IGNORE INTO rullst_connect_token_meta (id, schema_version, max_entries) VALUES (1, ?, ?)")
            .bind(SCHEMA_VERSION)
            .bind(max_entries)
            .execute(&mut *connection)
            .await
            .map_err(|_| unavailable("register configuration"))?;
        let stored: (i64, i64) = sqlx::query_as(
            "SELECT schema_version, max_entries FROM rullst_connect_token_meta WHERE id = 1",
        )
        .fetch_one(&mut *connection)
        .await
        .map_err(|_| unavailable("read configuration"))?;
        if stored.0 != SCHEMA_VERSION || stored.1 <= 0 {
            return Err(corrupt("schema configuration"));
        }
        if stored.1 != max_entries {
            return Err(TokenStoreError::InvalidConfiguration(
                "entry limit conflicts with stored configuration",
            ));
        }
        Ok(())
    }
    .await;
    finish(connection, result, "finish schema preparation").await
}

pub(super) async fn begin_write(
    pool: &SqlitePool,
    operation: &'static str,
) -> Result<Transaction<'static, Sqlite>, TokenStoreError> {
    pool.begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| unavailable(operation))
}

pub(super) async fn finish<T>(
    transaction: Transaction<'static, Sqlite>,
    result: Result<T, TokenStoreError>,
    operation: &'static str,
) -> Result<T, TokenStoreError> {
    match result {
        Ok(value) => {
            transaction
                .commit()
                .await
                .map_err(|_| unavailable(operation))?;
            Ok(value)
        }
        Err(error) => {
            transaction
                .rollback()
                .await
                .map_err(|_| unavailable(operation))?;
            Err(error)
        }
    }
}

pub(super) fn subject_key(binding: &TokenSnapshotBinding) -> String {
    let mut digest = Sha256::new();
    append_field(&mut digest, SUBJECT_KEY_DOMAIN);
    append_field(&mut digest, binding.provider().as_bytes());
    append_field(&mut digest, binding.account_id().as_bytes());
    hex::encode(digest.finalize())
}

fn append_field(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}

pub(super) fn as_i64(value: u64, context: &'static str) -> Result<i64, TokenStoreError> {
    i64::try_from(value).map_err(|_| corrupt(context))
}

pub(super) fn usize_as_i64(value: usize, context: &'static str) -> Result<i64, TokenStoreError> {
    i64::try_from(value).map_err(|_| corrupt(context))
}

pub(super) const fn unavailable(operation: &'static str) -> TokenStoreError {
    TokenStoreError::StorageUnavailable(operation)
}

pub(super) const fn corrupt(context: &'static str) -> TokenStoreError {
    TokenStoreError::CorruptStorage(context)
}

fn volatile_database_url(database_url: &str, filename: &Path) -> bool {
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

fn reject_existing_unsafe_target(path: &Path) -> Result<(), TokenStoreError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(
            TokenStoreError::InvalidConfiguration("target must be a regular file"),
        ),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(unavailable("inspect database target")),
    }
}
