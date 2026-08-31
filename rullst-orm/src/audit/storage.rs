use super::{
    context::current_metadata,
    diff::{self, compute_diff},
    revision::build_reverse_patch,
};
use serde::{Deserialize, Serialize};
use std::fmt;

const MAX_PAYLOAD_LEN: usize = 5 * 1024 * 1024;

/// One persisted mutation record.
#[derive(Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i32,
    pub model_type: String,
    pub model_id: i32,
    pub event: String,
    pub old_values: Option<String>,
    pub new_values: Option<String>,
    pub actor_kind: String,
    pub actor_id: String,
    pub tenant_key: Option<String>,
    pub correlation_id: Option<String>,
    pub reverted_audit_id: Option<i32>,
    pub reason: Option<String>,
    pub format_version: i32,
    pub restore_patch: Option<String>,
    pub created_at: Option<String>,
}

impl fmt::Debug for AuditLog {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuditLog")
            .field("id", &self.id)
            .field("model_type", &self.model_type)
            .field("model_id", &self.model_id)
            .field("event", &self.event)
            .field("has_old_values", &self.old_values.is_some())
            .field("has_new_values", &self.new_values.is_some())
            .field("actor_kind", &self.actor_kind)
            .field("has_tenant_key", &self.tenant_key.is_some())
            .field("has_correlation_id", &self.correlation_id.is_some())
            .field("reverted_audit_id", &self.reverted_audit_id)
            .field("has_reason", &self.reason.is_some())
            .field("format_version", &self.format_version)
            .field("has_restore_patch", &self.restore_patch.is_some())
            .field("created_at", &self.created_at)
            .finish()
    }
}

struct PreparedAudit {
    model_type: String,
    model_id: i32,
    event: String,
    old_values: Option<String>,
    new_values: Option<String>,
    restore_patch: Option<String>,
    metadata: super::context::AuditMetadata,
}

fn prepare_audit(
    model_type: &str,
    model_id: i32,
    event: &str,
    old_values: Option<String>,
    new_values: Option<String>,
    restore_patch: Option<String>,
) -> Result<PreparedAudit, crate::Error> {
    if model_id <= 0 {
        return Err(crate::Error::Validation(
            "audit model ID must be positive".to_string(),
        ));
    }
    if model_type.is_empty()
        || model_type.len() > 255
        || model_type.trim().len() != model_type.len()
        || event.is_empty()
        || event.len() > 50
        || event.trim().len() != event.len()
        || model_type.chars().any(char::is_control)
        || event.chars().any(char::is_control)
    {
        return Err(crate::Error::Validation(
            "audit model type or event is invalid".to_string(),
        ));
    }
    let old_values = sanitize_payload(old_values)?;
    let new_values = sanitize_payload(new_values)?;
    Ok(PreparedAudit {
        model_type: model_type.to_string(),
        model_id,
        event: event.to_string(),
        old_values,
        new_values,
        restore_patch,
        metadata: current_metadata()?,
    })
}

fn sanitize_payload(payload: Option<String>) -> Result<Option<String>, crate::Error> {
    let Some(payload) = payload else {
        return Ok(None);
    };
    if payload.len() > MAX_PAYLOAD_LEN {
        return Ok(Some(r#"{"error":"payload_too_large"}"#.to_string()));
    }
    let value = match serde_json::from_str(&payload) {
        Ok(value) => diff::mask_nested(value),
        Err(_) => serde_json::json!({
            "$audit_error": "invalid_json",
            "bytes": payload.len(),
        }),
    };
    Ok(Some(serde_json::to_string(&value)?))
}

fn insert_sql() -> Result<String, crate::Error> {
    let sql = "INSERT INTO rullst_audits (model_type, model_id, event, old_values, new_values, actor_kind, actor_id, tenant_key, correlation_id, reverted_audit_id, reason, format_version, restore_patch) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
    if crate::Orm::driver()? == "postgres" {
        Ok(crate::replace_placeholders(sql))
    } else {
        Ok(sql.to_string())
    }
}

async fn insert_prepared(entry: &PreparedAudit) -> Result<(), crate::Error> {
    let sql = insert_sql()?;
    let query = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(&entry.model_type)
        .bind(entry.model_id)
        .bind(&entry.event)
        .bind(&entry.old_values)
        .bind(&entry.new_values)
        .bind(entry.metadata.actor_kind)
        .bind(&entry.metadata.actor_id)
        .bind(&entry.metadata.tenant_key)
        .bind(&entry.metadata.correlation_id)
        .bind(entry.metadata.reverted_audit_id)
        .bind(&entry.metadata.reason)
        .bind(2_i32)
        .bind(&entry.restore_patch);
    crate::execute_query!(query, execute, pool)?;
    Ok(())
}

async fn insert_prepared_with_tx(
    tx: &mut crate::db::Transaction<'_>,
    entry: &PreparedAudit,
) -> Result<(), crate::Error> {
    let sql = insert_sql()?;
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(&entry.model_type)
        .bind(entry.model_id)
        .bind(&entry.event)
        .bind(&entry.old_values)
        .bind(&entry.new_values)
        .bind(entry.metadata.actor_kind)
        .bind(&entry.metadata.actor_id)
        .bind(&entry.metadata.tenant_key)
        .bind(&entry.metadata.correlation_id)
        .bind(entry.metadata.reverted_audit_id)
        .bind(&entry.metadata.reason)
        .bind(2_i32)
        .bind(&entry.restore_patch)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Persists an audit record, reusing a task-scoped ORM transaction when present.
#[cfg_attr(test, mutants::skip)]
pub async fn log_audit(
    model_type: &str,
    model_id: i32,
    event: &str,
    old_values: Option<String>,
    new_values: Option<String>,
) -> Result<(), crate::Error> {
    let entry = prepare_audit(model_type, model_id, event, old_values, new_values, None)?;
    insert_prepared(&entry).await
}

/// Persists an audit record in a caller-owned transaction.
#[cfg_attr(test, mutants::skip)]
pub async fn log_audit_with_tx(
    tx: &mut crate::db::Transaction<'_>,
    model_type: &str,
    model_id: i32,
    event: &str,
    old_values: Option<String>,
    new_values: Option<String>,
) -> Result<(), crate::Error> {
    let entry = prepare_audit(model_type, model_id, event, old_values, new_values, None)?;
    insert_prepared_with_tx(tx, &entry).await
}

fn diff_payloads(old_json: &str, new_json: &str) -> Option<(Option<String>, Option<String>)> {
    if old_json.len() > MAX_PAYLOAD_LEN || new_json.len() > MAX_PAYLOAD_LEN {
        let marker = Some(r#"{"error":"payload_too_large_for_diff"}"#.to_string());
        return Some((marker.clone(), marker));
    }
    let payloads = compute_diff(old_json, new_json);
    (payloads.0.is_some() || payloads.1.is_some()).then_some(payloads)
}

/// Persists a recursively redacted bounded difference when values changed.
#[cfg_attr(test, mutants::skip)]
pub async fn log_audit_diff(
    model_type: &str,
    model_id: i32,
    event: &str,
    old_json: &str,
    new_json: &str,
) -> Result<(), crate::Error> {
    if let Some((old_values, new_values)) = diff_payloads(old_json, new_json) {
        let restore_patch = restore_patch_for(old_json, new_json)?;
        let entry = prepare_audit(
            model_type,
            model_id,
            event,
            old_values,
            new_values,
            restore_patch,
        )?;
        insert_prepared(&entry).await?;
    }
    Ok(())
}

/// Persists a bounded audit difference in a caller-owned transaction.
#[cfg_attr(test, mutants::skip)]
pub async fn log_audit_diff_with_tx(
    tx: &mut crate::db::Transaction<'_>,
    model_type: &str,
    model_id: i32,
    event: &str,
    old_json: &str,
    new_json: &str,
) -> Result<(), crate::Error> {
    if let Some((old_values, new_values)) = diff_payloads(old_json, new_json) {
        let restore_patch = restore_patch_for(old_json, new_json)?;
        let entry = prepare_audit(
            model_type,
            model_id,
            event,
            old_values,
            new_values,
            restore_patch,
        )?;
        insert_prepared_with_tx(tx, &entry).await?;
    }
    Ok(())
}

fn restore_patch_for(old_json: &str, new_json: &str) -> Result<Option<String>, crate::Error> {
    if old_json.len() > MAX_PAYLOAD_LEN || new_json.len() > MAX_PAYLOAD_LEN {
        return Ok(None);
    }
    build_reverse_patch(old_json, new_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditContext, with_audit_context};

    #[tokio::test]
    async fn preparation_requires_context_and_bounds_payloads() {
        assert!(matches!(
            prepare_audit("User", 1, "created", None, None, None),
            Err(crate::Error::Validation(_))
        ));
        let context = AuditContext::system("unit-test").expect("valid context");
        let entry = with_audit_context(context, async {
            prepare_audit(
                "User",
                1,
                "created",
                Some("A".repeat(MAX_PAYLOAD_LEN + 1)),
                Some("B".repeat(MAX_PAYLOAD_LEN + 1)),
                None,
            )
        })
        .await
        .expect("prepared audit");
        assert_eq!(
            entry.old_values.as_deref(),
            Some(r#"{"error":"payload_too_large"}"#)
        );
        assert_eq!(entry.new_values, entry.old_values);
    }
}
