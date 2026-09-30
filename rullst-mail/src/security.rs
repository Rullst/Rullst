// src/security.rs — Outbound Phishing, Homograph URL Interceptor & Threat Scanner.

use crate::error::MailError;
use std::collections::HashSet;

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

    /// Deterministic xorshift sequence for reproducible differential inputs.
    fn tokens(seed: &mut u64, alphabet: &[&str], count: usize) -> String {
        let mut output = String::new();
        for _ in 0..count {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            output.push_str(alphabet[(*seed % alphabet.len() as u64) as usize]);
        }
        output
    }

    /// The previous quadratic redactor, kept only as a behavioural oracle.
    fn legacy_redact(input: &str) -> String {
        fn values_after(output: &mut String, marker: &str, stop_at_ampersand: bool) {
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
        fn pem(output: &mut String, label: &str) {
            let begin = format!("-----BEGIN {label}-----");
            let end = format!("-----END {label}-----");
            while let Some(start) = output.find(&begin) {
                let search_start = start + begin.len();
                let Some(relative_end) =
                    output.get(search_start..).and_then(|tail| tail.find(&end))
                else {
                    output.replace_range(start.., "[REDACTED PRIVATE KEY]");
                    break;
                };
                output.replace_range(
                    start..search_start + relative_end + end.len(),
                    "[REDACTED PRIVATE KEY]",
                );
            }
        }
        let mut output = input.to_string();
        values_after(&mut output, "bearer ", false);
        for key in ["password=", "secret=", "api_key=", "key=", "token="] {
            values_after(&mut output, key, true);
        }
        redact_aws_access_keys(&mut output);
        pem(&mut output, "PRIVATE KEY");
        pem(&mut output, "RSA PRIVATE KEY");
        output
    }

    /// The previous extractor with linear-search deduplication.
    fn legacy_extract(content: &str) -> Vec<String> {
        let mut urls: Vec<String> = Vec::new();
        let bytes = content.as_bytes();
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
                    urls.push(rest[1..=end_quote].to_string());
                    pos = actual_idx + end_quote + 1;
                    continue;
                }
                pos = actual_idx;
            } else {
                pos += 1;
            }
        }
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
                    .find(['<', '>', '"', '\'', ')', '(', ']', '['])
                    .unwrap_or(candidate.len());
                let clean = &candidate[..end_idx];
                if !clean.is_empty() && !urls.contains(&clean.to_string()) {
                    urls.push(clean.to_string());
                }
            }
        }
        urls
    }

    #[test]
    fn linear_redaction_matches_the_previous_redactor() {
        let alphabet = [
            "key=",
            "KEY=",
            "Key=",
            "password=",
            "token=",
            "secret=",
            "api_key=",
            "Bearer ",
            "bearer ",
            "abc",
            " ",
            "&",
            "\"",
            "'",
            ",",
            "<",
            "\n",
            "AKIA",
            "AKIAABCDEFGHIJKLMNOP",
            "-----BEGIN PRIVATE KEY-----",
            "-----END PRIVATE KEY-----",
            "-----BEGIN RSA PRIVATE KEY-----",
            "-----END RSA PRIVATE KEY-----",
            "[REDACTED]",
            "é",
            "x",
            "=",
        ];
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        for round in 0..20_000 {
            let input = tokens(&mut seed, &alphabet, round % 14);
            assert_eq!(
                redact_email_secrets(&input),
                legacy_redact(&input),
                "{input:?}"
            );
        }
    }

    #[test]
    fn hash_set_deduplication_matches_the_previous_extractor() {
        let alphabet = [
            "href=\"",
            "href='",
            "HREF=\"",
            "Href='",
            "\"",
            "'",
            "http://a",
            "https://b",
            "HTTP://C",
            "HTTPS://d",
            " ",
            "(",
            ")",
            "<",
            ">",
            "[",
            "]",
            "x",
            "/",
            "\n",
            "é",
        ];
        let mut seed = 0x2545_f491_4f6c_dd1d_u64;
        for round in 0..20_000 {
            let input = tokens(&mut seed, &alphabet, round % 16);
            assert_eq!(extract_urls(&input), legacy_extract(&input), "{input:?}");
        }
        let distinct: String = (0..20_000)
            .map(|index| format!("http://h{index} "))
            .collect();
        assert_eq!(extract_urls(&distinct).len(), 20_000);
    }

    #[test]
    fn test_homograph_detection() {
        // Cyrillic 'а' in paypal
        assert!(is_homograph_domain("p\u{0430}ypal.com"));
        assert!(!is_homograph_domain("paypal.com"));
    }
}
