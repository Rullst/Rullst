// src/security.rs — Outbound Phishing, Homograph URL Interceptor & Threat Scanner.

use crate::drivers::MailError;

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
    let lower = content.to_lowercase();

    // 1. Extract href="..." occurrences
    let mut pos = 0;
    while let Some(href_idx) = lower[pos..].find("href=") {
        let actual_idx = pos + href_idx + 5;
        if actual_idx >= content.len() {
            break;
        }
        let rest = &content[actual_idx..];
        let quote_char = rest.chars().next().unwrap_or(' ');
        if (quote_char == '"' || quote_char == '\'')
            && let Some(end_quote) = rest[1..].find(quote_char)
        {
            let link = &rest[1..=end_quote];
            urls.push(link.to_string());
            pos = actual_idx + end_quote + 1;
            continue;
        }
        pos = actual_idx;
    }

    // 2. Extract plain https:// and http:// words
    for word in content.split_whitespace() {
        let clean = word.trim_matches(|c| {
            c == '"' || c == '\'' || c == '<' || c == '>' || c == '(' || c == ')'
        });
        if (clean.starts_with("http://") || clean.starts_with("https://"))
            && !urls.contains(&clean.to_string())
        {
            urls.push(clean.to_string());
        }
    }

    urls
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
