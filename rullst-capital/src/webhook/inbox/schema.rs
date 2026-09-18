use crate::SqlWebhookBackend;

pub(super) fn config_schema(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Mysql => {
            "CREATE TABLE IF NOT EXISTS rullst_stripe_inbox_config_v1 (scope_hash VARCHAR(64) NOT NULL PRIMARY KEY, capacity BIGINT NOT NULL) ENGINE=InnoDB"
        }
        SqlWebhookBackend::Postgres | SqlWebhookBackend::Sqlite => {
            "CREATE TABLE IF NOT EXISTS rullst_stripe_inbox_config_v1 (scope_hash VARCHAR(64) NOT NULL PRIMARY KEY, capacity BIGINT NOT NULL)"
        }
    }
}

pub(super) fn event_schema(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Mysql => {
            "CREATE TABLE IF NOT EXISTS rullst_stripe_inbox_v1 (scope_hash VARCHAR(64) NOT NULL, event_hash VARCHAR(64) NOT NULL, mutation_hash VARCHAR(64) NOT NULL, outcome VARCHAR(8) NOT NULL, event_created_at BIGINT NOT NULL, PRIMARY KEY (scope_hash, event_hash)) ENGINE=InnoDB"
        }
        SqlWebhookBackend::Postgres | SqlWebhookBackend::Sqlite => {
            "CREATE TABLE IF NOT EXISTS rullst_stripe_inbox_v1 (scope_hash VARCHAR(64) NOT NULL, event_hash VARCHAR(64) NOT NULL, mutation_hash VARCHAR(64) NOT NULL, outcome VARCHAR(8) NOT NULL, event_created_at BIGINT NOT NULL, PRIMARY KEY (scope_hash, event_hash))"
        }
    }
}

pub(super) fn insert_config(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "INSERT INTO rullst_stripe_inbox_config_v1 (scope_hash, capacity) VALUES ($1, $2) ON CONFLICT (scope_hash) DO NOTHING"
        }
        SqlWebhookBackend::Sqlite => {
            "INSERT INTO rullst_stripe_inbox_config_v1 (scope_hash, capacity) VALUES (?, ?) ON CONFLICT (scope_hash) DO NOTHING"
        }
        SqlWebhookBackend::Mysql => {
            "INSERT INTO rullst_stripe_inbox_config_v1 (scope_hash, capacity) VALUES (?, ?) ON DUPLICATE KEY UPDATE scope_hash = VALUES(scope_hash)"
        }
    }
}

/// The update takes a write lock before any admission read, including SQLite.
pub(super) fn lock_scope(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "UPDATE rullst_stripe_inbox_config_v1 SET capacity = capacity WHERE scope_hash = $1"
        }
        _ => "UPDATE rullst_stripe_inbox_config_v1 SET capacity = capacity WHERE scope_hash = ?",
    }
}

pub(super) fn capacity(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "SELECT capacity FROM rullst_stripe_inbox_config_v1 WHERE scope_hash = $1"
        }
        _ => "SELECT capacity FROM rullst_stripe_inbox_config_v1 WHERE scope_hash = ?",
    }
}

pub(super) fn existing(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "SELECT mutation_hash, outcome FROM rullst_stripe_inbox_v1 WHERE scope_hash = $1 AND event_hash = $2"
        }
        _ => {
            "SELECT mutation_hash, outcome FROM rullst_stripe_inbox_v1 WHERE scope_hash = ? AND event_hash = ?"
        }
    }
}

pub(super) fn count(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "SELECT COUNT(*) FROM rullst_stripe_inbox_v1 WHERE scope_hash = $1"
        }
        _ => "SELECT COUNT(*) FROM rullst_stripe_inbox_v1 WHERE scope_hash = ?",
    }
}

pub(super) fn insert_event(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => {
            "INSERT INTO rullst_stripe_inbox_v1 (scope_hash, event_hash, mutation_hash, outcome, event_created_at) VALUES ($1, $2, $3, $4, $5)"
        }
        _ => {
            "INSERT INTO rullst_stripe_inbox_v1 (scope_hash, event_hash, mutation_hash, outcome, event_created_at) VALUES (?, ?, ?, ?, ?)"
        }
    }
}
