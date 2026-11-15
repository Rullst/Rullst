//! PostgreSQL. Exercised against an owned server by
//! `.github/check-auth-session-registry.py`.
//!
//! Concurrent sign-ins of one account can each pass the per-account limit
//! check, so the account may briefly hold more sessions than the limit, by at
//! most their number; the next sign-in evicts the excess.

use super::sql::{Dialect, Statements, backend};
use sqlx::Postgres;

impl Dialect for Postgres {
    const STATEMENTS: Statements = Statements {
        begin: "BEGIN",
        schema: &[
            // Serializes schema preparation by several application processes.
            "SELECT pg_advisory_xact_lock(7212650481207635562)",
            "CREATE TABLE IF NOT EXISTS rullst_auth_sessions (token_hash TEXT PRIMARY KEY NOT NULL CHECK (length(token_hash) = 64), public_id TEXT NOT NULL CHECK (length(public_id) = 22), user_id INTEGER NOT NULL, created_at BIGINT NOT NULL, last_seen_at BIGINT NOT NULL, expires_at BIGINT NOT NULL, browser TEXT NOT NULL, platform TEXT NOT NULL)",
            "CREATE INDEX IF NOT EXISTS rullst_auth_sessions_user ON rullst_auth_sessions (user_id, last_seen_at)",
            "CREATE INDEX IF NOT EXISTS rullst_auth_sessions_expiry ON rullst_auth_sessions (expires_at)",
        ],
        probe: "SELECT COUNT(*) FROM rullst_auth_sessions WHERE 1 = 0",
        prune_user: "DELETE FROM rullst_auth_sessions WHERE user_id = $1 AND expires_at < $2",
        count_user: "SELECT COUNT(*) FROM rullst_auth_sessions WHERE user_id = $1",
        oldest_user: "SELECT token_hash FROM rullst_auth_sessions WHERE user_id = $1 ORDER BY last_seen_at, created_at, token_hash LIMIT $2",
        insert: "INSERT INTO rullst_auth_sessions (token_hash, public_id, user_id, created_at, last_seen_at, expires_at, browser, platform) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        find: "SELECT token_hash, public_id, user_id, created_at, last_seen_at, expires_at, browser, platform FROM rullst_auth_sessions WHERE token_hash = $1",
        touch: "UPDATE rullst_auth_sessions SET last_seen_at = $1 WHERE token_hash = $2 AND last_seen_at < $3",
        delete_hash: "DELETE FROM rullst_auth_sessions WHERE token_hash = $1",
        delete_token: "DELETE FROM rullst_auth_sessions WHERE user_id = $1 AND token_hash = $2",
        delete_reference: "DELETE FROM rullst_auth_sessions WHERE user_id = $1 AND public_id = $2",
        delete_others: "DELETE FROM rullst_auth_sessions WHERE user_id = $1 AND token_hash <> $2",
        delete_user: "DELETE FROM rullst_auth_sessions WHERE user_id = $1",
        list_user: "SELECT token_hash, public_id, user_id, created_at, last_seen_at, expires_at, browser, platform FROM rullst_auth_sessions WHERE user_id = $1 AND expires_at >= $2 ORDER BY last_seen_at DESC, created_at DESC, token_hash LIMIT $3",
        expired: "SELECT token_hash FROM rullst_auth_sessions WHERE expires_at < $1 ORDER BY expires_at, token_hash LIMIT $2",
    };

    fn rows_affected(result: &Self::QueryResult) -> u64 {
        result.rows_affected()
    }
}

backend!(Postgres);
