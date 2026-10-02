//! Per-dialect SQL statements of [`super::SqlQuotaStore`].

use super::{SqlQuotaBackend, mysql};

pub(super) fn schema_sql(backend: SqlQuotaBackend) -> (&'static str, &'static str) {
    match backend {
        SqlQuotaBackend::Postgres => (POSTGRES_COUNTERS, POSTGRES_CLAIMS),
        SqlQuotaBackend::Mysql => (mysql::COUNTERS, mysql::CLAIMS),
        SqlQuotaBackend::Sqlite => (SQLITE_COUNTERS, SQLITE_CLAIMS),
    }
}

pub(super) fn insert_claim_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_INSERT_CLAIM,
        SqlQuotaBackend::Mysql => mysql::INSERT_CLAIM,
        SqlQuotaBackend::Sqlite => SQLITE_INSERT_CLAIM,
    }
}

pub(super) fn insert_counter_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_INSERT_COUNTER,
        SqlQuotaBackend::Mysql => mysql::INSERT_COUNTER,
        SqlQuotaBackend::Sqlite => SQLITE_INSERT_COUNTER,
    }
}

pub(super) fn select_claim_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_SELECT_CLAIM,
        _ => PORTABLE_SELECT_CLAIM,
    }
}

pub(super) fn update_counter_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_UPDATE_COUNTER,
        _ => PORTABLE_UPDATE_COUNTER,
    }
}

pub(super) fn update_claim_usage_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_UPDATE_CLAIM_USAGE,
        _ => PORTABLE_UPDATE_CLAIM_USAGE,
    }
}

pub(super) fn select_usage_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_SELECT_USAGE,
        _ => PORTABLE_SELECT_USAGE,
    }
}

pub(super) fn delete_claim_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_DELETE_CLAIM,
        _ => PORTABLE_DELETE_CLAIM,
    }
}

pub(super) fn decrement_counter_sql(backend: SqlQuotaBackend) -> &'static str {
    match backend {
        SqlQuotaBackend::Postgres => POSTGRES_DECREMENT_COUNTER,
        _ => PORTABLE_DECREMENT_COUNTER,
    }
}

const POSTGRES_COUNTERS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_counters (subject_kind VARCHAR(32) NOT NULL, subject_id VARCHAR(128) NOT NULL, feature VARCHAR(128) NOT NULL, used_units BIGINT NOT NULL DEFAULT 0 CHECK (used_units >= 0), PRIMARY KEY (subject_kind, subject_id, feature))";
const POSTGRES_CLAIMS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_claims (subject_kind VARCHAR(32) NOT NULL, subject_id VARCHAR(128) NOT NULL, feature VARCHAR(128) NOT NULL, event_key VARCHAR(128) NOT NULL, units BIGINT NOT NULL CHECK (units > 0), limit_at_claim BIGINT NOT NULL CHECK (limit_at_claim > 0), used_after BIGINT NOT NULL CHECK (used_after >= 0), claim_token VARCHAR(32) NOT NULL, PRIMARY KEY (subject_kind, subject_id, feature, event_key))";
const SQLITE_COUNTERS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_counters (subject_kind TEXT NOT NULL, subject_id TEXT NOT NULL, feature TEXT NOT NULL, used_units INTEGER NOT NULL DEFAULT 0 CHECK (used_units >= 0), PRIMARY KEY (subject_kind, subject_id, feature))";
const SQLITE_CLAIMS: &str = "CREATE TABLE IF NOT EXISTS rullst_capital_quota_claims (subject_kind TEXT NOT NULL, subject_id TEXT NOT NULL, feature TEXT NOT NULL, event_key TEXT NOT NULL, units INTEGER NOT NULL CHECK (units > 0), limit_at_claim INTEGER NOT NULL CHECK (limit_at_claim > 0), used_after INTEGER NOT NULL CHECK (used_after >= 0), claim_token TEXT NOT NULL, PRIMARY KEY (subject_kind, subject_id, feature, event_key))";

const POSTGRES_INSERT_CLAIM: &str = "INSERT INTO rullst_capital_quota_claims (subject_kind, subject_id, feature, event_key, units, limit_at_claim, used_after, claim_token) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (subject_kind, subject_id, feature, event_key) DO NOTHING";
const SQLITE_INSERT_CLAIM: &str = "INSERT INTO rullst_capital_quota_claims (subject_kind, subject_id, feature, event_key, units, limit_at_claim, used_after, claim_token) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT (subject_kind, subject_id, feature, event_key) DO NOTHING";
const POSTGRES_INSERT_COUNTER: &str = "INSERT INTO rullst_capital_quota_counters (subject_kind, subject_id, feature, used_units) VALUES ($1, $2, $3, $4) ON CONFLICT (subject_kind, subject_id, feature) DO NOTHING";
const SQLITE_INSERT_COUNTER: &str = "INSERT INTO rullst_capital_quota_counters (subject_kind, subject_id, feature, used_units) VALUES (?, ?, ?, ?) ON CONFLICT (subject_kind, subject_id, feature) DO NOTHING";

const POSTGRES_SELECT_CLAIM: &str = "SELECT units, limit_at_claim, used_after, claim_token FROM rullst_capital_quota_claims WHERE subject_kind = $1 AND subject_id = $2 AND feature = $3 AND event_key = $4";
const PORTABLE_SELECT_CLAIM: &str = "SELECT units, limit_at_claim, used_after, claim_token FROM rullst_capital_quota_claims WHERE subject_kind = ? AND subject_id = ? AND feature = ? AND event_key = ?";
const POSTGRES_UPDATE_COUNTER: &str = "UPDATE rullst_capital_quota_counters SET used_units = used_units + $1 WHERE subject_kind = $2 AND subject_id = $3 AND feature = $4 AND used_units <= $5";
const PORTABLE_UPDATE_COUNTER: &str = "UPDATE rullst_capital_quota_counters SET used_units = used_units + ? WHERE subject_kind = ? AND subject_id = ? AND feature = ? AND used_units <= ?";
const POSTGRES_UPDATE_CLAIM_USAGE: &str = "UPDATE rullst_capital_quota_claims SET used_after = $1 WHERE subject_kind = $2 AND subject_id = $3 AND feature = $4 AND event_key = $5 AND claim_token = $6";
const PORTABLE_UPDATE_CLAIM_USAGE: &str = "UPDATE rullst_capital_quota_claims SET used_after = ? WHERE subject_kind = ? AND subject_id = ? AND feature = ? AND event_key = ? AND claim_token = ?";
const POSTGRES_SELECT_USAGE: &str = "SELECT used_units FROM rullst_capital_quota_counters WHERE subject_kind = $1 AND subject_id = $2 AND feature = $3";
const PORTABLE_SELECT_USAGE: &str = "SELECT used_units FROM rullst_capital_quota_counters WHERE subject_kind = ? AND subject_id = ? AND feature = ?";
const POSTGRES_DELETE_CLAIM: &str = "DELETE FROM rullst_capital_quota_claims WHERE subject_kind = $1 AND subject_id = $2 AND feature = $3 AND event_key = $4 AND claim_token = $5";
const PORTABLE_DELETE_CLAIM: &str = "DELETE FROM rullst_capital_quota_claims WHERE subject_kind = ? AND subject_id = ? AND feature = ? AND event_key = ? AND claim_token = ?";
const POSTGRES_DECREMENT_COUNTER: &str = "UPDATE rullst_capital_quota_counters SET used_units = used_units - $1 WHERE subject_kind = $2 AND subject_id = $3 AND feature = $4 AND used_units >= $5";
const PORTABLE_DECREMENT_COUNTER: &str = "UPDATE rullst_capital_quota_counters SET used_units = used_units - ? WHERE subject_kind = ? AND subject_id = ? AND feature = ? AND used_units >= ?";
