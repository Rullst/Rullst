//! The database profile shown in the inspector: which database family the
//! project configures, never any part of the connection URL.

use std::{fs::File, io::Read};

const ENV_READ_LIMIT: u64 = 64 * 1_024;

pub(super) fn detect_database_profile() -> String {
    let mut contents = String::new();
    let Ok(file) = File::open(".env") else {
        return "not configured (.env missing)".to_string();
    };
    if file
        .take(ENV_READ_LIMIT)
        .read_to_string(&mut contents)
        .is_err()
    {
        return "configuration unreadable".to_string();
    }

    database_profile_from_env(&contents)
}

pub(super) fn database_profile_from_env(contents: &str) -> String {
    for line in contents.lines() {
        let Some((name, raw_value)) = line.trim().split_once('=') else {
            continue;
        };
        if name.trim() != "DATABASE_URL" {
            continue;
        }
        let value = raw_value.trim().trim_matches(['"', '\'']);
        return format!("configured: {}", database_kind(value));
    }

    if contents.lines().any(|line| {
        line.trim()
            .strip_prefix("TURSO_DATABASE_URL=")
            .is_some_and(|value| !value.trim().is_empty())
    }) {
        return "configured: Turso/libSQL".to_string();
    }
    "not configured".to_string()
}

/// Names the database family of a connection URL without exposing any part
/// of the URL itself (credentials, hosts or file paths).
pub(in crate::ui) fn database_kind(url: &str) -> &'static str {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        "PostgreSQL"
    } else if url.starts_with("mysql://") {
        "MySQL/MariaDB"
    } else if url.starts_with("sqlite:") {
        "SQLite"
    } else if url.starts_with("libsql://") {
        "Turso/libSQL"
    } else if url.is_empty() {
        "empty DATABASE_URL"
    } else {
        "custom URL"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_profile_reports_configuration_without_exposing_credentials() {
        let postgres =
            database_profile_from_env("DATABASE_URL=postgres://admin:secret@127.0.0.1/app\n");
        assert_eq!(postgres, "configured: PostgreSQL");
        assert!(!postgres.contains("admin"));
        assert!(!postgres.contains("secret"));

        assert_eq!(
            database_profile_from_env("TURSO_DATABASE_URL=libsql://example.turso.io\n"),
            "configured: Turso/libSQL"
        );
        assert_eq!(
            database_profile_from_env("APP_ENV=development\n"),
            "not configured"
        );
    }
}
