//! Durable shared SQLite revocation state for application-issued JWTs.

use super::{
    ApplicationJwtClaims, AsyncJwtRevocationStore, JwtError, JwtRevocationMode, unix_time,
    valid_identifier, valid_identity,
};
use sqlx::pool::PoolConnection;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Executor, Sqlite, SqliteConnection, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

const SCHEMA_VERSION: i64 = 1;
const MAX_REVOCATION_ENTRIES: usize = 1_000_000;

const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS rullst_auth_jwt_meta (id INTEGER PRIMARY KEY CHECK (id = 1), schema_version INTEGER NOT NULL CHECK (schema_version > 0), max_entries INTEGER NOT NULL CHECK (max_entries > 0))",
    "CREATE TABLE IF NOT EXISTS rullst_auth_jwt_tokens (jti TEXT PRIMARY KEY, expires_at INTEGER NOT NULL CHECK (expires_at > 0))",
    "CREATE TABLE IF NOT EXISTS rullst_auth_jwt_subjects (subject TEXT PRIMARY KEY, minimum_session_version INTEGER NOT NULL CHECK (minimum_session_version > 0))",
    "CREATE INDEX IF NOT EXISTS rullst_auth_jwt_token_expiry_idx ON rullst_auth_jwt_tokens(expires_at)",
];

/// Current bounded counts for one durable JWT revocation database.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SqliteJwtRevocationSnapshot {
    token_revocations: usize,
    subject_revocations: usize,
    max_entries: usize,
}

impl SqliteJwtRevocationSnapshot {
    #[must_use]
    pub const fn token_revocations(self) -> usize {
        self.token_revocations
    }

    #[must_use]
    pub const fn subject_revocations(self) -> usize {
        self.subject_revocations
    }

    #[must_use]
    pub const fn total_entries(self) -> usize {
        self.token_revocations + self.subject_revocations
    }

    #[must_use]
    pub const fn max_entries(self) -> usize {
        self.max_entries
    }
}

/// File-backed, bounded JWT revocation state shared by local processes.
///
/// Mutations use `BEGIN IMMEDIATE`, token expiry is pruned before capacity is
/// assessed, and the configured quota is persisted so another process cannot
/// silently open the same database with a different limit. The host owns file
/// permissions, backup, availability and multi-host replication.
#[derive(Clone)]
pub struct SqliteJwtRevocationStore {
    pool: SqlitePool,
    max_entries: usize,
}

impl SqliteJwtRevocationStore {
    /// Opens or creates a file-backed revocation database.
    pub async fn connect(
        database_url: impl Into<String>,
        max_entries: usize,
    ) -> Result<Self, JwtError> {
        if !(1..=MAX_REVOCATION_ENTRIES).contains(&max_entries) {
            return Err(JwtError::InvalidConfiguration(
                "SQLite revocation max_entries",
            ));
        }
        let database_url = database_url.into();
        if !database_url.starts_with("sqlite:") {
            return Err(JwtError::InvalidConfiguration(
                "SQLite revocation database URL",
            ));
        }
        let options = SqliteConnectOptions::from_str(&database_url)
            .map_err(|_| backend_error("parse SQLite revocation database URL"))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30));
        if volatile_database_url(&database_url, options.get_filename()) {
            return Err(JwtError::InvalidConfiguration(
                "SQLite revocation database must be file-backed",
            ));
        }
        reject_existing_unsafe_target(options.get_filename())?;
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await
            .map_err(|_| backend_error("connect SQLite revocation database"))?;
        if let Err(error) = prepare_schema(&pool, max_entries).await {
            pool.close().await;
            return Err(error);
        }
        Ok(Self { pool, max_entries })
    }

    /// Persists one token identifier until the token expires.
    pub async fn revoke_token(&self, claims: &ApplicationJwtClaims) -> Result<(), JwtError> {
        if !valid_identifier(&claims.jti, 64) {
            return Err(JwtError::InvalidConfiguration("jti"));
        }
        let now = unix_time()?;
        if claims.exp <= now {
            return Ok(());
        }
        let mut connection = self.begin_write("begin token revocation").await?;
        let result = self
            .revoke_token_in_transaction(&mut connection, &claims.jti, claims.exp, now)
            .await;
        finish(&mut connection, result, "finish token revocation").await
    }

    /// Rejects subject tokens below a monotonic session version.
    pub async fn revoke_subject_before(
        &self,
        subject: impl Into<String>,
        minimum_session_version: u64,
    ) -> Result<(), JwtError> {
        let subject = subject.into();
        if !valid_identity(&subject) || minimum_session_version == 0 {
            return Err(JwtError::InvalidConfiguration("subject revocation"));
        }
        let minimum_session_version = i64::try_from(minimum_session_version)
            .map_err(|_| JwtError::InvalidConfiguration("subject revocation"))?;
        let now = unix_time()?;
        let mut connection = self.begin_write("begin subject revocation").await?;
        let result = self
            .revoke_subject_in_transaction(&mut connection, &subject, minimum_session_version, now)
            .await;
        finish(&mut connection, result, "finish subject revocation").await
    }

    /// Returns active token and subject counts after pruning expired tokens.
    pub async fn snapshot(&self) -> Result<SqliteJwtRevocationSnapshot, JwtError> {
        let now = unix_time()?;
        let mut connection = self.begin_write("begin revocation snapshot").await?;
        let result = async {
            prune_expired(&mut connection, now).await?;
            let (token_revocations, subject_revocations) = counts(&mut connection).await?;
            Ok(SqliteJwtRevocationSnapshot {
                token_revocations,
                subject_revocations,
                max_entries: self.max_entries,
            })
        }
        .await;
        finish(&mut connection, result, "finish revocation snapshot").await
    }

    /// Gracefully closes all pooled connections, useful before rotating a file.
    pub async fn close(self) {
        self.pool.close().await;
    }

    async fn begin_write(
        &self,
        operation: &'static str,
    ) -> Result<PoolConnection<Sqlite>, JwtError> {
        let mut connection = self
            .pool
            .acquire()
            .await
            .map_err(|_| backend_error(operation))?;
        connection
            .execute("BEGIN IMMEDIATE")
            .await
            .map_err(|_| backend_error(operation))?;
        Ok(connection)
    }

    async fn revoke_token_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        jti: &str,
        expires_at: u64,
        now: u64,
    ) -> Result<(), JwtError> {
        prune_expired(connection, now).await?;
        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT expires_at FROM rullst_auth_jwt_tokens WHERE jti = ?")
                .bind(jti)
                .fetch_optional(&mut *connection)
                .await
                .map_err(|_| backend_error("lookup token revocation"))?;
        if existing.is_none() {
            ensure_capacity(connection, self.max_entries).await?;
        }
        let expires_at = i64::try_from(expires_at)
            .map_err(|_| JwtError::InvalidConfiguration("token expiry"))?;
        sqlx::query("INSERT INTO rullst_auth_jwt_tokens (jti, expires_at) VALUES (?, ?) ON CONFLICT(jti) DO UPDATE SET expires_at = MAX(expires_at, excluded.expires_at)")
            .bind(jti)
            .bind(expires_at)
            .execute(&mut *connection)
            .await
            .map_err(|_| backend_error("persist token revocation"))?;
        Ok(())
    }

    async fn revoke_subject_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        subject: &str,
        minimum_session_version: i64,
        now: u64,
    ) -> Result<(), JwtError> {
        prune_expired(connection, now).await?;
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT minimum_session_version FROM rullst_auth_jwt_subjects WHERE subject = ?",
        )
        .bind(subject)
        .fetch_optional(&mut *connection)
        .await
        .map_err(|_| backend_error("lookup subject revocation"))?;
        if existing.is_none() {
            ensure_capacity(connection, self.max_entries).await?;
        }
        sqlx::query("INSERT INTO rullst_auth_jwt_subjects (subject, minimum_session_version) VALUES (?, ?) ON CONFLICT(subject) DO UPDATE SET minimum_session_version = MAX(minimum_session_version, excluded.minimum_session_version)")
            .bind(subject)
            .bind(minimum_session_version)
            .execute(&mut *connection)
            .await
            .map_err(|_| backend_error("persist subject revocation"))?;
        Ok(())
    }
}

impl AsyncJwtRevocationStore for SqliteJwtRevocationStore {
    fn mode(&self) -> JwtRevocationMode {
        JwtRevocationMode::Shared
    }

    async fn is_revoked(&self, claims: &ApplicationJwtClaims, now: u64) -> Result<bool, JwtError> {
        let now = i64::try_from(now).map_err(|_| JwtError::InvalidSystemTime)?;
        let token: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM rullst_auth_jwt_tokens WHERE jti = ? AND expires_at > ?")
                .bind(&claims.jti)
                .bind(now)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| backend_error("read token revocation"))?;
        if token.is_some() {
            return Ok(true);
        }
        let subject: Option<(i64,)> = sqlx::query_as(
            "SELECT minimum_session_version FROM rullst_auth_jwt_subjects WHERE subject = ?",
        )
        .bind(&claims.sub)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| backend_error("read subject revocation"))?;
        match subject {
            Some((minimum,)) => {
                let minimum = u64::try_from(minimum)
                    .map_err(|_| backend_error("validate subject revocation"))?;
                Ok(claims.session_version < minimum)
            }
            None => Ok(false),
        }
    }
}

async fn prepare_schema(pool: &SqlitePool, max_entries: usize) -> Result<(), JwtError> {
    for statement in SCHEMA {
        pool.execute(*statement)
            .await
            .map_err(|_| backend_error("prepare SQLite revocation schema"))?;
    }
    let max_entries = i64::try_from(max_entries)
        .map_err(|_| JwtError::InvalidConfiguration("SQLite revocation max_entries"))?;
    sqlx::query("INSERT OR IGNORE INTO rullst_auth_jwt_meta (id, schema_version, max_entries) VALUES (1, ?, ?)")
        .bind(SCHEMA_VERSION)
        .bind(max_entries)
        .execute(pool)
        .await
        .map_err(|_| backend_error("register SQLite revocation configuration"))?;
    let stored: (i64, i64) =
        sqlx::query_as("SELECT schema_version, max_entries FROM rullst_auth_jwt_meta WHERE id = 1")
            .fetch_one(pool)
            .await
            .map_err(|_| backend_error("read SQLite revocation configuration"))?;
    if stored.0 != SCHEMA_VERSION || stored.1 <= 0 {
        return Err(backend_error("validate SQLite revocation schema"));
    }
    if stored.1 != max_entries {
        return Err(JwtError::InvalidConfiguration(
            "SQLite revocation max_entries conflicts with stored configuration",
        ));
    }
    Ok(())
}

async fn prune_expired(connection: &mut SqliteConnection, now: u64) -> Result<(), JwtError> {
    let now = i64::try_from(now).map_err(|_| JwtError::InvalidSystemTime)?;
    sqlx::query("DELETE FROM rullst_auth_jwt_tokens WHERE expires_at <= ?")
        .bind(now)
        .execute(&mut *connection)
        .await
        .map_err(|_| backend_error("prune expired token revocations"))?;
    Ok(())
}

async fn ensure_capacity(
    connection: &mut SqliteConnection,
    max_entries: usize,
) -> Result<(), JwtError> {
    let (tokens, subjects) = counts(connection).await?;
    let total = tokens
        .checked_add(subjects)
        .ok_or(JwtError::RevocationStoreCapacity)?;
    if total >= max_entries {
        return Err(JwtError::RevocationStoreCapacity);
    }
    Ok(())
}

async fn counts(connection: &mut SqliteConnection) -> Result<(usize, usize), JwtError> {
    let row: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM rullst_auth_jwt_tokens), (SELECT COUNT(*) FROM rullst_auth_jwt_subjects)",
    )
    .fetch_one(&mut *connection)
    .await
    .map_err(|_| backend_error("count SQLite revocations"))?;
    let tokens = usize::try_from(row.0).map_err(|_| backend_error("validate token count"))?;
    let subjects = usize::try_from(row.1).map_err(|_| backend_error("validate subject count"))?;
    Ok((tokens, subjects))
}

async fn finish<T>(
    connection: &mut PoolConnection<Sqlite>,
    result: Result<T, JwtError>,
    operation: &'static str,
) -> Result<T, JwtError> {
    let statement = if result.is_ok() { "COMMIT" } else { "ROLLBACK" };
    if connection.execute(statement).await.is_err() {
        connection.close_on_drop();
        return Err(backend_error(operation));
    }
    result
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

fn reject_existing_unsafe_target(path: &Path) -> Result<(), JwtError> {
    #[cfg(windows)]
    let portable_path = path.as_os_str().to_string_lossy();
    #[cfg(windows)]
    let path = windows_file_url_target(&portable_path)
        .map(Path::new)
        .unwrap_or(path);

    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(
            JwtError::InvalidConfiguration("SQLite revocation target must be a regular file"),
        ),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(backend_error("inspect SQLite revocation target")),
    }
}

#[cfg(any(windows, test))]
fn windows_file_url_target(path: &str) -> Option<&str> {
    let bytes = path.as_bytes();
    (bytes.len() >= 3
        && matches!(bytes[0], b'/' | b'\\')
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b':')
        .then(|| &path[1..])
}

fn backend_error(operation: &'static str) -> JwtError {
    JwtError::RevocationBackend(operation.to_string())
}

#[cfg(test)]
mod tests {
    use super::windows_file_url_target;

    #[test]
    fn windows_file_url_target_removes_only_a_leading_drive_separator() {
        assert_eq!(
            windows_file_url_target("/C:/temp/auth.sqlite"),
            Some("C:/temp/auth.sqlite")
        );
        assert_eq!(
            windows_file_url_target("\\D:/temp/auth.sqlite"),
            Some("D:/temp/auth.sqlite")
        );
        assert_eq!(windows_file_url_target("C:/temp/auth.sqlite"), None);
        assert_eq!(windows_file_url_target("/tmp/auth.sqlite"), None);
        assert_eq!(windows_file_url_target("//server/share/auth.sqlite"), None);
    }
}
