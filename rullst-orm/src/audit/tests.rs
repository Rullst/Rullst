use super::AuditLog;

fn sample_log() -> AuditLog {
    AuditLog {
        id: 1,
        model_type: "User".to_string(),
        model_id: 42,
        event: "updated".to_string(),
        old_values: Some(r#"{"name":"Before"}"#.to_string()),
        new_values: Some(r#"{"name":"After"}"#.to_string()),
        actor_kind: "user".to_string(),
        actor_id: "actor-secret-marker".to_string(),
        tenant_key: Some("string:tenant-secret-marker".to_string()),
        correlation_id: Some("correlation-secret-marker".to_string()),
        reverted_audit_id: None,
        reason: Some("reason-secret-marker".to_string()),
        format_version: 2,
        restore_patch: Some("restore-secret-marker".to_string()),
        created_at: Some("2026-08-31T00:00:00Z".to_string()),
    }
}

#[test]
fn audit_log_serialization_round_trip_is_explicit() {
    let log = sample_log();
    let json = serde_json::to_string(&log).expect("serialize audit log");
    let decoded: AuditLog = serde_json::from_str(&json).expect("deserialize audit log");
    assert_eq!(decoded.id, 1);
    assert_eq!(decoded.model_id, 42);
    assert_eq!(decoded.actor_kind, "user");
    assert_eq!(decoded.actor_id, "actor-secret-marker");
}

#[test]
fn audit_log_debug_redacts_payload_and_identity_values() {
    let debug = format!("{:?}", sample_log());
    assert!(debug.contains("has_old_values: true"));
    assert!(debug.contains("has_correlation_id: true"));
    for secret in [
        "Before",
        "After",
        "actor-secret-marker",
        "tenant-secret-marker",
        "correlation-secret-marker",
        "reason-secret-marker",
        "restore-secret-marker",
    ] {
        assert!(!debug.contains(secret));
    }
}

#[test]
fn compute_diff_preserves_changes_and_redacts_secrets() {
    let old = r#"{"name":"Alice","password":"before","age":30}"#;
    let new = r#"{"name":"Alice","password":"after","age":31}"#;
    let (old_diff, new_diff) = super::compute_diff(old, new);
    let old_diff = old_diff.expect("old difference");
    let new_diff = new_diff.expect("new difference");
    assert!(old_diff.contains(r#""age":30"#));
    assert!(new_diff.contains(r#""age":31"#));
    assert!(old_diff.contains(r#""password":"***""#));
    assert!(new_diff.contains(r#""password":"***""#));
    assert!(!old_diff.contains("before"));
    assert!(!new_diff.contains("after"));
}

#[tokio::test]
async fn identical_diff_does_not_require_a_database_or_context() {
    let result = super::log_audit_diff(
        "User",
        1,
        "updated",
        r#"{"name":"Alice"}"#,
        r#"{"name":"Alice"}"#,
    )
    .await;
    assert!(result.is_ok());
}
