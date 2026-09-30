#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn test_extract_urls_zero_copy() {
    let html = r#"<p>Visit <a href="https://example.com/login">here</a> or http://test.org</p>"#;
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
            let Some(relative_end) = output.get(search_start..).and_then(|tail| tail.find(&end))
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

#[test]
fn security_errors_omit_the_unredacted_link() {
    for body in [
        "<a href=\"https://p\u{0430}ypal.com/reset?email=alice@example.com&password=hunter2\">x</a>",
        "<a href=\"javascript:fetch('/x?token=hunter2&to=alice@example.com')\">x</a>",
    ] {
        let error = scan_content_security(body).expect_err("unsafe link");
        let display = error.to_string();
        for fragment in ["hunter2", "alice@example.com", "ypal.com", "fetch("] {
            assert!(!display.contains(fragment), "error echoes link content");
        }
    }
}
