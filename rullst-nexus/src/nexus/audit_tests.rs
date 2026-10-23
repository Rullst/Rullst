//! Validation, minimization and schemas of the Nexus mutation audit.

use super::*;

#[test]
fn audit_text_and_limits_fail_closed() {
    assert!(validate_audit_text("actor", "admin", 16).is_ok());
    assert!(validate_audit_text("actor", " admin", 16).is_err());
    assert!(validate_audit_text("actor", "a\n", 16).is_err());
    assert!(validate_audit_text("actor", "12345", 4).is_err());
}

#[test]
fn unrepresentable_record_keys_are_recorded_as_absent() {
    assert_eq!(auditable_record_key("ACME-1"), Some("ACME-1"));
    let long = "k".repeat(MAX_RECORD_KEY_BYTES + 1);
    for key in ["ACME ", " ACME", "line\nbreak", "", long.as_str()] {
        assert_eq!(auditable_record_key(key), None, "{key:?}");
    }
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

#[test]
fn audit_text_accepts_exactly_its_maximum_and_rejects_empty_or_control_text() {
    assert!(validate_audit_text("actor", "1234", 4).is_ok());
    assert!(validate_audit_text("actor", "", 4).is_err());
    assert!(validate_audit_text("actor", "a\u{1}b", 4).is_err());
    let error = validate_audit_text("tenant ID", "", 128).unwrap_err();
    assert_eq!(
        error.to_string(),
        rullst_orm::Error::Validation(
            "Nexus audit tenant ID must contain 1 to 128 unpadded bytes without controls"
                .to_string()
        )
        .to_string()
    );
    assert_eq!(
        auditable_record_key(&"k".repeat(256)).map(str::len),
        Some(256)
    );
    assert_eq!(auditable_record_key(&"k".repeat(257)), None);
}

#[test]
fn correlation_ids_come_only_from_a_valid_request_id_header() {
    let mut headers = axum::http::HeaderMap::new();
    assert_eq!(correlation_id(&headers), None);
    headers.insert("x-request-id", "req-42".parse().unwrap());
    assert_eq!(correlation_id(&headers).as_deref(), Some("req-42"));
    headers.insert("x-request-id", " req-42".parse().unwrap());
    assert_eq!(correlation_id(&headers), None);
    headers.insert("x-request-id", "r".repeat(256).parse().unwrap());
    assert_eq!(correlation_id(&headers), None);
}

#[test]
fn audit_time_is_the_current_unix_time_in_milliseconds() {
    let millis = |time: SystemTime| {
        i64::try_from(time.duration_since(UNIX_EPOCH).unwrap().as_millis()).unwrap()
    };
    let before = millis(SystemTime::now());
    let recorded = unix_epoch_millis().unwrap();
    let after = millis(SystemTime::now());
    assert!(
        (before..=after).contains(&recorded),
        "{before} {recorded} {after}"
    );
}

#[test]
fn each_driver_gets_its_own_key_column_syntax() {
    assert!(create_table_sql("postgres").contains("id BIGSERIAL PRIMARY KEY"));
    assert!(create_table_sql("mysql").contains("id BIGINT AUTO_INCREMENT PRIMARY KEY"));
    assert!(create_table_sql("sqlite").contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
}

#[test]
fn audit_records_debug_without_identities_or_keys() {
    let record = NexusAuditRecord {
        id: 7,
        actor_id: "ops-admin".to_string(),
        tenant_id: Some("school-a".to_string()),
        table_name: "courses".to_string(),
        action: "update".to_string(),
        record_key: Some("course-1".to_string()),
        record_count: 1,
        outcome: "committed".to_string(),
        correlation_id: None,
        occurred_at_ms: 1_700_000_000_000,
        format_version: 1,
    };
    assert_eq!(
        format!("{record:?}"),
        "NexusAuditRecord { id: 7, actor_id_bytes: 9, has_tenant_id: true, \
         table_name: \"courses\", action: \"update\", has_record_key: true, \
         record_count: 1, outcome: \"committed\", has_correlation_id: false, \
         occurred_at_ms: 1700000000000, format_version: 1 }"
    );
}
