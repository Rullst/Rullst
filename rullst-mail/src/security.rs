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
pub fn is_dangerous_scheme(url: &str) -> bool {
    let trimmed = url.trim().to_lowercase();
    trimmed.starts_with("javascript:")
        || trimmed.starts_with("vbscript:")
        || trimmed.starts_with("data:text/html")
        || trimmed.starts_with("file:")
}

/// Extracts all URL links (`href="..."` and plain `http://` / `https://` occurrences) from HTML/text.
///
/// Plain-text URLs are deduplicated through a hash set, so extraction stays
/// linear in the content length even when it contains many links.
pub fn extract_urls(content: &str) -> Vec<String> {
    let mut urls: Vec<&str> = Vec::new();
    let bytes = content.as_bytes();

    // 1. Extract href="..." occurrences case-insensitively without full heap clone
    let mut pos = 0;
    while pos + 5 <= bytes.len() {
        if bytes[pos..].starts_with(b"href=")
            || bytes[pos..].starts_with(b"HREF=")
            || bytes[pos..].starts_with(b"Href=")
        {
            let actual_idx = pos + 5;
            let rest = &content[actual_idx..];
            if let Some(quote_char) = rest.chars().next()
                && (quote_char == '"' || quote_char == '\'')
                && let Some(end_quote) = rest[1..].find(quote_char)
            {
                urls.push(&rest[1..=end_quote]);
                pos = actual_idx + end_quote + 1;
                continue;
            }
            pos = actual_idx;
        } else {
            pos += 1;
        }
    }

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
pub fn scan_content_security(content: &str) -> Result<(), MailError> {
    let urls = extract_urls(content);
    for url in urls {
        if is_dangerous_scheme(&url) {
            return Err(MailError::SendError(format!(
                "Outbound mail security violation: Dangerous URI scheme detected in link: '{}'",
                url
            )));
        }

        if let Some(domain) = homograph::homograph_link_host(&url) {
            return Err(MailError::SendError(format!(
                "Outbound mail security violation: Homograph domain spoofing attempt detected: '{}' (domain '{}')",
                url, domain
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
