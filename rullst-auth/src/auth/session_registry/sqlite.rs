//! SQLite: `BEGIN IMMEDIATE` sign-ins, so the per-account limit is exact.

use super::sql::{Dialect, Statements, backend};
use sqlx::Sqlite;

impl Dialect for Sqlite {
    const STATEMENTS: Statements = Statements {
        begin: "BEGIN IMMEDIATE",
        schema: &[
            "CREATE TABLE IF NOT EXISTS rullst_auth_sessions (token_hash TEXT PRIMARY KEY NOT NULL CHECK (length(token_hash) = 64), public_id TEXT NOT NULL CHECK (length(public_id) = 22), user_id INTEGER NOT NULL, created_at INTEGER NOT NULL, last_seen_at INTEGER NOT NULL, expires_at INTEGER NOT NULL, browser TEXT NOT NULL, platform TEXT NOT NULL)",
            "CREATE INDEX IF NOT EXISTS rullst_auth_sessions_user ON rullst_auth_sessions (user_id, last_seen_at)",
            "CREATE INDEX IF NOT EXISTS rullst_auth_sessions_expiry ON rullst_auth_sessions (expires_at)",
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

backend!(Sqlite);
