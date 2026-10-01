//! Dialect-specific SQL text for the durable webhook replay ledger.

use super::SqlWebhookBackend;

pub(super) fn timestamp_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "SELECT CAST(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP) AS BIGINT)"
        }
        SqlWebhookBackend::Mysql => "SELECT UNIX_TIMESTAMP()",
        SqlWebhookBackend::Sqlite => "SELECT CAST(strftime('%s', 'now') AS INTEGER)",
    }
}

pub(super) fn config_schema_sql(backend: SqlWebhookBackend) -> &'static str {
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

pub(super) fn claim_schema_sql(backend: SqlWebhookBackend) -> &'static str {
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

pub(super) fn claim_expiry_index_sql(backend: SqlWebhookBackend) -> Option<&'static str> {
    match backend {
        SqlWebhookBackend::Postgres | SqlWebhookBackend::Sqlite => Some(
            "CREATE INDEX IF NOT EXISTS rullst_webhook_replay_expiry ON rullst_webhook_replay_claims (expires_at)",
        ),
        SqlWebhookBackend::Mysql => None,
    }
}

pub(super) fn insert_config_sql(backend: SqlWebhookBackend) -> &'static str {
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

pub(super) fn insert_claim_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "INSERT INTO rullst_webhook_replay_claims (provider, replay_hash, accepted_at, expires_at) VALUES ($1, $2, $3, $4) ON CONFLICT (provider, replay_hash) DO NOTHING"
        }
        SqlWebhookBackend::Sqlite => {
            "INSERT INTO rullst_webhook_replay_claims (provider, replay_hash, accepted_at, expires_at) VALUES (?, ?, ?, ?) ON CONFLICT (provider, replay_hash) DO NOTHING"
        }
        // Not an upsert: sqlx connects with CLIENT_FOUND_ROWS, so an
        // `ON DUPLICATE KEY UPDATE` no-op would still report one affected row.
        SqlWebhookBackend::Mysql => {
            "INSERT INTO rullst_webhook_replay_claims (provider, replay_hash, accepted_at, expires_at) VALUES (?, ?, ?, ?)"
        }
    }
}

pub(super) fn select_config_sql() -> &'static str {
    "SELECT max_entries, ttl_seconds FROM rullst_webhook_replay_config WHERE singleton = 1"
}

pub(super) fn select_config_for_update_sql() -> &'static str {
    "SELECT max_entries, ttl_seconds FROM rullst_webhook_replay_config WHERE singleton = 1 FOR UPDATE"
}

pub(super) fn lock_sqlite_config_sql() -> &'static str {
    "UPDATE rullst_webhook_replay_config SET singleton = singleton WHERE singleton = 1"
}

pub(super) fn delete_expired_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "DELETE FROM rullst_webhook_replay_claims WHERE expires_at <= $1"
        }
        _ => "DELETE FROM rullst_webhook_replay_claims WHERE expires_at <= ?",
    }
}

pub(super) fn contains_claim_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "SELECT expires_at FROM rullst_webhook_replay_claims WHERE provider = $1 AND replay_hash = $2"
        }
        _ => {
            "SELECT expires_at FROM rullst_webhook_replay_claims WHERE provider = ? AND replay_hash = ?"
        }
    }
}

pub(super) fn active_count_sql() -> &'static str {
    "SELECT COUNT(*) FROM rullst_webhook_replay_claims"
}
