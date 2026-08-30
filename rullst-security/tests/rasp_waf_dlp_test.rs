// tests/rasp_waf_dlp_test.rs — Comprehensive RASP, WAF & DLP threat detection tests.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_security::dlp::mask_response_payload;
use rullst_security::log_redactor::redact_secrets;
use rullst_security::rasp::RaspInspector;

#[test]
fn test_rasp_sqli_detection() {
    // Classic SQLi
    assert!(RaspInspector::inspect_text("1' OR '1'='1"));
    assert!(RaspInspector::inspect_text("1; DROP TABLE users--"));
    assert!(RaspInspector::inspect_text(
        "UNION SELECT null, username, password FROM users"
    ));

    // Benign queries
    assert!(!RaspInspector::inspect_text("john_doe_99"));
    assert!(!RaspInspector::inspect_text(
        "search query with normal words"
    ));
    assert!(!RaspInspector::inspect_text("12345"));
}

#[test]
fn test_rasp_xss_and_command_injection() {
    // Command injection
    assert!(RaspInspector::inspect_text("; cat /etc/passwd"));
    assert!(RaspInspector::inspect_text("| sh"));
    assert!(RaspInspector::inspect_text("/bin/bash -c id"));

    // Path traversal
    assert!(RaspInspector::inspect_text("../../../etc/shadow"));
    assert!(RaspInspector::inspect_text(
        "..\\..\\windows\\system32\\cmd.exe"
    ));
    assert!(!RaspInspector::inspect_text("/static/images/logo.png"));
}

#[test]
fn test_dlp_payload_masking() {
    // 1. Private Key masking
    let payload_with_key = "-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQD3-----END PRIVATE KEY-----";
    let (masked_bytes, changed) = mask_response_payload(payload_with_key.as_bytes());
    assert!(changed);
    let masked_str = String::from_utf8_lossy(&masked_bytes);
    assert!(masked_str.contains("[DLP_BLOCKED_PRIVATE_KEY]"));

    // 2. AWS Key masking
    let payload_with_aws = r#"{"key":"AKIAIOSFODNN7EXAMPLE"}"#;
    let (masked_aws_bytes, changed_aws) = mask_response_payload(payload_with_aws.as_bytes());
    assert!(changed_aws);
    let masked_aws = String::from_utf8_lossy(&masked_aws_bytes);
    assert!(!masked_aws.contains("AKIAIOSFODNN7EXAMPLE"));

    // 3. Database URL masking
    let payload_with_db = "postgres://admin:SuperSecretPass@localhost:5432/production";
    let (masked_db_bytes, changed_db) = mask_response_payload(payload_with_db.as_bytes());
    assert!(changed_db);
    let masked_db = String::from_utf8_lossy(&masked_db_bytes);
    assert!(!masked_db.contains("SuperSecretPass"));
    assert!(masked_db.contains("*****"));
}

#[test]
fn test_log_redaction() {
    // 1. Password parameter
    let log_msg = "User login attempt with password=SuperSecret123 for user alice";
    let redacted = redact_secrets(log_msg);
    assert!(!redacted.contains("SuperSecret123"));
    assert!(redacted.contains("[REDACTED]"));

    // 2. Bearer token
    let bearer_log = "API request Authorization: Bearer eyJhbGciOiJIUzI1NiJ9_secret_token";
    let redacted_bearer = redact_secrets(bearer_log);
    assert!(!redacted_bearer.contains("eyJhbGciOiJIUzI1NiJ9_secret_token"));
    assert!(redacted_bearer.contains("Bearer [REDACTED]"));

    // 3. AWS Key
    let aws_log = "Uploaded to S3 using key AKIAIOSFODNN7EXAMPLE safely";
    let redacted_aws = redact_secrets(aws_log);
    assert!(!redacted_aws.contains("AKIAIOSFODNN7EXAMPLE"));
}
