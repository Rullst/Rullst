// src/security.rs — Outbound Phishing, Homograph URL Interceptor & Threat Scanner.

use crate::error::MailError;
use std::collections::HashSet;

mod homograph;
pub use homograph::is_homograph_domain;

/// Sanitizes all recognized credentials, AWS access keys, and private-key blocks.
///
/// Every pass is one forward scan that builds its output incrementally, so the
/// cost grows linearly with the input instead of with `matches * length`.
pub fn redact_email_secrets(input: &str) -> String {
    let mut output = redact_values_after(input, "bearer ", false);
    for key in ["password=", "secret=", "api_key=", "key=", "token="] {
        output = redact_values_after(&output, key, true);
    }
    redact_aws_access_keys(&mut output);
    let output = redact_pem_blocks(&output, "PRIVATE KEY");
    redact_pem_blocks(&output, "RSA PRIVATE KEY")
}

fn redact_values_after(input: &str, marker: &str, stop_at_ampersand: bool) -> String {
    // ASCII lowercasing preserves every byte offset and character boundary.
    let lower = input.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut copied = 0usize;
    let mut offset = 0usize;
    while let Some(relative) = lower.get(offset..).and_then(|tail| tail.find(marker)) {
        let start = offset + relative + marker.len();
        let Some(tail) = input.get(start..) else {
            break;
        };
        let end = tail
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(character, '"' | '\'' | ',' | '<')
                    || (stop_at_ampersand && character == '&')
            })
            .map_or(input.len(), |relative_end| start + relative_end);
        if end > start {
            output.push_str(input.get(copied..start).unwrap_or_default());
            output.push_str("[REDACTED]");
            copied = end;
        }
        offset = end;
    }
    output.push_str(input.get(copied..).unwrap_or_default());
    output
}

fn redact_aws_access_keys(output: &mut String) {
    // Same-length replacement keeps this in-place pass linear.
    let mut offset = 0usize;
    while let Some(relative) = output.get(offset..).and_then(|tail| tail.find("AKIA")) {
        let start = offset + relative;
        let candidate_end = start.saturating_add(20);
        let valid = output
            .get(start..candidate_end)
            .is_some_and(|candidate| candidate.bytes().all(|byte| byte.is_ascii_alphanumeric()));
        if valid {
            output.replace_range(start..candidate_end, "AKIA****************");
            offset = start + 20;
        } else {
            offset = start + 4;
        }
    }
}

fn redact_pem_blocks(input: &str, label: &str) -> String {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let mut output = String::with_capacity(input.len());
    let mut remaining = input;
    while let Some(start) = remaining.find(&begin) {
        output.push_str(remaining.get(..start).unwrap_or_default());
        output.push_str("[REDACTED PRIVATE KEY]");
        let body = remaining.get(start + begin.len()..).unwrap_or_default();
        let Some(relative_end) = body.find(&end) else {
            // An unterminated block is redacted through the end of the input.
            return output;
        };
        remaining = body.get(relative_end + end.len()..).unwrap_or_default();
    }
    output.push_str(remaining);
    output
}

/// Checks if a link uses a forbidden or dangerous URI scheme (e.g. `javascript:`, `vbscript:`, `data:text/html`).
///
/// HTML character references are decoded, and tab/newline characters and
/// leading whitespace or control characters are ignored, as browsers do when
/// they resolve an `href`.
pub fn is_dangerous_scheme(url: &str) -> bool {
    has_dangerous_scheme(&crate::entities::decode(url))
}

/// Scheme check for an already decoded link.
fn has_dangerous_scheme(url: &str) -> bool {
    let scheme: String = url
        .trim_start_matches(|c: char| c <= ' ' || c.is_whitespace())
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .take("data:text/html".len())
        .flat_map(char::to_lowercase)
        .collect();
    scheme.starts_with("javascript:")
        || scheme.starts_with("vbscript:")
        || scheme.starts_with("data:text/html")
        || scheme.starts_with("file:")
}

/// Returns every `href` attribute value. The name is matched ASCII
/// case-insensitively, whitespace may surround `=`, and the value may be
/// double-quoted, single-quoted or unquoted, as HTML allows.
fn href_values(content: &str) -> Vec<&str> {
    let bytes = content.as_bytes();
    let skip_whitespace = |mut index: usize| {
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        index
    };
    let mut values = Vec::new();
    let mut pos = 0;
    while pos + 4 <= bytes.len() {
        if !bytes[pos..pos + 4].eq_ignore_ascii_case(b"href") {
            pos += 1;
            continue;
        }
        let equals = skip_whitespace(pos + 4);
        if bytes.get(equals) != Some(&b'=') {
            pos += 4;
            continue;
        }
        let start = skip_whitespace(equals + 1);
        match bytes.get(start) {
            Some(&quote @ (b'"' | b'\'')) => {
                let value_start = start + 1;
                match bytes[value_start..].iter().position(|byte| *byte == quote) {
                    Some(length) => {
                        values.push(&content[value_start..value_start + length]);
                        pos = value_start + length + 1;
                    }
                    // An unterminated value is skipped, as before.
                    None => pos = value_start,
                }
            }
            Some(_) => {
                let length = bytes[start..]
                    .iter()
                    .position(|byte| byte.is_ascii_whitespace() || *byte == b'>')
                    .unwrap_or(bytes.len() - start);
                if length > 0 {
                    values.push(&content[start..start + length]);
                }
                pos = start + length.max(1);
            }
            None => break,
        }
    }
    values
}

/// Extracts all URL links (`href` attribute values and plain `http://` / `https://` occurrences) from HTML/text.
///
/// Plain-text URLs are deduplicated through a hash set, so extraction stays
/// linear in the content length even when it contains many links.
pub fn extract_urls(content: &str) -> Vec<String> {
    // 1. `href` attribute values, however they are cased, spaced or quoted.
    let mut urls: Vec<&str> = href_values(content);

    // 2. Extract plain https:// and http:// words
    let mut seen: HashSet<&str> = urls.iter().copied().collect();
    for word in content.split_whitespace() {
        let trimmed = word.trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '(');
        let start_pos = if trimmed.starts_with("https://")
            || trimmed.starts_with("http://")
            || trimmed.starts_with("HTTPS://")
            || trimmed.starts_with("HTTP://")
        {
            Some(0)
        } else {
            trimmed
                .find("https://")
                .or_else(|| trimmed.find("http://"))
                .or_else(|| trimmed.find("HTTPS://"))
                .or_else(|| trimmed.find("HTTP://"))
        };

        if let Some(url_start) = start_pos {
            let candidate = &trimmed[url_start..];
            let end_idx = candidate
                .find(|c: char| {
                    c == '<'
                        || c == '>'
                        || c == '"'
                        || c == '\''
                        || c == ')'
                        || c == '('
                        || c == ']'
                        || c == '['
                })
                .unwrap_or(candidate.len());
            let clean = &candidate[..end_idx];
            if !clean.is_empty() && seen.insert(clean) {
                urls.push(clean);
            }
        }
    }

    urls.into_iter().map(str::to_string).collect()
}

/// Checks if an email header value (Subject, To, From, etc.) is safe from CRLF injection attacks (`\r` or `\n`).
pub fn is_crlf_safe(header_value: &str) -> bool {
    !header_value.contains('\r') && !header_value.contains('\n')
}

/// Validates that none of the links inside the given content are dangerous or homograph spoofing attempts.
///
/// The error names only the violated rule. The offending link is omitted
/// because this scan runs before secret redaction, so its query can still
/// carry tokens, addresses or credentials.
pub fn scan_content_security(content: &str) -> Result<(), MailError> {
    let urls = extract_urls(content);
    for url in urls {
        // Browsers decode references in attribute values before navigating.
        let url = crate::entities::decode(&url);
        if has_dangerous_scheme(&url) {
            return Err(MailError::SendError(
                "Outbound mail security violation: a link uses a dangerous URI scheme".to_string(),
            ));
        }

        if homograph::homograph_link_host(&url).is_some() {
            return Err(MailError::SendError(
                "Outbound mail security violation: a link host is a homograph spoofing attempt"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
