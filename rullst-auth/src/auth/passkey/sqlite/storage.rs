use super::PasskeyStoreError;
use sqlx::pool::PoolConnection;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Executor, Sqlite, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 1;
const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS rullst_auth_passkey_meta (id INTEGER PRIMARY KEY CHECK (id = 1), schema_version INTEGER NOT NULL CHECK (schema_version > 0), max_total_credentials INTEGER NOT NULL CHECK (max_total_credentials > 0), max_credentials_per_subject INTEGER NOT NULL CHECK (max_credentials_per_subject > 0))",
    "CREATE TABLE IF NOT EXISTS rullst_auth_passkey_devices (credential_id BLOB PRIMARY KEY, subject TEXT NOT NULL, label TEXT NOT NULL, public_key BLOB NOT NULL, sign_count INTEGER NOT NULL CHECK (sign_count >= 0), created_at INTEGER NOT NULL CHECK (created_at > 0), last_used_at INTEGER, revoked_at INTEGER, CHECK (last_used_at IS NULL OR last_used_at >= created_at), CHECK (revoked_at IS NULL OR revoked_at >= created_at))",
    "CREATE INDEX IF NOT EXISTS rullst_auth_passkey_subject_idx ON rullst_auth_passkey_devices(subject, created_at, credential_id)",
];

pub(super) async fn connect_pool(database_url: &str) -> Result<SqlitePool, PasskeyStoreError> {
    if !database_url.starts_with("sqlite:") {
        return Err(PasskeyStoreError::InvalidConfiguration("database URL"));
    }
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|_| unavailable("parse database URL"))?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(30));
    if volatile_database_url(database_url, options.get_filename()) {
        return Err(PasskeyStoreError::InvalidConfiguration(
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
    max_total: usize,
    max_per_subject: usize,
) -> Result<(), PasskeyStoreError> {
    for statement in SCHEMA {
        pool.execute(*statement)
            .await
            .map_err(|_| unavailable("prepare schema"))?;
    }
    let total = as_i64(max_total, "total limit")?;
    let per_subject = as_i64(max_per_subject, "per-subject limit")?;
    sqlx::query("INSERT OR IGNORE INTO rullst_auth_passkey_meta (id, schema_version, max_total_credentials, max_credentials_per_subject) VALUES (1, ?, ?, ?)")
        .bind(SCHEMA_VERSION)
        .bind(total)
        .bind(per_subject)
        .execute(pool)
        .await
        .map_err(|_| unavailable("register configuration"))?;
    let stored: (i64, i64, i64) = sqlx::query_as("SELECT schema_version, max_total_credentials, max_credentials_per_subject FROM rullst_auth_passkey_meta WHERE id = 1")
        .fetch_one(pool)
        .await
        .map_err(|_| unavailable("read configuration"))?;
    if stored.0 != SCHEMA_VERSION || stored.1 <= 0 || stored.2 <= 0 {
        return Err(corrupt("schema configuration"));
    }
    if stored.1 != total || stored.2 != per_subject {
        return Err(PasskeyStoreError::InvalidConfiguration(
            "credential limits conflict with stored configuration",
        ));
    }
    Ok(())
}

pub(super) async fn finish<T>(
    connection: &mut PoolConnection<Sqlite>,
    result: Result<T, PasskeyStoreError>,
    operation: &'static str,
) -> Result<T, PasskeyStoreError> {
    let statement = if result.is_ok() { "COMMIT" } else { "ROLLBACK" };
    if connection.execute(statement).await.is_err() {
        connection.close_on_drop();
        return Err(unavailable(operation));
    }
    result
}

pub(super) fn unix_time() -> Result<u64, PasskeyStoreError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| corrupt("system time"))
        .map(|duration| duration.as_secs())
}

pub(super) fn as_i64(value: usize, context: &'static str) -> Result<i64, PasskeyStoreError> {
    i64::try_from(value).map_err(|_| corrupt(context))
}

pub(super) const fn unavailable(operation: &'static str) -> PasskeyStoreError {
    PasskeyStoreError::StorageUnavailable(operation)
}

pub(super) const fn corrupt(context: &'static str) -> PasskeyStoreError {
    PasskeyStoreError::CorruptStorage(context)
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

fn reject_existing_unsafe_target(path: &Path) -> Result<(), PasskeyStoreError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(
            PasskeyStoreError::InvalidConfiguration("target must be a regular file"),
        ),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(unavailable("inspect database target")),
    }
}
