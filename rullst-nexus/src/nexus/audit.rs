//! Transaction-coupled, minimized mutation audit records for Nexus.

use crate::nexus::NexusPrincipal;
use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_RECORD_KEY_BYTES: usize = 256;
const MAX_CORRELATION_ID_BYTES: usize = 255;
const MAX_AUDIT_PAGE: u16 = 1_000;

/// One persisted Nexus mutation summary.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NexusAuditRecord {
    /// Monotonic database-local audit identifier.
    pub id: i64,
    /// Identity established by the Nexus authentication policy.
    pub actor_id: String,
    /// Trusted tenant identifier for a tenant-scoped model.
    pub tenant_id: Option<String>,
    /// Registered model table.
    pub table_name: String,
    /// Bounded mutation verb (`create`, `update`, `delete`, or batch variant).
    pub action: String,
    /// Exact primary key when the mutation has one known key.
    pub record_key: Option<String>,
    /// Number of rows changed in the same transaction.
    pub record_count: i64,
    /// Terminal outcome persisted by this schema (`committed`).
    pub outcome: String,
    /// Optional validated request correlation identifier.
    pub correlation_id: Option<String>,
    /// Unix epoch milliseconds captured before commit.
    pub occurred_at_ms: i64,
    /// Version of the persisted record contract.
    pub format_version: i32,
}

impl fmt::Debug for NexusAuditRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NexusAuditRecord")
            .field("id", &self.id)
            .field("actor_id_bytes", &self.actor_id.len())
            .field("has_tenant_id", &self.tenant_id.is_some())
            .field("table_name", &self.table_name)
            .field("action", &self.action)
            .field("has_record_key", &self.record_key.is_some())
            .field("record_count", &self.record_count)
            .field("outcome", &self.outcome)
            .field("has_correlation_id", &self.correlation_id.is_some())
            .field("occurred_at_ms", &self.occurred_at_ms)
            .field("format_version", &self.format_version)
            .finish()
    }
}

/// Installs the fixed cross-database audit table used by
/// [`crate::Nexus::with_required_audit`].
///
/// Run this as an explicit migration/deployment step. Nexus never grants
/// itself schema privileges while serving a mutation request.
#[cfg_attr(mutants, mutants::skip)]
pub async fn create_nexus_audit_table() -> Result<(), rullst_orm::Error> {
    let pool = rullst_orm::Orm::try_pool()?;
    let driver = rullst_orm::Orm::try_driver()?;
    rullst_orm::_sqlx::query(create_table_sql(driver))
        .execute(pool)
        .await?;
    verify_nexus_audit_table().await
}

/// Verifies that the complete audit schema is queryable without mutating it.
#[cfg_attr(mutants, mutants::skip)]
pub async fn verify_nexus_audit_table() -> Result<(), rullst_orm::Error> {
    let pool = rullst_orm::Orm::try_pool()?;
    rullst_orm::_sqlx::query(
        "SELECT id, actor_id, tenant_id, table_name, action, record_key, \
         record_count, outcome, correlation_id, occurred_at_ms, format_version \
         FROM rullst_nexus_audits WHERE 1 = 0",
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Loads a bounded page of newest mutation summaries for operational export.
///
/// Supplying a tenant ID restricts the query to that exact persisted tenant.
/// The caller remains responsible for authorizing access to audit records.
#[cfg_attr(mutants, mutants::skip)]
pub async fn recent_nexus_audits(
    limit: u16,
    tenant_id: Option<&str>,
) -> Result<Vec<NexusAuditRecord>, rullst_orm::Error> {
    if limit == 0 || limit > MAX_AUDIT_PAGE {
        return Err(rullst_orm::Error::Validation(
            "Nexus audit page must contain between 1 and 1000 records".to_string(),
        ));
    }
    if let Some(tenant_id) = tenant_id {
        validate_audit_text("tenant ID", tenant_id, 128)?;
    }
    let driver = rullst_orm::Orm::try_driver()?;
    let filter = match (driver, tenant_id) {
        ("postgres", Some(_)) => " WHERE tenant_id = $1",
        (_, Some(_)) => " WHERE tenant_id = ?",
        (_, None) => "",
    };
    let sql = format!(
        "SELECT id, actor_id, tenant_id, table_name, action, record_key, record_count, \
         outcome, correlation_id, occurred_at_ms, format_version \
         FROM rullst_nexus_audits{filter} \
         ORDER BY id DESC LIMIT {limit}"
    );
    let query =
        rullst_orm::_sqlx::query_as::<_, AuditRow>(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    let rows = if let Some(tenant_id) = tenant_id {
        query
            .bind(tenant_id)
            .fetch_all(rullst_orm::Orm::try_pool()?)
            .await?
    } else {
        query.fetch_all(rullst_orm::Orm::try_pool()?).await?
    };
    Ok(rows.into_iter().map(decode_audit_row).collect())
}

type AuditRow = (
    i64,
    String,
    Option<String>,
    String,
    String,
    Option<String>,
    i64,
    String,
    Option<String>,
    i64,
    i32,
);

fn decode_audit_row(row: AuditRow) -> NexusAuditRecord {
    NexusAuditRecord {
        id: row.0,
        actor_id: row.1,
        tenant_id: row.2,
        table_name: row.3,
        action: row.4,
        record_key: row.5,
        record_count: row.6,
        outcome: row.7,
        correlation_id: row.8,
        occurred_at_ms: row.9,
        format_version: row.10,
    }
}

pub(crate) fn correlation_id(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            validate_audit_text("correlation ID", value, MAX_CORRELATION_ID_BYTES).is_ok()
        })
        .map(str::to_string)
}

pub(crate) struct MutationAudit<'a> {
    pub principal: &'a NexusPrincipal,
    pub tenant_id: Option<&'a str>,
    pub table_name: &'a str,
    pub action: &'static str,
    pub record_key: Option<&'a str>,
    pub record_count: u64,
    pub correlation_id: Option<&'a str>,
}

pub(crate) async fn append_mutation(
    transaction: &mut rullst_orm::db::Transaction<'_>,
    audit: &MutationAudit<'_>,
) -> Result<(), rullst_orm::Error> {
    validate_audit_text("actor ID", audit.principal.actor_id(), 255)?;
    validate_audit_text("table name", audit.table_name, 64)?;
    validate_audit_text("action", audit.action, 32)?;
    if let Some(value) = audit.tenant_id {
        validate_audit_text("tenant ID", value, 128)?;
    }
    if let Some(value) = audit.record_key {
        validate_audit_text("record key", value, MAX_RECORD_KEY_BYTES)?;
    }
    if let Some(value) = audit.correlation_id {
        validate_audit_text("correlation ID", value, MAX_CORRELATION_ID_BYTES)?;
    }
    let record_count = i64::try_from(audit.record_count).map_err(|_| {
        rullst_orm::Error::Validation("Nexus audit record count is too large".to_string())
    })?;
    if record_count <= 0 {
        return Err(rullst_orm::Error::Validation(
            "Nexus audit record count must be positive".to_string(),
        ));
    }
    let occurred_at_ms = unix_epoch_millis()?;
    let sql = "INSERT INTO rullst_nexus_audits (actor_id, tenant_id, table_name, action, \
               record_key, record_count, outcome, correlation_id, occurred_at_ms, format_version) \
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
    let sql = if rullst_orm::Orm::try_driver()? == "postgres" {
        rullst_orm::replace_placeholders(sql)
    } else {
        sql.to_string()
    };
    rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()))
        .bind(audit.principal.actor_id())
        .bind(audit.tenant_id)
        .bind(audit.table_name)
        .bind(audit.action)
        .bind(audit.record_key)
        .bind(record_count)
        .bind("committed")
        .bind(audit.correlation_id)
        .bind(occurred_at_ms)
        .bind(1_i32)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

fn unix_epoch_millis() -> Result<i64, rullst_orm::Error> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| rullst_orm::Error::Internal("system clock predates Unix epoch".to_string()))?;
    i64::try_from(duration.as_millis()).map_err(|_| {
        rullst_orm::Error::Internal("system clock exceeds Nexus audit range".to_string())
    })
}

fn validate_audit_text(
    label: &'static str,
    value: &str,
    maximum: usize,
) -> Result<(), rullst_orm::Error> {
    if value.is_empty()
        || value.len() > maximum
        || value.trim().len() != value.len()
        || value.chars().any(char::is_control)
    {
        return Err(rullst_orm::Error::Validation(format!(
            "Nexus audit {label} must contain 1 to {maximum} unpadded bytes without controls"
        )));
    }
    Ok(())
}

fn create_table_sql(driver: &str) -> &'static str {
    match driver {
        "postgres" => {
            "CREATE TABLE IF NOT EXISTS rullst_nexus_audits (\
             id BIGSERIAL PRIMARY KEY, actor_id VARCHAR(255) NOT NULL, tenant_id VARCHAR(128), \
             table_name VARCHAR(64) NOT NULL, action VARCHAR(32) NOT NULL, \
             record_key VARCHAR(256), record_count BIGINT NOT NULL, outcome VARCHAR(16) NOT NULL, \
             correlation_id VARCHAR(255), occurred_at_ms BIGINT NOT NULL, \
             format_version INT NOT NULL)"
        }
        "mysql" => {
            "CREATE TABLE IF NOT EXISTS rullst_nexus_audits (\
             id BIGINT AUTO_INCREMENT PRIMARY KEY, actor_id VARCHAR(255) NOT NULL, \
             tenant_id VARCHAR(128), table_name VARCHAR(64) NOT NULL, action VARCHAR(32) NOT NULL, \
             record_key VARCHAR(256), record_count BIGINT NOT NULL, outcome VARCHAR(16) NOT NULL, \
             correlation_id VARCHAR(255), occurred_at_ms BIGINT NOT NULL, \
             format_version INT NOT NULL)"
        }
        _ => {
            "CREATE TABLE IF NOT EXISTS rullst_nexus_audits (\
             id INTEGER PRIMARY KEY AUTOINCREMENT, actor_id TEXT NOT NULL, tenant_id TEXT, \
             table_name TEXT NOT NULL, action TEXT NOT NULL, record_key TEXT, \
             record_count INTEGER NOT NULL, outcome TEXT NOT NULL, correlation_id TEXT, \
             occurred_at_ms INTEGER NOT NULL, format_version INTEGER NOT NULL)"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_text_and_limits_fail_closed() {
        assert!(validate_audit_text("actor", "admin", 16).is_ok());
        assert!(validate_audit_text("actor", " admin", 16).is_err());
        assert!(validate_audit_text("actor", "a\n", 16).is_err());
        assert!(validate_audit_text("actor", "12345", 4).is_err());
    }

    #[test]
    // TM-NEXUS-06
    fn schemas_keep_required_identity_and_scope_columns() {
        for driver in ["sqlite", "postgres", "mysql"] {
            let schema = create_table_sql(driver);
            for column in [
                "actor_id",
                "tenant_id",
                "table_name",
                "record_count",
                "outcome",
                "correlation_id",
                "occurred_at_ms",
                "format_version",
            ] {
                assert!(schema.contains(column), "{driver} schema omitted {column}");
            }
        }
    }
}
