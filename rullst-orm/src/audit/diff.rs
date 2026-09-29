use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

/// Marker persisted in place of a value the audit trail must not contain.
///
/// Generated model projections emit it for `#[orm(encrypted)]` and
/// `#[orm(masked)]` fields. Revision restore treats it as non-restorable.
#[doc(hidden)]
pub const REDACTED_VALUE: &str = "***";

/// Compares two in-memory values of a redacted field without persisting them.
///
/// A value that cannot be serialized is reported as changed so the audit
/// trail records a redacted change instead of silently omitting it.
#[doc(hidden)]
pub fn redacted_value_changed<T: Serialize + ?Sized>(current: &T, previous: &T) -> bool {
    match (
        serde_json::to_value(current),
        serde_json::to_value(previous),
    ) {
        (Ok(current), Ok(previous)) => current != previous,
        _ => true,
    }
}

/// Adds `field: "***"` to both diff payloads for each changed redacted field.
pub(super) fn with_redacted_changes(
    payloads: (Option<String>, Option<String>),
    redacted_changes: &[&str],
) -> (Option<String>, Option<String>) {
    if redacted_changes.is_empty() {
        return payloads;
    }
    let mark = |payload: Option<String>| {
        let mut object = match payload.as_deref().map(serde_json::from_str::<Value>) {
            None => Map::new(),
            Some(Ok(Value::Object(object))) => object,
            // A non-object diff is already a bounded sentinel; keep it intact.
            Some(_) => return payload,
        };
        for field in redacted_changes {
            object.insert(
                (*field).to_string(),
                Value::String(REDACTED_VALUE.to_string()),
            );
        }
        serde_json::to_string(&Value::Object(object)).ok()
    };
    (mark(payloads.0), mark(payloads.1))
}

pub(super) fn is_sensitive(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    [
        "password",
        "passwd",
        "token",
        "secret",
        "senha",
        "api_key",
        "private_key",
        "credential",
        "cookie",
        "session",
        "cvv",
        "ssn",
        "credit_card",
        "auth_code",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

pub(super) fn mask_nested(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    let value = mask_if_sensitive(&key, value);
                    (key, value)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(mask_nested).collect()),
        other => other,
    }
}

pub(super) fn mask_if_sensitive(key: &str, value: Value) -> Value {
    if is_sensitive(key) {
        Value::String("***".to_string())
    } else {
        mask_nested(value)
    }
}

fn diff_values(old: &Value, new: &Value, sensitive: bool) -> Option<(Value, Value)> {
    if old == new {
        return None;
    }
    if sensitive {
        let redacted = Value::String("***".to_string());
        return Some((redacted.clone(), redacted));
    }

    match (old, new) {
        (Value::Object(old_object), Value::Object(new_object)) => {
            let keys: BTreeSet<&String> = old_object.keys().chain(new_object.keys()).collect();
            let mut old_diff = Map::new();
            let mut new_diff = Map::new();

            for key in keys {
                let changed = match (old_object.get(key), new_object.get(key)) {
                    (Some(old_value), Some(new_value)) => {
                        diff_values(old_value, new_value, is_sensitive(key))
                    }
                    (Some(old_value), None) => {
                        Some((mask_if_sensitive(key, old_value.clone()), Value::Null))
                    }
                    (None, Some(new_value)) => {
                        Some((Value::Null, mask_if_sensitive(key, new_value.clone())))
                    }
                    (None, None) => None,
                };

                if let Some((old_value, new_value)) = changed {
                    old_diff.insert(key.clone(), old_value);
                    new_diff.insert(key.clone(), new_value);
                }
            }

            Some((Value::Object(old_diff), Value::Object(new_diff)))
        }
        _ => Some((mask_nested(old.clone()), mask_nested(new.clone()))),
    }
}

/// Computes a recursively redacted JSON difference.
///
/// Arrays and primitive roots are retained when changed. Invalid JSON is represented by a
/// metadata-only sentinel so the audit event is not silently lost and raw malformed secrets are
/// never persisted.
#[cfg_attr(test, mutants::skip)]
pub fn compute_diff(old_json: &str, new_json: &str) -> (Option<String>, Option<String>) {
    if old_json == new_json {
        return (None, None);
    }

    let old_res: Result<Value, _> = serde_json::from_str(old_json);
    let new_res: Result<Value, _> = serde_json::from_str(new_json);

    if old_res.is_err() || new_res.is_err() {
        let old_val = match old_res {
            Ok(v) => mask_nested(v),
            Err(_) => serde_json::json!({
                "$audit_error": "invalid_json",
                "bytes": old_json.len(),
            }),
        };
        let new_val = match new_res {
            Ok(v) => mask_nested(v),
            Err(_) => serde_json::json!({
                "$audit_error": "invalid_json",
                "bytes": new_json.len(),
            }),
        };
        return (
            serde_json::to_string(&old_val).ok(),
            serde_json::to_string(&new_val).ok(),
        );
    }

    let Ok(old) = old_res else {
        return (None, None);
    };
    let Ok(new) = new_res else {
        return (None, None);
    };
    let Some((old_diff, new_diff)) = diff_values(&old, &new, false) else {
        return (None, None);
    };

    match (
        serde_json::to_string(&old_diff),
        serde_json::to_string(&new_diff),
    ) {
        (Ok(old), Ok(new)) => (Some(old), Some(new)),
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_secrets_are_redacted_in_objects_and_arrays() {
        let old = r#"{"profile":{"password":"before"},"sessions":[{"token":"one"}]}"#;
        let new = r#"{"profile":{"password":"after"},"sessions":[{"token":"two"}]}"#;
        let (old_diff, new_diff) = compute_diff(old, new);
        let old_diff = old_diff.expect("changed input has an old diff");
        let new_diff = new_diff.expect("changed input has a new diff");

        assert!(!old_diff.contains("before"));
        assert!(!old_diff.contains("one"));
        assert!(!new_diff.contains("after"));
        assert!(!new_diff.contains("two"));
        assert!(old_diff.contains("***"));
        assert!(new_diff.contains("***"));
    }

    #[test]
    fn redacted_changes_are_marked_on_both_sides() {
        let payloads = with_redacted_changes(
            compute_diff(r#"{"a":1,"cpf":"***"}"#, r#"{"a":2,"cpf":"***"}"#),
            &["cpf"],
        );
        assert_eq!(payloads.0.as_deref(), Some(r#"{"a":1,"cpf":"***"}"#));
        assert_eq!(payloads.1.as_deref(), Some(r#"{"a":2,"cpf":"***"}"#));

        let only_redacted = with_redacted_changes(compute_diff("{}", "{}"), &["cpf"]);
        assert_eq!(only_redacted.0.as_deref(), Some(r#"{"cpf":"***"}"#));
        assert_eq!(only_redacted.1.as_deref(), Some(r#"{"cpf":"***"}"#));
        assert!(redacted_value_changed("before", "after"));
        assert!(!redacted_value_changed("same", "same"));
    }

    #[test]
    fn invalid_and_non_object_json_changes_are_not_dropped() {
        let invalid = compute_diff("not-json-a", "not-json-b");
        assert!(
            invalid
                .0
                .as_deref()
                .is_some_and(|value| value.contains("invalid_json"))
        );
        assert!(
            invalid
                .1
                .as_deref()
                .is_some_and(|value| value.contains("invalid_json"))
        );

        assert_eq!(
            compute_diff("[1,2]", "[1,3]"),
            (Some("[1,2]".to_string()), Some("[1,3]".to_string()))
        );
        assert_eq!(
            compute_diff("true", "false"),
            (Some("true".to_string()), Some("false".to_string()))
        );
    }
}
