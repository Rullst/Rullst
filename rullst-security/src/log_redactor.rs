//! Bounded secret-pattern redaction helper for application-owned log pipelines.

use crate::{dlp::mask_response_payload, telemetry::SecurityStore};

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
        redacted |= redact_assignment_values(&mut result, key);
    }
    redacted |= redact_bearer_tokens(&mut result);

    if redacted {
        SecurityStore::global().inc_log_redactions();
    }
    result
}

fn redact_bearer_tokens(value: &mut String) -> bool {
    let mut changed = false;
    let mut cursor = 0;
    loop {
        let lower = value.to_ascii_lowercase();
        let Some(offset) = lower[cursor..].find("bearer ") else {
            break;
        };
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
        if &value[start..end] == REDACTION_MARKER {
            cursor = end;
            continue;
        }
        value.replace_range(start..end, "[REDACTED]");
        changed = true;
        cursor = start + "[REDACTED]".len();
    }
    changed
}

fn redact_assignment_values(value: &mut String, key: &str) -> bool {
    let mut changed = false;
    let mut cursor = 0;
    loop {
        let lower = value.to_ascii_lowercase();
        let Some(offset) = lower[cursor..].find(key) else {
            break;
        };
        let key_start = cursor + offset;
        let key_end = key_start + key.len();
        let boundary_before = key_start == 0
            || !lower.as_bytes()[key_start - 1].is_ascii_alphanumeric()
                && lower.as_bytes()[key_start - 1] != b'_';
        if !boundary_before {
            cursor = key_end;
            continue;
        }

        let bytes = value.as_bytes();
        let mut separator = key_end;
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
            cursor = key_end;
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
            let lower_value = value[start..].to_ascii_lowercase();
            if lower_value.starts_with("bearer ") {
                start += "bearer ".len();
            } else if lower_value.starts_with("basic ") {
                start += "basic ".len();
            }
            while value
                .as_bytes()
                .get(start)
                .is_some_and(u8::is_ascii_whitespace)
            {
                start += 1;
            }
        }
        let end = secret_value_end(value, start, quote);
        if end == start || &value[start..end] == "[REDACTED]" {
            cursor = end.max(key_end);
            continue;
        }
        value.replace_range(start..end, "[REDACTED]");
        changed = true;
        cursor = start + "[REDACTED]".len();
    }
    changed
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
            redact_secrets("Authorization: Bearer secret_jwt_token_123, fallback Bearer 12345");
        assert_eq!(
            clean,
            "Authorization: Bearer [REDACTED], fallback Bearer [REDACTED]"
        );
        assert_eq!(
            redact_secrets("Authorization: Bearer    hidden, fallback Bearer   other"),
            "Authorization: Bearer    [REDACTED], fallback Bearer   [REDACTED]"
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
    fn unrelated_text_and_empty_input_are_preserved() {
        assert_eq!(redact_secrets("ordinary event"), "ordinary event");
        assert_eq!(redact_secrets(""), "");
        assert_eq!(redact_secrets("compassword=value"), "compassword=value");
    }
}
