//! MySQL and MariaDB. Exercised against owned servers by
//! `.github/check-auth-session-registry.py`.
//!
//! The hash and reference columns use a binary ASCII collation, so they
//! compare exactly (MySQL's default collations ignore case). Concurrent
//! sign-ins of one account can briefly pass the per-account limit, by at most
//! their number, as on PostgreSQL.

use super::sql::{Dialect, Statements, backend};
use sqlx::MySql;

impl Dialect for MySql {
    const STATEMENTS: Statements = Statements {
        begin: "BEGIN",
        schema: &[
            "CREATE TABLE IF NOT EXISTS rullst_auth_sessions (token_hash CHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL PRIMARY KEY, public_id CHAR(22) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, user_id INT NOT NULL, created_at BIGINT NOT NULL, last_seen_at BIGINT NOT NULL, expires_at BIGINT NOT NULL, browser VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, platform VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, INDEX rullst_auth_sessions_user (user_id, last_seen_at), INDEX rullst_auth_sessions_expiry (expires_at))",
        ],
        prune_user: "DELETE FROM rullst_auth_sessions WHERE user_id = ? AND expires_at < ?",
        count_user: "SELECT COUNT(*) FROM rullst_auth_sessions WHERE user_id = ?",
        oldest_user: "SELECT token_hash FROM rullst_auth_sessions WHERE user_id = ? ORDER BY last_seen_at, created_at, token_hash LIMIT ?",
        insert: "INSERT INTO rullst_auth_sessions (token_hash, public_id, user_id, created_at, last_seen_at, expires_at, browser, platform) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        find: "SELECT token_hash, public_id, user_id, created_at, last_seen_at, expires_at, browser, platform FROM rullst_auth_sessions WHERE token_hash = ?",
        touch: "UPDATE rullst_auth_sessions SET last_seen_at = ? WHERE token_hash = ? AND last_seen_at < ?",
        delete_hash: "DELETE FROM rullst_auth_sessions WHERE token_hash = ?",
        delete_token: "DELETE FROM rullst_auth_sessions WHERE user_id = ? AND token_hash = ?",
        delete_reference: "DELETE FROM rullst_auth_sessions WHERE user_id = ? AND public_id = ?",
        delete_others: "DELETE FROM rullst_auth_sessions WHERE user_id = ? AND token_hash <> ?",
        delete_user: "DELETE FROM rullst_auth_sessions WHERE user_id = ?",
        list_user: "SELECT token_hash, public_id, user_id, created_at, last_seen_at, expires_at, browser, platform FROM rullst_auth_sessions WHERE user_id = ? AND expires_at >= ? ORDER BY last_seen_at DESC, created_at DESC, token_hash LIMIT ?",
        expired: "SELECT token_hash FROM rullst_auth_sessions WHERE expires_at < ? ORDER BY expires_at, token_hash LIMIT ?",
    };

    fn rows_affected(result: &Self::QueryResult) -> u64 {
        result.rows_affected()
    }
}

backend!(MySql);
