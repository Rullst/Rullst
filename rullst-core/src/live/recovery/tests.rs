#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::http::{HeaderMap, HeaderValue, header};
use std::time::Duration;

fn command(fields: &str) -> String {
    format!(
        r#"{{"version":1,"kind":"action","id":"0123456789abcdef0123456789abcdef","revision":"7","action":"answer","fields":{fields}}}"#
    )
}

#[test]
fn command_boundary_rejects_ambiguous_fields_and_noncanonical_revisions() {
    let valid = command(r#"{"answer":"private-answer"}"#);
    let decoded = LiveCommand::decode(&valid).unwrap();
    assert_eq!(decoded.action(), "answer");
    assert_eq!(decoded.expected_revision(), 7);
    assert_eq!(decoded.fields().get("answer").unwrap(), "private-answer");
    assert!(!format!("{decoded:?}").contains("private-answer"));
    assert!(!format!("{decoded:?}").contains(decoded.id()));
    for invalid in [
        command(r#"{"answer":"first","answer":"second"}"#),
        command(r#"{"answer":7}"#),
        valid.replace("\"7\"", "\"07\""),
        valid.replace("\"7\"", "\"18446744073709551616\""),
        valid.replace("\"7\"", "7"),
        valid.replace("\"action\"", "\"unsupported\""),
        valid.replace("\"version\":1", "\"version\":2"),
        valid.replace("0123456789abcdef", "ABCDEF0123456789"),
        valid.replace("\"version\":1", "\"extra\":1,\"version\":1"),
        command(
            &serde_json::to_string(&std::collections::BTreeMap::from([(
                "answer",
                "x".repeat(2049),
            )]))
            .unwrap(),
        ),
        "x".repeat(16385),
    ] {
        assert!(matches!(
            LiveCommand::decode(&invalid),
            Err(LiveRecoveryError::Invalid)
        ));
    }
}

#[test]
fn scopes_and_snapshots_do_not_expose_private_values_in_debug() {
    let tenant = crate::security::TenantMembership::try_new(["school-a"])
        .unwrap()
        .select("school-a")
        .unwrap();
    let scope = LiveScope::try_new(&tenant, "private-account", "exam/7").unwrap();
    assert_eq!(scope.tenant(), "school-a");
    assert_eq!(scope.account(), "private-account");
    assert_eq!(scope.component(), "exam/7");
    assert!(!format!("{scope:?}").contains("private-account"));
    let snapshot = LiveSnapshot::try_new(u64::MAX, "<p>private-state</p>").unwrap();
    assert_eq!(snapshot.revision(), u64::MAX);
    assert!(!format!("{snapshot:?}").contains("private-state"));
    assert!(LiveSnapshot::try_new(0, "x".repeat(65537)).is_err());
    assert!(LiveScope::try_new(&tenant, "", "view").is_err());
    assert!(LiveScope::try_new(&tenant, "account", "\nview").is_err());
}

#[test]
fn exact_origin_subprotocol_and_limits_cannot_be_implicitly_weakened() {
    for origin in [
        "http://example.com",
        "https://user@example.com",
        "https://example.com/",
        "https://example.com?mode=1",
        "https://example.com#fragment",
        "null",
        "*",
        "https://example.com:0",
    ] {
        assert!(LiveRecoveryConfig::try_new(origin).is_err());
    }
    assert!(LiveRecoveryConfig::loopback_for_tests("http://localhost:3000").is_err());
    assert!(LiveRecoveryConfig::loopback_for_tests("http://127.0.0.1:3000").is_ok());
    let config = LiveRecoveryConfig::try_new("https://academy.example").unwrap();
    let mut headers = HeaderMap::new();
    assert!(!config.permits(&headers));
    headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("https://academy.example"),
    );
    headers.insert(
        header::SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_static("rullst.live.v1"),
    );
    assert!(config.permits(&headers));
    headers.append(
        header::ORIGIN,
        HeaderValue::from_static("https://evil.example"),
    );
    assert!(!config.permits(&headers));
    headers.remove(header::ORIGIN);
    headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("https://academy.example"),
    );
    headers.insert(
        header::SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_static("rullst.live.v1,other"),
    );
    assert!(!config.permits(&headers));
    assert!(config.clone().with_capacity(0, 1).is_err());
    assert!(config.clone().with_capacity(1, 1025).is_err());
    assert!(
        config
            .with_timing(
                Duration::ZERO,
                Duration::from_secs(1),
                Duration::from_secs(1)
            )
            .is_err()
    );
}
