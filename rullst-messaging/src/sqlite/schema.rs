use crate::{BrokerConfig, MessagingError, Result};
use sqlx::{Executor, SqlitePool};

use super::transaction::storage_error;

const SCHEMA_VERSION: i64 = 1;

const STATEMENTS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS rullst_messaging_brokers (namespace TEXT PRIMARY KEY, schema_version INTEGER NOT NULL, max_retained_messages INTEGER NOT NULL, max_subscriptions INTEGER NOT NULL, max_attempts INTEGER NOT NULL, max_payload_bytes INTEGER NOT NULL)",
    "CREATE TABLE IF NOT EXISTS rullst_messaging_topics (namespace TEXT NOT NULL, topic TEXT NOT NULL, next_sequence INTEGER NOT NULL CHECK (next_sequence > 0), PRIMARY KEY (namespace, topic), FOREIGN KEY (namespace) REFERENCES rullst_messaging_brokers(namespace) ON DELETE CASCADE)",
    "CREATE TABLE IF NOT EXISTS rullst_messaging_messages (namespace TEXT NOT NULL, topic TEXT NOT NULL, sequence INTEGER NOT NULL CHECK (sequence > 0), message_id TEXT NOT NULL, event_kind TEXT NOT NULL, content_type TEXT NOT NULL, headers_json TEXT NOT NULL, payload BLOB NOT NULL, published_at_ms INTEGER NOT NULL CHECK (published_at_ms >= 0), idempotency_key TEXT NOT NULL, fingerprint BLOB NOT NULL CHECK (length(fingerprint) = 32), PRIMARY KEY (namespace, topic, sequence), UNIQUE (namespace, topic, idempotency_key), UNIQUE (namespace, message_id), FOREIGN KEY (namespace, topic) REFERENCES rullst_messaging_topics(namespace, topic) ON DELETE CASCADE)",
    "CREATE TABLE IF NOT EXISTS rullst_messaging_subscriptions (namespace TEXT NOT NULL, topic TEXT NOT NULL, group_name TEXT NOT NULL, start_sequence INTEGER NOT NULL CHECK (start_sequence > 0), PRIMARY KEY (namespace, topic, group_name), FOREIGN KEY (namespace, topic) REFERENCES rullst_messaging_topics(namespace, topic) ON DELETE CASCADE)",
    "CREATE TABLE IF NOT EXISTS rullst_messaging_deliveries (namespace TEXT NOT NULL, topic TEXT NOT NULL, group_name TEXT NOT NULL, sequence INTEGER NOT NULL CHECK (sequence > 0), state TEXT NOT NULL CHECK (state IN ('pending','in_flight','acked','dead')), available_at_ms INTEGER, attempt INTEGER NOT NULL DEFAULT 0 CHECK (attempt >= 0), ack_token TEXT, consumer_name TEXT, lease_expires_at_ms INTEGER, failure_code TEXT, dead_lettered_at_ms INTEGER, CHECK ((state = 'pending' AND available_at_ms IS NOT NULL AND ack_token IS NULL AND consumer_name IS NULL AND lease_expires_at_ms IS NULL AND failure_code IS NULL AND dead_lettered_at_ms IS NULL) OR (state = 'in_flight' AND available_at_ms IS NULL AND ack_token IS NOT NULL AND consumer_name IS NOT NULL AND lease_expires_at_ms IS NOT NULL AND failure_code IS NULL AND dead_lettered_at_ms IS NULL) OR (state = 'acked' AND available_at_ms IS NULL AND ack_token IS NULL AND consumer_name IS NULL AND lease_expires_at_ms IS NULL AND failure_code IS NULL AND dead_lettered_at_ms IS NULL) OR (state = 'dead' AND available_at_ms IS NULL AND ack_token IS NULL AND consumer_name IS NULL AND lease_expires_at_ms IS NULL AND failure_code IS NOT NULL AND dead_lettered_at_ms IS NOT NULL)), PRIMARY KEY (namespace, topic, group_name, sequence), UNIQUE (namespace, ack_token), FOREIGN KEY (namespace, topic, sequence) REFERENCES rullst_messaging_messages(namespace, topic, sequence) ON DELETE CASCADE, FOREIGN KEY (namespace, topic, group_name) REFERENCES rullst_messaging_subscriptions(namespace, topic, group_name) ON DELETE CASCADE)",
    "CREATE INDEX IF NOT EXISTS rullst_messaging_pending_idx ON rullst_messaging_deliveries(namespace, topic, group_name, state, available_at_ms, sequence)",
    "CREATE INDEX IF NOT EXISTS rullst_messaging_dead_idx ON rullst_messaging_deliveries(namespace, topic, group_name, state, sequence)",
];

pub(super) async fn prepare(pool: &SqlitePool, config: &BrokerConfig) -> Result<()> {
    for statement in STATEMENTS {
        pool.execute(*statement)
            .await
            .map_err(|_| storage_error("prepare schema"))?;
    }
    let retained = as_i64(config.max_retained_messages(), "retained-message limit")?;
    let subscriptions = as_i64(config.max_subscriptions(), "subscription limit")?;
    let attempts = i64::from(config.max_attempts());
    let payload = as_i64(config.max_payload_bytes(), "payload limit")?;
    sqlx::query("INSERT OR IGNORE INTO rullst_messaging_brokers (namespace, schema_version, max_retained_messages, max_subscriptions, max_attempts, max_payload_bytes) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(config.namespace().as_str())
        .bind(SCHEMA_VERSION)
        .bind(retained)
        .bind(subscriptions)
        .bind(attempts)
        .bind(payload)
        .execute(pool)
        .await
        .map_err(|_| storage_error("register namespace"))?;
    let stored: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT schema_version, max_retained_messages, max_subscriptions, max_attempts, max_payload_bytes FROM rullst_messaging_brokers WHERE namespace = ?",
    )
    .bind(config.namespace().as_str())
    .fetch_one(pool)
    .await
    .map_err(|_| storage_error("read namespace configuration"))?;
    if stored.0 != SCHEMA_VERSION {
        return Err(MessagingError::CorruptStorage {
            context: "schema version",
        });
    }
    if stored.1 != retained
        || stored.2 != subscriptions
        || stored.3 != attempts
        || stored.4 != payload
    {
        return Err(MessagingError::ConfigurationConflict);
    }
    Ok(())
}

fn as_i64(value: usize, context: &'static str) -> Result<i64> {
    i64::try_from(value).map_err(|_| MessagingError::InternalState { context })
}
