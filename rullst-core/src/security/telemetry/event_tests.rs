#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn version_one_json_contract_is_exact_and_legacy_input_defaults_to_v1() {
    let event = LiveSecurityEvent::local("RBAC_DENIAL", "denied", "192.0.2.4");
    let value = serde_json::to_value(&event).expect("serialize event");
    let object = value.as_object().expect("event object");
    let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "client_ip",
            "details",
            "event_type",
            "schema_version",
            "timestamp_str",
            "verified_hmac"
        ]
    );
    assert_eq!(value["schema_version"], 1);

    let legacy = serde_json::json!({
        "event_type": "RBAC_DENIAL",
        "details": "denied",
        "client_ip": "192.0.2.4",
        "timestamp_str": "2026-08-27T12:00:00Z",
        "verified_hmac": false
    });
    let decoded: LiveSecurityEvent =
        serde_json::from_value(legacy).expect("legacy event remains readable");
    assert_eq!(decoded.schema_version, 1);
}

#[test]
fn local_events_normalize_identity_type_timestamp_and_utf8_size() {
    let oversized = "á".repeat(MAX_EVENT_DETAILS_BYTES);
    let mut event = LiveSecurityEvent {
        schema_version: 900,
        event_type: "invalid type".to_string(),
        details: oversized,
        client_ip: "attacker-controlled".to_string(),
        timestamp_str: "moments ago".to_string(),
        verified_hmac: false,
    }
    .normalized();

    assert_eq!(event.schema_version, 1);
    assert_eq!(event.event_type, "SECURITY_EVENT");
    assert_eq!(event.client_ip, "unknown");
    assert!(event.details.len() <= 2048);
    assert!(event.details.is_char_boundary(event.details.len()));
    assert!(crate::rfc3339::parse_unix_seconds(&event.timestamp_str).is_some());

    event.event_type = "A".repeat(65);
    assert_eq!(event.normalized().event_type, "SECURITY_EVENT");
}

#[test]
fn normalization_keeps_a_valid_timestamp_ip_and_integrity_flag() {
    let event = LiveSecurityEvent {
        schema_version: 1,
        event_type: "RBAC_DENIAL".to_string(),
        details: "denied".to_string(),
        client_ip: "2001:db8::0:1".to_string(),
        timestamp_str: "2026-08-27T12:00:00Z".to_string(),
        verified_hmac: true,
    }
    .normalized();

    assert_eq!(event.timestamp_str, "2026-08-27T12:00:00Z");
    assert_eq!(event.client_ip, "2001:db8::1");
    assert!(
        event.verified_hmac,
        "normalization does not change integrity"
    );
}

#[test]
fn event_types_are_one_to_sixty_four_uppercase_digits_or_underscores() {
    let kept = |event_type: &str| {
        LiveSecurityEvent::local(event_type, "details", "unknown").event_type == event_type
    };

    assert!(kept(&"A".repeat(64)));
    assert!(kept("TOTP_2FA"));
    assert!(kept("_"));
    assert!(kept("9"));
    assert!(!kept(&"A".repeat(65)));
    assert!(!kept(""));
    assert!(!kept("Rbac_denial"));
    assert!(!kept("RBAC-DENIAL"));
    assert!(!kept("RBAC DENIAL"));
}

/// Runs `work` on its own thread so a non-terminating loop fails the test.
fn within_deadline<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("work finished before the deadline")
}

#[test]
fn details_keep_the_whole_budget_and_cut_before_a_split_character() {
    let limit = 2 * 1024;
    let exact = "a".repeat(limit);
    let event = LiveSecurityEvent::local("TEST", exact.clone(), "unknown");
    assert_eq!(event.details, exact);

    let over = "a".repeat(limit + 1);
    let event = LiveSecurityEvent::local("TEST", over, "unknown");
    assert_eq!(event.details, "a".repeat(limit));

    // The three-byte euro sign straddles the limit, so the cut falls before it.
    let split = format!("{}€tail", "a".repeat(limit - 1));
    let event = within_deadline(move || LiveSecurityEvent::local("TEST", split, "unknown"));
    assert_eq!(event.details, "a".repeat(limit - 1));
}
