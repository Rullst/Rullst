use super::{context::current_metadata, diff};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

const PATCH_VERSION: u8 = 1;
const MAX_PATCH_DEPTH: usize = 64;
const MAX_PATCH_OPERATIONS: usize = 4_096;
const MAX_PATCH_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct ReversePatch {
    version: u8,
    operations: Vec<ReverseOperation>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReverseOperation {
    path: Vec<String>,
    before: ValueState,
    after: ValueState,
    restorable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValueState {
    present: bool,
    value: Option<Value>,
}

impl ValueState {
    fn absent() -> Self {
        Self {
            present: false,
            value: None,
        }
    }

    fn from_value(value: &Value) -> (Self, bool) {
        let value = diff::mask_nested(value.clone());
        let restorable = !contains_redaction(&value);
        (
            Self {
                present: true,
                value: Some(value),
            },
            restorable,
        )
    }

    fn matches(&self, value: Option<&Value>) -> bool {
        if !self.present {
            return value.is_none();
        }
        self.value.as_ref() == value
    }
}

pub(crate) fn build_reverse_patch(
    old_json: &str,
    new_json: &str,
) -> Result<Option<String>, crate::Error> {
    let old: Value = serde_json::from_str(old_json)?;
    let new: Value = serde_json::from_str(new_json)?;
    if !old.is_object() || !new.is_object() {
        return Err(crate::Error::Validation(
            "restorable audit values must be JSON objects".to_string(),
        ));
    }
    let mut operations = Vec::new();
    collect_operations(&old, &new, &mut Vec::new(), 0, &mut operations)?;
    if operations.is_empty() {
        return Ok(None);
    }
    let encoded = serde_json::to_string(&ReversePatch {
        version: PATCH_VERSION,
        operations,
    })?;
    if encoded.len() > MAX_PATCH_BYTES {
        return Err(crate::Error::Validation(
            "audit restore patch exceeds the bounded size".to_string(),
        ));
    }
    Ok(Some(encoded))
}

fn collect_operations(
    old: &Value,
    new: &Value,
    path: &mut Vec<String>,
    depth: usize,
    operations: &mut Vec<ReverseOperation>,
) -> Result<(), crate::Error> {
    if old == new {
        return Ok(());
    }
    if depth > MAX_PATCH_DEPTH || operations.len() >= MAX_PATCH_OPERATIONS {
        return Err(crate::Error::Validation(
            "audit restore patch exceeds its depth or operation bound".to_string(),
        ));
    }
    if let (Value::Object(old_object), Value::Object(new_object)) = (old, new) {
        let keys: BTreeSet<&String> = old_object.keys().chain(new_object.keys()).collect();
        for key in keys {
            path.push(key.clone());
            match (old_object.get(key), new_object.get(key)) {
                (Some(before), Some(after)) if before == after => {}
                (Some(_), Some(_)) if diff::is_sensitive(key) => {
                    operations.push(ReverseOperation {
                        path: path.clone(),
                        before: ValueState {
                            present: true,
                            value: None,
                        },
                        after: ValueState {
                            present: true,
                            value: None,
                        },
                        restorable: false,
                    });
                }
                (Some(before), Some(after)) => {
                    collect_operations(before, after, path, depth + 1, operations)?;
                }
                (Some(before), None) => {
                    let (before, restorable) = ValueState::from_value(before);
                    operations.push(ReverseOperation {
                        path: path.clone(),
                        before,
                        after: ValueState::absent(),
                        restorable: restorable && !diff::is_sensitive(key),
                    });
                }
                (None, Some(after)) => {
                    let (after, restorable) = ValueState::from_value(after);
                    operations.push(ReverseOperation {
                        path: path.clone(),
                        before: ValueState::absent(),
                        after,
                        restorable: restorable && !diff::is_sensitive(key),
                    });
                }
                (None, None) => {}
            }
            path.pop();
        }
        return Ok(());
    }
    let (before, before_restorable) = ValueState::from_value(old);
    let (after, after_restorable) = ValueState::from_value(new);
    operations.push(ReverseOperation {
        path: path.clone(),
        before,
        after,
        restorable: before_restorable && after_restorable,
    });
    Ok(())
}

/// Applies one bounded reverse patch after verifying the current value still
/// matches the revision's recorded post-state.
#[doc(hidden)]
pub fn apply_reverse_patch(mut current: Value, encoded: &str) -> Result<Value, crate::Error> {
    if encoded.is_empty() || encoded.len() > MAX_PATCH_BYTES {
        return Err(crate::Error::Validation(
            "audit restore patch is empty or oversized".to_string(),
        ));
    }
    let patch: ReversePatch = serde_json::from_str(encoded)?;
    if patch.version != PATCH_VERSION
        || patch.operations.is_empty()
        || patch.operations.len() > MAX_PATCH_OPERATIONS
    {
        return Err(crate::Error::Validation(
            "audit restore patch version or operation count is invalid".to_string(),
        ));
    }
    for operation in patch.operations {
        validate_operation(&operation)?;
        if !operation
            .after
            .matches(value_at_path(&current, &operation.path))
        {
            return Err(crate::Error::Validation(
                "audit revision is stale relative to the current model state".to_string(),
            ));
        }
        apply_state(&mut current, &operation.path, operation.before)?;
    }
    Ok(current)
}

fn validate_operation(operation: &ReverseOperation) -> Result<(), crate::Error> {
    if operation.path.is_empty()
        || operation.path.len() > MAX_PATCH_DEPTH
        || operation.path.iter().any(|segment| segment.is_empty())
        || !operation.restorable
    {
        return Err(crate::Error::Validation(
            "audit revision contains a redacted or invalid operation".to_string(),
        ));
    }
    Ok(())
}

fn value_at_path<'a>(root: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

fn apply_state(root: &mut Value, path: &[String], state: ValueState) -> Result<(), crate::Error> {
    let (last, parents) = path
        .split_last()
        .ok_or_else(|| crate::Error::Validation("audit restore path is empty".to_string()))?;
    let mut parent = root;
    for segment in parents {
        parent = parent
            .as_object_mut()
            .and_then(|object| object.get_mut(segment))
            .ok_or_else(|| {
                crate::Error::Validation("audit restore path is no longer valid".to_string())
            })?;
    }
    let object = parent.as_object_mut().ok_or_else(|| {
        crate::Error::Validation("audit restore parent is not an object".to_string())
    })?;
    if state.present {
        let value = state.value.ok_or_else(|| {
            crate::Error::Validation("audit restore value is missing".to_string())
        })?;
        object.insert(last.clone(), value);
    } else {
        object.remove(last);
    }
    Ok(())
}

fn contains_redaction(value: &Value) -> bool {
    match value {
        Value::String(value) => value == "***",
        Value::Array(values) => values.iter().any(contains_redaction),
        Value::Object(values) => values.values().any(contains_redaction),
        _ => false,
    }
}

/// Validated audit material used by generated revision restoration.
pub struct RestorableRevision {
    restore_patch: String,
}

impl RestorableRevision {
    /// Returns the bounded versioned reverse patch.
    pub fn restore_patch(&self) -> &str {
        &self.restore_patch
    }
}

/// Loads one exact update revision and enforces the active tenant boundary.
#[doc(hidden)]
pub async fn load_restorable_revision_with_tx(
    tx: &mut crate::db::Transaction<'_>,
    audit_id: i32,
    model_type: &str,
    model_id: i32,
) -> Result<RestorableRevision, crate::Error> {
    if audit_id <= 0 || model_id <= 0 {
        return Err(crate::Error::Validation(
            "audit and model IDs must be positive".to_string(),
        ));
    }
    let metadata = current_metadata()?;
    let sql = if crate::Orm::driver()? == "postgres" {
        "SELECT event, tenant_key, format_version, restore_patch FROM rullst_audits WHERE id = $1 AND model_type = $2 AND model_id = $3"
    } else {
        "SELECT event, tenant_key, format_version, restore_patch FROM rullst_audits WHERE id = ? AND model_type = ? AND model_id = ?"
    };
    let row: Option<(String, Option<String>, i32, Option<String>)> =
        sqlx::query_as(sqlx::AssertSqlSafe(sql))
            .bind(audit_id)
            .bind(model_type)
            .bind(model_id)
            .fetch_optional(&mut **tx)
            .await?;
    let (event, tenant_key, format_version, restore_patch) =
        row.ok_or(crate::Error::RecordNotFound)?;
    if tenant_key != metadata.tenant_key {
        return Err(crate::Error::Validation(
            "audit revision is outside the active tenant scope".to_string(),
        ));
    }
    if event != "updated" || format_version != 2 {
        return Err(crate::Error::Validation(
            "only version-2 update revisions can be restored".to_string(),
        ));
    }
    let restore_patch = restore_patch.ok_or_else(|| {
        crate::Error::Validation("audit revision has no restorable patch".to_string())
    })?;
    Ok(RestorableRevision { restore_patch })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reverse_patch_restores_nested_values_and_rejects_stale_state() {
        let before = json!({"name":"before","profile":{"locale":"en","theme":"dark"}});
        let after = json!({"name":"after","profile":{"locale":"pt","theme":"dark"}});
        let patch = build_reverse_patch(&before.to_string(), &after.to_string())
            .expect("build patch")
            .expect("changed values");
        assert_eq!(
            apply_reverse_patch(after.clone(), &patch).expect("restore"),
            before
        );
        let stale = json!({"name":"later","profile":{"locale":"pt","theme":"dark"}});
        assert!(apply_reverse_patch(stale, &patch).is_err());
    }

    #[test]
    fn reverse_patch_refuses_redacted_sensitive_changes() {
        let patch = build_reverse_patch(r#"{"password":"before"}"#, r#"{"password":"after"}"#)
            .expect("build patch")
            .expect("changed values");
        assert!(apply_reverse_patch(json!({"password":"after"}), &patch).is_err());
        assert!(!patch.contains(r#"\"before\""#));
        assert!(!patch.contains(r#"\"after\""#));
    }
}
