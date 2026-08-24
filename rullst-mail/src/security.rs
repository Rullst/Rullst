// src/security.rs — Outbound Phishing, Homograph URL Interceptor & Threat Scanner.

use crate::error::MailError;

/// Sanitizes all recognized credentials, AWS access keys, and private-key blocks.
pub fn redact_email_secrets(input: &str) -> String {
    let mut output = input.to_string();
    redact_values_after(&mut output, "bearer ", false);
    for key in ["password=", "secret=", "api_key=", "key=", "token="] {
        redact_values_after(&mut output, key, true);
    }
    redact_aws_access_keys(&mut output);
    redact_pem_blocks(&mut output, "PRIVATE KEY");
    redact_pem_blocks(&mut output, "RSA PRIVATE KEY");
    output
}

fn redact_values_after(output: &mut String, marker: &str, stop_at_ampersand: bool) {
    let mut offset = 0usize;
    loop {
        let lower = output.to_ascii_lowercase();
        let Some(relative) = lower.get(offset..).and_then(|tail| tail.find(marker)) else {
            break;
        };
        let start = offset + relative + marker.len();
        let Some(tail) = output.get(start..) else {
            break;
        };
        let end = tail
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(character, '"' | '\'' | ',' | '<')
                    || (stop_at_ampersand && character == '&')
            })
            .map_or(output.len(), |relative_end| start + relative_end);
        if end <= start {
            offset = start;
            continue;
        }
        if output.get(start..end) != Some("[REDACTED]") {
            output.replace_range(start..end, "[REDACTED]");
        }
        offset = start + "[REDACTED]".len();
    }
}

fn redact_aws_access_keys(output: &mut String) {
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

fn redact_pem_blocks(output: &mut String, label: &str) {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    while let Some(start) = output.find(&begin) {
        let search_start = start + begin.len();
        let Some(relative_end) = output.get(search_start..).and_then(|tail| tail.find(&end)) else {
            output.replace_range(start.., "[REDACTED PRIVATE KEY]");
            break;
        };
        let block_end = search_start + relative_end + end.len();
        output.replace_range(start..block_end, "[REDACTED PRIVATE KEY]");
    }
}

/// Scans a URL string for internationalized domain name (IDN) homograph spoofing attacks.
///
/// An IDN homograph attack occurs when an attacker registers a domain using visually identical
/// glyphs from mixed scripts (e.g. Cyrillic `а` / `U+0430` instead of Latin `a` / `U+0061`
/// in `pаypal.com`).
pub fn is_homograph_domain(domain: &str) -> bool {
    let mut has_latin = false;
    let mut has_cyrillic = false;
    let mut has_greek = false;

    for c in domain.chars() {
        if c == '.' || c == '-' || c.is_ascii_digit() {
            continue;
        }
        if c.is_ascii_alphabetic() {
            has_latin = true;
        } else if ('\u{0400}'..='\u{04FF}').contains(&c) {
            has_cyrillic = true;
        } else if ('\u{0370}'..='\u{03FF}').contains(&c) {
            has_greek = true;
        }
    }

    // Mixed scripts within the same domain label indicate a classic homograph attack
    let script_count = (has_latin as u8) + (has_cyrillic as u8) + (has_greek as u8);
    script_count > 1
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
pub fn extract_urls(content: &str) -> Vec<String> {
    let mut urls = Vec::new();
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
                let link = &rest[1..=end_quote];
                urls.push(link.to_string());
                pos = actual_idx + end_quote + 1;
                continue;
            }
            pos = actual_idx;
        } else {
            pos += 1;
        }
    }

    // 2. Extract plain https:// and http:// words
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
            if !clean.is_empty() && !urls.contains(&clean.to_string()) {
                urls.push(clean.to_string());
            }
        }
    }

    urls
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

        let domain = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("");

        if is_homograph_domain(domain) {
            return Err(MailError::SendError(format!(
                "Outbound mail security violation: Homograph domain spoofing attempt detected: '{}' (domain '{}')",
                url, domain
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_urls_zero_copy() {
        let html =
            r#"<p>Visit <a href="https://example.com/login">here</a> or http://test.org</p>"#;
        let urls = extract_urls(html);
        assert_eq!(urls, vec!["https://example.com/login", "http://test.org"]);
    }

    #[test]
    fn test_crlf_safety() {
        assert!(is_crlf_safe("Welcome to Rullst!"));
        assert!(!is_crlf_safe(
            "Welcome to Rullst!\r\nBcc: evil@attacker.com"
        ));
        assert!(!is_crlf_safe("Subject\nInjected-Header: 123"));
    }

    #[test]
    fn test_homograph_detection() {
        // Cyrillic 'а' in paypal
        assert!(is_homograph_domain("p\u{0430}ypal.com"));
        assert!(!is_homograph_domain("paypal.com"));
    }
}
