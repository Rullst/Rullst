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
mod tests;
