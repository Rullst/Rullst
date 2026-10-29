use super::{ClaimedOutboxEvent, same_payload, validate_key};

#[test]
fn replayed_payloads_compare_as_json_values() {
    assert!(same_payload(r#"{"b":2,"a":1}"#, r#"{"a":1,"b":2}"#));
    assert!(same_payload(
        r#"{"outer":{"y":[1,2],"x":null}}"#,
        r#"{"outer":{"x":null,"y":[1,2]}}"#
    ));
    assert!(!same_payload(r#"{"a":1,"b":2}"#, r#"{"a":1,"b":3}"#));
    assert!(!same_payload(r#"{"list":[1,2]}"#, r#"{"list":[2,1]}"#));
    assert!(!same_payload("not json", r#"{"a":1}"#));
}

#[test]
fn the_outbox_migration_keeps_its_recorded_name() {
    use crate::schema::Migration;
    // Renaming it would make the runner apply the migration again.
    assert_eq!(
        super::OutboxMigration.name(),
        "m20260830_000001_create_rullst_outbox"
    );
}

#[test]
fn keys_hold_one_to_128_portable_bytes() {
    assert!(validate_key("stream", &"a".repeat(128)).is_ok());
    assert!(validate_key("stream", "tenant-1:orders.v2_x").is_ok());
    for invalid in [String::new(), "a".repeat(129), "tenant a".to_string()] {
        let error = validate_key("stream", &invalid).expect_err("invalid key");
        assert_eq!(
            error.to_string(),
            "Validation error: outbox stream must contain 1-128 ASCII letters, digits, '.', '-', '_' or ':'"
        );
    }
}

#[test]
fn claimed_events_debug_without_keys_or_payload() {
    let event = ClaimedOutboxEvent {
        id: 7,
        stream: "tenant-a".to_string(),
        event_key: "order-1".to_string(),
        event_kind: "order.created".to_string(),
        payload_json: r#"{"a":1}"#.to_string(),
        attempts: 2,
        claim_key: "claim-secret".to_string(),
        claim_expires_at_epoch: 99,
    };
    assert_eq!(
        format!("{event:?}"),
        "ClaimedOutboxEvent { id: 7, stream: \"[REDACTED]\", event_key: \"[REDACTED]\", event_kind: \"order.created\", payload_bytes: 7, attempts: 2, claim_key: \"[REDACTED]\", claim_expires_at_epoch: 99 }"
    );
}
