#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use serde_json::{Map, json};

fn operation(path: Value, before: Value, after: Value, restorable: bool) -> Value {
    json!({
        "path": path,
        "before": before,
        "after": after,
        "restorable": restorable
    })
}

fn state(present: bool, value: Option<Value>) -> Value {
    json!({ "present": present, "value": value })
}

fn encoded_patch(operations: Vec<Value>) -> String {
    json!({ "version": PATCH_VERSION, "operations": operations }).to_string()
}

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
fn reverse_patch_restores_additions_removals_arrays_and_scalars() {
    let before = json!({
        "kept": true,
        "removed": { "nested": [1, 2] },
        "changed": ["old"]
    });
    let after = json!({
        "kept": true,
        "added": { "nested": [3, 4] },
        "changed": ["new"]
    });
    let patch = build_reverse_patch(&before.to_string(), &after.to_string())
        .expect("build patch")
        .expect("changed values");
    assert_eq!(
        apply_reverse_patch(after, &patch).expect("restore additions and removals"),
        before
    );
    assert!(
        build_reverse_patch(r#"{"same":1}"#, r#"{"same":1}"#)
            .expect("unchanged objects")
            .is_none()
    );
}

#[test]
fn reverse_patch_refuses_redacted_sensitive_changes() {
    let patch = build_reverse_patch(r#"{"password":"before"}"#, r#"{"password":"after"}"#)
        .expect("build patch")
        .expect("changed values");
    assert!(apply_reverse_patch(json!({"password":"after"}), &patch).is_err());
    assert!(!patch.contains(r#"\"before\""#));
    assert!(!patch.contains(r#"\"after\""#));

    let nested_redaction = build_reverse_patch(r#"{}"#, r#"{"profile":{"tokens":["safe","***"]}}"#)
        .expect("build redacted patch")
        .expect("changed values");
    assert!(
        apply_reverse_patch(
            json!({"profile":{"tokens":["safe","***"]}}),
            &nested_redaction,
        )
        .is_err()
    );
}

#[test]
fn patch_building_rejects_non_objects_depth_and_operation_floods() {
    assert!(build_reverse_patch("[]", "{}").is_err());
    assert!(build_reverse_patch("{}", "null").is_err());

    let mut deeply_nested_before = json!("before");
    let mut deeply_nested_after = json!("after");
    for _ in 0..=MAX_PATCH_DEPTH {
        deeply_nested_before = json!({ "child": deeply_nested_before });
        deeply_nested_after = json!({ "child": deeply_nested_after });
    }
    assert!(
        build_reverse_patch(
            &deeply_nested_before.to_string(),
            &deeply_nested_after.to_string(),
        )
        .is_err()
    );

    let mut flood = Map::new();
    for index in 0..=MAX_PATCH_OPERATIONS {
        flood.insert(format!("key-{index}"), json!(index));
    }
    assert!(build_reverse_patch("{}", &Value::Object(flood).to_string()).is_err());
}

#[test]
fn patch_decoder_rejects_invalid_envelopes_and_operations() {
    assert!(apply_reverse_patch(json!({}), "").is_err());
    assert!(apply_reverse_patch(json!({}), "{").is_err());
    assert!(apply_reverse_patch(json!({}), &"x".repeat(MAX_PATCH_BYTES + 1)).is_err());
    assert!(apply_reverse_patch(json!({}), r#"{"version":2,"operations":[]}"#).is_err());
    assert!(apply_reverse_patch(json!({}), r#"{"version":1,"operations":[]}"#).is_err());

    let invalid_operations = [
        operation(json!([]), state(false, None), state(false, None), true),
        operation(json!([""]), state(false, None), state(false, None), true),
        operation(
            json!(["field"]),
            state(false, None),
            state(false, None),
            false,
        ),
        operation(
            Value::Array(
                (0..=MAX_PATCH_DEPTH)
                    .map(|index| json!(format!("segment-{index}")))
                    .collect(),
            ),
            state(false, None),
            state(false, None),
            true,
        ),
    ];
    for invalid in invalid_operations {
        assert!(apply_reverse_patch(json!({}), &encoded_patch(vec![invalid])).is_err());
    }
}

#[test]
fn patch_application_rejects_broken_paths_and_missing_restore_values() {
    let missing_parent = operation(
        json!(["missing", "field"]),
        state(true, Some(json!(1))),
        state(false, None),
        true,
    );
    assert!(apply_reverse_patch(json!({}), &encoded_patch(vec![missing_parent])).is_err());

    let scalar_parent = operation(
        json!(["parent", "field"]),
        state(true, Some(json!(1))),
        state(false, None),
        true,
    );
    assert!(
        apply_reverse_patch(json!({ "parent": 7 }), &encoded_patch(vec![scalar_parent]),).is_err()
    );

    let missing_value = operation(
        json!(["field"]),
        state(true, None),
        state(false, None),
        true,
    );
    assert!(apply_reverse_patch(json!({}), &encoded_patch(vec![missing_value])).is_err());
}
