//! Bounded secret-pattern redaction helper for application-owned log pipelines.

use crate::{
    dlp::{SegmentRewriter, mask_response_payload},
    telemetry::SecurityStore,
};

const MAX_LOG_RECORD_BYTES: usize = 64 * 1024;
const REDACTION_MARKER: &str = "[REDACTED]";

/// Sanitizes common sensitive values in a single textual log record.
///
/// Applications must call this helper before emitting untrusted values or wrap
/// their tracing formatter. This function is not installed globally by merely
/// depending on `rullst-security`.
/// Records above 64 KiB are replaced in full before pattern processing.
pub fn redact_secrets(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    if input.len() > MAX_LOG_RECORD_BYTES {
        SecurityStore::global().inc_log_redactions();
        return "[REDACTED_OVERSIZED_LOG_RECORD]".to_string();
    }

    let (dlp_masked, dlp_modified) = mask_response_payload(input.as_bytes());
    let mut result = String::from_utf8(dlp_masked)
        .unwrap_or_else(|_| "[REDACTED_INVALID_LOG_RECORD]".to_string());
    let mut redacted = dlp_modified;
    for key in [
        "password",
        "passwd",
        "secret",
        "api_key",
        "token",
        "authorization",
        "cookie",
        "session",
    ] {
        if let Some(next) = redact_assignment_values(&result, key) {
            result = next;
            redacted = true;
        }
    }
    if let Some(next) = redact_bearer_tokens(&result) {
        result = next;
        redacted = true;
    }

    if redacted {
        SecurityStore::global().inc_log_redactions();
    }
    result
}

// Each pass lowercases the record once. ASCII lowercasing preserves byte
// offsets, so `lower` indexes the unmodified `value`; replacements are written
// to a separate buffer, keeping the pass linear in the record length.
fn redact_bearer_tokens(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let mut rewriter = SegmentRewriter::new(value);
    let mut cursor = 0;
    while let Some(offset) = lower[cursor..].find("bearer ") {
        let mut start = cursor + offset + "bearer ".len();
        while value
            .as_bytes()
            .get(start)
            .is_some_and(u8::is_ascii_whitespace)
        {
            start += 1;
        }
        let end = secret_value_end(value, start, None);
        if end == start {
            cursor = start;
            continue;
        }
        if &value[start..end] != REDACTION_MARKER {
            rewriter.replace(start, end, REDACTION_MARKER);
        }
        cursor = end;
    }
    rewriter.finish()
}

/// Final identifier components that name a secret when they follow a key
/// word, as in `SECRET_KEY`, `aws_secret_access_key` or `session_id`.
const SECRET_NAME_SUFFIXES: [&str; 2] = ["key", "id"];

/// Lowercases ASCII and maps `-` to `_`, so `X-API-Key` matches `api_key`.
/// Both mappings are ASCII-to-ASCII and therefore preserve byte offsets.
fn normalized_key_text(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '-' => '_',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

/// Scans the compound-identifier continuation that starts at `from`: further
/// alphanumeric components joined by `_` or `.`. Returns the identifier end
/// and the start of its last continuation component, if there is one.
fn identifier_continuation(lower: &[u8], from: usize) -> (usize, Option<usize>) {
    let mut end = from;
    let mut last_component = None;
    while lower
        .get(end)
        .is_some_and(|byte| matches!(byte, b'_' | b'.'))
        && lower.get(end + 1).is_some_and(u8::is_ascii_alphanumeric)
    {
        end += 1;
        last_component = Some(end);
        while lower.get(end).is_some_and(u8::is_ascii_alphanumeric) {
            end += 1;
        }
    }
    (end, last_component)
}

fn redact_assignment_values(value: &str, key: &str) -> Option<String> {
    let lower = normalized_key_text(value);
    let lower_bytes = lower.as_bytes();
    let bytes = value.as_bytes();
    let mut rewriter = SegmentRewriter::new(value);
    let mut cursor = 0;
    while let Some(offset) = lower[cursor..].find(key) {
        let key_start = cursor + offset;
        let key_end = key_start + key.len();
        // The key must be a whole component of a compound identifier such as
        // `DB_PASSWORD`, `client-secret` or `app.db.password`.
        let whole_component = (key_start == 0
            || !lower_bytes[key_start - 1].is_ascii_alphanumeric())
            && !lower_bytes
                .get(key_end)
                .is_some_and(u8::is_ascii_alphanumeric);
        if !whole_component {
            cursor = key_end;
            continue;
        }
        // Every occurrence inside one identifier shares its last component,
        // so a rejected identifier is skipped as a whole, keeping the pass
        // linear.
        let (identifier_end, last_component) = identifier_continuation(lower_bytes, key_end);
        if last_component.is_some_and(|start| {
            let last = &lower[start..identifier_end];
            last != key && !SECRET_NAME_SUFFIXES.contains(&last)
        }) {
            cursor = identifier_end;
            continue;
        }

        let mut separator = identifier_end;
        if bytes
            .get(separator)
            .is_some_and(|byte| matches!(byte, b'"' | b'\''))
        {
            separator += 1;
        }
        while bytes
            .get(separator)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            separator += 1;
        }
        if !bytes
            .get(separator)
            .is_some_and(|byte| matches!(byte, b'=' | b':'))
        {
            cursor = identifier_end;
            continue;
        }
        separator += 1;
        while bytes
            .get(separator)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            separator += 1;
        }
        let quote = bytes
            .get(separator)
            .copied()
            .filter(|byte| matches!(byte, b'"' | b'\''));
        let mut start = separator + usize::from(quote.is_some());
        if key == "authorization" {
            start = skip_authorization_scheme(bytes, start);
        }
        let end = if quote.is_none() && matches!(key, "authorization" | "cookie") {
            header_value_end(value, start)
        } else {
            secret_value_end(value, start, quote)
        };
        if end == start || &value[start..end] == REDACTION_MARKER {
            cursor = end.max(identifier_end);
            continue;
        }
        rewriter.replace(start, end, REDACTION_MARKER);
        cursor = end;
    }
    rewriter.finish()
}

/// Authentication schemes kept in front of a redacted credential: the IANA
/// HTTP Authentication Scheme Registry plus common unregistered schemes.
const AUTHORIZATION_SCHEMES: [&str; 17] = [
    "aws4-hmac-sha256",
    "apikey",
    "basic",
    "bearer",
    "concealed",
    "digest",
    "dpop",
    "gnap",
    "hoba",
    "mutual",
    "negotiate",
    "ntlm",
    "oauth",
    "privatetoken",
    "scram-sha-1",
    "scram-sha-256",
    "token",
];

/// Returns where the credential starts: after a recognized scheme and the
/// spaces or tabs that follow it, or at `start` when the value begins with
/// anything else, so an unrecognized word is redacted with the credential.
fn skip_authorization_scheme(bytes: &[u8], start: usize) -> usize {
    let tail = bytes.get(start..).unwrap_or_default();
    let scheme_len = tail
        .iter()
        .take_while(|byte| byte.is_ascii_alphanumeric() || **byte == b'-')
        .count();
    let scheme = &tail[..scheme_len];
    if !AUTHORIZATION_SCHEMES
        .iter()
        .any(|known| scheme.eq_ignore_ascii_case(known.as_bytes()))
    {
        return start;
    }
    let spaces = tail[scheme_len..]
        .iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    if spaces == 0 {
        return start;
    }
    start + scheme_len + spaces
}

/// End of an unquoted `Authorization` or `Cookie` value. Such a value may
/// contain spaces (`Digest a="1", b="2"`, `a=1; sid=2`), so it runs to the end
/// of its line, excluding trailing spaces and tabs.
fn header_value_end(value: &str, start: usize) -> usize {
    let line_end = value[start..]
        .find(['\r', '\n'])
        .map_or(value.len(), |offset| start + offset);
    start + value[start..line_end].trim_end_matches([' ', '\t']).len()
}

fn secret_value_end(value: &str, start: usize, quote: Option<u8>) -> usize {
    if let Some(quote) = quote {
        let mut escaped = false;
        for (offset, byte) in value[start..].bytes().enumerate() {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == quote {
                return start + offset;
            }
        }
        return value.len();
    }
    // An existing marker is one token, including its closing bracket. Only
    // preserve it if no secret suffix follows it before the actual delimiter.
    let search_start = if value[start..].starts_with(REDACTION_MARKER) {
        start + REDACTION_MARKER.len()
    } else {
        start
    };
    value[search_start..]
        .find(|character: char| {
            character.is_whitespace() || matches!(character, '"' | '\'' | ',' | '&' | '}' | ']')
        })
        .map_or(value.len(), |offset| search_start + offset)
}

#[cfg(test)]
mod tests {
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
        let clean =
            redact_secrets("Authorization: Bearer secret_jwt_token_123\nfallback Bearer 12345");
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
        let input =
            "password=first&secret=second JSON {\"password\":\"third\",\"token\": \"fourth\"}";
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
            redact_secrets(
                "authorization: Digest username=\"u\", response=\"6629fae4\"\nnext line"
            ),
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
}
