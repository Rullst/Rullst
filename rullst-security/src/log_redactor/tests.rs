use super::*;
use std::time::{Duration, Instant};

/// Linear passes need a few milliseconds for a maximum-size record in a
/// debug build; the previous per-match lowercasing of the whole record took
/// more than a second for each of these records.
const MAX_RECORD_BUDGET: Duration = Duration::from_millis(250);

fn redact_within_budget(record: &str) -> String {
    assert!(record.len() <= MAX_LOG_RECORD_BYTES);
    let started = Instant::now();
    let clean = redact_secrets(record);
    let elapsed = started.elapsed();
    assert!(
        elapsed < MAX_RECORD_BUDGET,
        "{} byte record took {elapsed:?}",
        record.len()
    );
    clean
}

#[test]
fn maximum_size_records_with_many_matches_are_redacted_in_linear_time() {
    let unassigned = "token ".repeat(MAX_LOG_RECORD_BYTES / 6);
    assert_eq!(redact_within_budget(&unassigned), unassigned);

    // Many key-word components in one identifier that does not name a
    // secret are rejected together, not rescanned per occurrence.
    let compound = format!("{}x=1", "token_".repeat(MAX_LOG_RECORD_BYTES / 6 - 1));
    assert_eq!(redact_within_budget(&compound), compound);

    let assignments = "token=a ".repeat(MAX_LOG_RECORD_BYTES / 8);
    assert_eq!(
        redact_within_budget(&assignments),
        "token=[REDACTED] ".repeat(MAX_LOG_RECORD_BYTES / 8)
    );

    let headers = "Authorization=Bearer x\n".repeat(MAX_LOG_RECORD_BYTES / 23);
    assert_eq!(
        redact_within_budget(&headers),
        "Authorization=Bearer [REDACTED]\n".repeat(MAX_LOG_RECORD_BYTES / 23)
    );
}

#[test]
fn quoted_secrets_with_escaped_quotes_are_completely_redacted() {
    let json = r#"{"password":"first\"still-secret","event":"ok"}"#;
    assert_eq!(
        redact_secrets(json),
        r#"{"password":"[REDACTED]","event":"ok"}"#
    );
    assert_eq!(
        redact_secrets(r#"secret='first\'still-secret' event=ok"#),
        "secret='[REDACTED]' event=ok"
    );
    assert_eq!(
        redact_secrets(r#"{"password":"trailing-backslash\\","event":"ok"}"#),
        r#"{"password":"[REDACTED]","event":"ok"}"#
    );
    assert_eq!(
        redact_secrets(r#"{"authorization":"Bearer first\"still-secret"}"#),
        r#"{"authorization":"Bearer [REDACTED]"}"#
    );
}

#[test]
fn oversized_records_fail_closed_before_secret_pattern_processing() {
    let input = format!("{} password=must-not-be-emitted", "x".repeat(64 * 1024));
    assert!(redact_secrets(&input) == "[REDACTED_OVERSIZED_LOG_RECORD]");
}

#[test]
fn redaction_markers_cannot_hide_a_secret_suffix_and_repeated_redaction_is_stable() {
    let input = r#"{"password":"[REDACTED]still-secret"} token=[REDACTED]suffix"#;
    assert_eq!(
        redact_secrets(input),
        r#"{"password":"[REDACTED]"} token=[REDACTED]"#
    );
    let redacted = "Authorization: Bearer [REDACTED]";
    assert_eq!(redact_secrets(redacted), redacted);
}

#[test]
fn bearer_tokens_are_fully_redacted_including_short_and_repeated_values() {
    let clean = redact_secrets("Authorization: Bearer secret_jwt_token_123\nfallback Bearer 12345");
    assert_eq!(
        clean,
        "Authorization: Bearer [REDACTED]\nfallback Bearer [REDACTED]"
    );
    assert_eq!(
        redact_secrets("Authorization: Bearer    hidden\nfallback Bearer   other, next"),
        "Authorization: Bearer    [REDACTED]\nfallback Bearer   [REDACTED], next"
    );
    assert_eq!(
        redact_secrets("Authorization: Basic   c2VjcmV0OnZhbHVl"),
        "Authorization: Basic   [REDACTED]"
    );
}

#[test]
fn query_json_and_repeated_assignments_are_redacted() {
    let input = "password=first&secret=second JSON {\"password\":\"third\",\"token\": \"fourth\"}";
    let clean = redact_secrets(input);
    assert!(!clean.contains("first"));
    assert!(!clean.contains("second"));
    assert!(!clean.contains("third"));
    assert!(!clean.contains("fourth"));
    assert_eq!(clean.matches("[REDACTED]").count(), 4);
}

#[test]
fn pem_aws_and_database_credentials_share_the_dlp_boundary() {
    let input = "-----BEGIN PRIVATE KEY-----abc-----END PRIVATE KEY----- AKIA1234567890123456 postgres://user:password@db/app";
    let clean = redact_secrets(input);
    assert!(clean.contains("[DLP_BLOCKED_PRIVATE_KEY]"));
    assert!(clean.contains("AKIA****************"));
    assert!(clean.contains("postgres://user:*****@db/app"));
    assert!(!clean.contains("password@"));
}

#[test]
fn compound_secret_key_names_are_redacted() {
    assert_eq!(
        redact_secrets("DB_PASSWORD=hunter2 access_token=eyJhbGciOi.x.y"),
        "DB_PASSWORD=[REDACTED] access_token=[REDACTED]"
    );
    assert_eq!(
        redact_secrets(r#"{"client_secret":"s3cr3t","refresh_token":"r1"}"#),
        r#"{"client_secret":"[REDACTED]","refresh_token":"[REDACTED]"}"#
    );
    assert_eq!(
        redact_secrets("SECRET_KEY=abc secret_key: def aws_secret_access_key=ghi"),
        "SECRET_KEY=[REDACTED] secret_key: [REDACTED] aws_secret_access_key=[REDACTED]"
    );
    assert_eq!(
        redact_secrets("/cb?id_token=a.b.c&state=ok session_id=s1 X-API-Key: k1"),
        "/cb?id_token=[REDACTED]&state=ok session_id=[REDACTED] X-API-Key: [REDACTED]"
    );
    assert_eq!(
        redact_secrets("app.db.password=pw csrf-token=t1"),
        "app.db.password=[REDACTED] csrf-token=[REDACTED]"
    );
    // A key word inside a longer identifier that names something else is
    // not a secret assignment.
    for unchanged in [
        "session_count=5",
        "max_tokens=100",
        "token_budget=7",
        "cookie_consent=yes",
        "compassword=value",
    ] {
        assert_eq!(redact_secrets(unchanged), unchanged);
    }
}

#[test]
fn unquoted_authorization_and_cookie_values_are_redacted_to_the_end_of_the_line() {
    assert_eq!(
        redact_secrets("Authorization: Token ghp_16C7e42F292c6912E7710c838347Ae178B4a"),
        "Authorization: Token [REDACTED]"
    );
    assert_eq!(
        redact_secrets("authorization: Digest username=\"u\", response=\"6629fae4\"\nnext line"),
        "authorization: Digest [REDACTED]\nnext line"
    );
    assert_eq!(
        redact_secrets("Proxy-Authorization: opaque-credential with spaces\r\nHost: x"),
        "Proxy-Authorization: [REDACTED]\r\nHost: x"
    );
    assert_eq!(
        redact_secrets("Cookie: theme=dark; sid=s3cr3t\nUser-Agent: test"),
        "Cookie: [REDACTED]\nUser-Agent: test"
    );
    assert_eq!(
        redact_secrets("Set-Cookie: sid=abc; Path=/; HttpOnly"),
        "Set-Cookie: [REDACTED]"
    );
    assert_eq!(
        redact_secrets(r#"{"cookie":"a=1; sid=2","authorization":"Token t"}"#),
        r#"{"cookie":"[REDACTED]","authorization":"Token [REDACTED]"}"#
    );
    let redacted = "Authorization: Token [REDACTED]\nCookie: [REDACTED]";
    assert_eq!(redact_secrets(redacted), redacted);
}

#[test]
fn unrelated_text_and_empty_input_are_preserved() {
    assert_eq!(redact_secrets("ordinary event"), "ordinary event");
    assert_eq!(redact_secrets(""), "");
    assert_eq!(redact_secrets("compassword=value"), "compassword=value");
}
