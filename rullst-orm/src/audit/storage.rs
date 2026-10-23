use super::{
    context::current_metadata,
    diff::{self, compute_diff},
    revision::build_reverse_patch_with_redacted,
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

fn diff_payloads(
    old_json: &str,
    new_json: &str,
    redacted_changes: &[&str],
) -> Option<(Option<String>, Option<String>)> {
    if old_json.len() > MAX_PAYLOAD_LEN || new_json.len() > MAX_PAYLOAD_LEN {
        let marker = Some(r#"{"error":"payload_too_large_for_diff"}"#.to_string());
        return Some((marker.clone(), marker));
    }
    let payloads = diff::with_redacted_changes(compute_diff(old_json, new_json), redacted_changes);
    // Both diff helpers return both sides or neither.
    payloads.0.is_some().then_some(payloads)
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
    if let Some((old_values, new_values)) = diff_payloads(old_json, new_json, &[]) {
        let restore_patch = restore_patch_for(old_json, new_json, &[]);
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
    log_audit_diff_redacted_with_tx(tx, model_type, model_id, event, old_json, new_json, &[]).await
}

/// Persists a bounded audit difference in a caller-owned transaction and
/// records each `redacted_changes` field as a changed `***` value.
///
/// Generated models pass the `#[orm(encrypted)]`/`#[orm(masked)]` fields whose
/// in-memory values differ. Their payload values are already redacted, so the
/// change is recorded without the value and the revision is not restorable.
#[doc(hidden)]
#[cfg_attr(test, mutants::skip)]
pub async fn log_audit_diff_redacted_with_tx(
    tx: &mut crate::db::Transaction<'_>,
    model_type: &str,
    model_id: i32,
    event: &str,
    old_json: &str,
    new_json: &str,
    redacted_changes: &[&str],
) -> Result<(), crate::Error> {
    if let Some((old_values, new_values)) = diff_payloads(old_json, new_json, redacted_changes) {
        let restore_patch = restore_patch_for(old_json, new_json, redacted_changes);
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

/// Builds the revision's reverse patch, or `None` when the revision cannot be
/// restored. A patch that exceeds its depth, operation or size bound (or whose
/// input cannot be parsed) only makes the revision non-restorable: the audit
/// diff is still recorded and the audited write is not rejected.
fn restore_patch_for(old_json: &str, new_json: &str, redacted_changes: &[&str]) -> Option<String> {
    if old_json.len() > MAX_PAYLOAD_LEN || new_json.len() > MAX_PAYLOAD_LEN {
        return None;
    }
    build_reverse_patch_with_redacted(old_json, new_json, redacted_changes)
        .ok()
        .flatten()
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

    /// A JSON string document of exactly `len` bytes.
    fn json_of_len(len: usize) -> String {
        format!("\"{}\"", "a".repeat(len - 2))
    }

    #[tokio::test]
    async fn preparation_bounds_type_event_and_id() {
        let context = AuditContext::system("unit-test").expect("valid context");
        let results = with_audit_context(context, async {
            let prepare = |model_type: &str, model_id: i32, event: &str| {
                prepare_audit(model_type, model_id, event, None, None, None).map(|_| ())
            };
            vec![
                prepare(&"T".repeat(255), 1, &"e".repeat(50)),
                prepare("User", 0, "created"),
                prepare("", 1, "created"),
                prepare(&"T".repeat(256), 1, "created"),
                prepare(" User", 1, "created"),
                prepare("Us\ter", 1, "created"),
                prepare("User", 1, ""),
                prepare("User", 1, &"e".repeat(51)),
                prepare("User", 1, "created "),
                prepare("User", 1, "cre\nated"),
            ]
        })
        .await;
        let (accepted, rejected) = results.split_first().expect("results");
        assert!(accepted.is_ok(), "{accepted:?}");
        let messages: Vec<String> = rejected
            .iter()
            .map(|result| result.as_ref().expect_err("rejected").to_string())
            .collect();
        assert_eq!(
            messages[0],
            "Validation error: audit model ID must be positive"
        );
        for message in &messages[1..] {
            assert_eq!(
                message,
                "Validation error: audit model type or event is invalid"
            );
        }
    }

    #[tokio::test]
    async fn payloads_up_to_the_limit_are_kept() {
        let context = AuditContext::system("unit-test").expect("valid context");
        let exact = json_of_len(MAX_PAYLOAD_LEN);
        let entry = with_audit_context(context, async {
            prepare_audit("User", 1, "created", Some(exact.clone()), None, None)
        })
        .await
        .expect("prepared audit");
        assert_eq!(entry.old_values.as_deref(), Some(exact.as_str()));
        assert_eq!(entry.new_values, None);
    }

    #[test]
    fn diffs_and_restore_patches_respect_the_payload_limit_on_each_side() {
        let exact = json_of_len(MAX_PAYLOAD_LEN);
        let over = json_of_len(MAX_PAYLOAD_LEN + 1);
        let marker = Some(r#"{"error":"payload_too_large_for_diff"}"#.to_string());
        for (old, new) in [(exact.as_str(), "\"b\""), ("\"b\"", exact.as_str())] {
            let diff = diff_payloads(old, new, &[]).expect("changed");
            assert_ne!(diff.0, marker);
            assert_ne!(diff.1, marker);
        }
        for (old, new) in [(over.as_str(), "\"b\""), ("\"b\"", over.as_str())] {
            assert_eq!(
                diff_payloads(old, new, &[]),
                Some((marker.clone(), marker.clone()))
            );
            assert_eq!(restore_patch_for(old, new, &[]), None);
        }
        assert_eq!(diff_payloads(r#"{"a":1}"#, r#"{"a":1}"#, &[]), None);
        assert_eq!(
            diff_payloads(r#"{"a":1}"#, r#"{"a":1}"#, &["secret"]),
            Some((
                Some(r#"{"secret":"***"}"#.to_string()),
                Some(r#"{"secret":"***"}"#.to_string())
            ))
        );
        // Whitespace keeps the documents, and so the patch, small.
        let padded = |json: &str, len: usize| format!("{json}{}", " ".repeat(len - json.len()));
        let (old_exact, new_exact) = (
            padded(r#"{"x":1}"#, MAX_PAYLOAD_LEN),
            padded(r#"{"x":2}"#, MAX_PAYLOAD_LEN),
        );
        assert!(restore_patch_for(&old_exact, &new_exact, &[]).is_some());
        let old_over = padded(r#"{"x":1}"#, MAX_PAYLOAD_LEN + 1);
        assert_eq!(restore_patch_for(&old_over, r#"{"x":2}"#, &[]), None);
        assert_eq!(restore_patch_for(r#"{"x":1}"#, &old_over, &[]), None);
        assert!(restore_patch_for(r#"{"x":1}"#, r#"{"x":2}"#, &[]).is_some());
    }

    #[test]
    fn unbounded_restore_patches_make_the_revision_non_restorable() {
        // Each side is under the payload bound, but the patch holds both.
        let old_json = serde_json::json!({ "body": "a".repeat(3 * 1024 * 1024) }).to_string();
        let new_json = serde_json::json!({ "body": "b".repeat(3 * 1024 * 1024) }).to_string();
        assert!(diff_payloads(&old_json, &new_json, &[]).is_some());
        assert_eq!(restore_patch_for(&old_json, &new_json, &[]), None);

        let flood: serde_json::Map<String, serde_json::Value> = (0..4_097)
            .map(|index| (format!("key-{index}"), serde_json::Value::from(index)))
            .collect();
        let flood = serde_json::json!({ "settings": flood }).to_string();
        assert_eq!(restore_patch_for(r#"{"settings":{}}"#, &flood, &[]), None);

        let (mut deep_old, mut deep_new) = (serde_json::json!(1), serde_json::json!(2));
        for _ in 0..70 {
            deep_old = serde_json::json!({ "nested": deep_old });
            deep_new = serde_json::json!({ "nested": deep_new });
        }
        let (deep_old, deep_new) = (deep_old.to_string(), deep_new.to_string());
        assert_eq!(restore_patch_for(&deep_old, &deep_new, &[]), None);

        assert!(restore_patch_for(r#"{"name":"a"}"#, r#"{"name":"b"}"#, &[]).is_some());
    }
}
