//! Schema preparation and database-target checks for the SQLite JWT store.

use super::backend_error;
use crate::jwt::JwtError;
use sqlx::{Executor, SqlitePool};
use std::path::Path;

/// Unchanged by additive columns, so older releases keep opening the file.
const SCHEMA_VERSION: i64 = 1;
pub(super) const MAX_REVOCATION_ENTRIES: usize = 1_000_000;

const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS rullst_auth_jwt_meta (id INTEGER PRIMARY KEY CHECK (id = 1), schema_version INTEGER NOT NULL CHECK (schema_version > 0), max_entries INTEGER NOT NULL CHECK (max_entries > 0))",
    "CREATE TABLE IF NOT EXISTS rullst_auth_jwt_tokens (jti TEXT PRIMARY KEY, expires_at INTEGER NOT NULL CHECK (expires_at > 0))",
    "CREATE TABLE IF NOT EXISTS rullst_auth_jwt_subjects (subject TEXT PRIMARY KEY, minimum_session_version INTEGER NOT NULL CHECK (minimum_session_version > 0))",
    "CREATE INDEX IF NOT EXISTS rullst_auth_jwt_token_expiry_idx ON rullst_auth_jwt_tokens(expires_at)",
];

/// Additive columns and index for per-subject quotas and subject cutoffs.
/// Older releases write with explicit column lists, so these stay compatible.
const ADDITIVE_COLUMNS: &[(&str, &str, &str)] = &[
    (
        "rullst_auth_jwt_tokens",
        "subject",
        "ALTER TABLE rullst_auth_jwt_tokens ADD COLUMN subject TEXT",
    ),
    (
        "rullst_auth_jwt_subjects",
        "revoked_through_iat",
        "ALTER TABLE rullst_auth_jwt_subjects ADD COLUMN revoked_through_iat INTEGER NOT NULL DEFAULT 0 CHECK (revoked_through_iat >= 0)",
    ),
];

pub(super) async fn prepare_schema(pool: &SqlitePool, max_entries: usize) -> Result<(), JwtError> {
    for statement in SCHEMA {
        pool.execute(*statement)
            .await
            .map_err(|_| backend_error("prepare SQLite revocation schema"))?;
    }
    add_columns(pool).await?;
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

/// Adds missing columns under one write lock so concurrent openers agree.
async fn add_columns(pool: &SqlitePool) -> Result<(), JwtError> {
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| backend_error("begin SQLite revocation migration"))?;
    for (table, column, statement) in ADDITIVE_COLUMNS {
        let (present,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM pragma_table_info(?) WHERE name = ?")
                .bind(*table)
                .bind(*column)
                .fetch_one(&mut *transaction)
                .await
                .map_err(|_| backend_error("inspect SQLite revocation schema"))?;
        if present == 0 {
            transaction
                .execute(*statement)
                .await
                .map_err(|_| backend_error("migrate SQLite revocation schema"))?;
        }
    }
    transaction
        .execute("CREATE INDEX IF NOT EXISTS rullst_auth_jwt_token_subject_idx ON rullst_auth_jwt_tokens(subject)")
        .await
        .map_err(|_| backend_error("migrate SQLite revocation schema"))?;
    transaction
        .commit()
        .await
        .map_err(|_| backend_error("commit SQLite revocation migration"))
}

pub(super) fn volatile_database_url(database_url: &str, filename: &Path) -> bool {
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

pub(super) fn reject_existing_unsafe_target(path: &Path) -> Result<(), JwtError> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
