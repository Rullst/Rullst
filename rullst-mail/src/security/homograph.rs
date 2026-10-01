//! Per-label IDN homograph heuristics for outbound links.
//!
//! Scripts are compared inside one DNS label, so a single-script Greek or
//! Cyrillic label under an ASCII TLD (`παράδειγμα.gr`, `пример.com`) is
//! legitimate, while `pаypal.com` (Cyrillic `а` inside a Latin label) is not.
//! Only the link host and user-info are inspected; path, query and fragment
//! text is ordinary content.

/// Label separators that IDNA (UTS #46) maps to U+002E FULL STOP.
fn is_label_separator(c: char) -> bool {
    matches!(c, '.' | '\u{3002}' | '\u{FF0E}' | '\u{FF61}')
}

fn is_cyrillic(c: char) -> bool {
    ('\u{0400}'..='\u{052F}').contains(&c)
}

fn is_greek(c: char) -> bool {
    ('\u{0370}'..='\u{03FF}').contains(&c) || ('\u{1F00}'..='\u{1FFF}').contains(&c)
}

fn script_count(label: &str) -> u8 {
    let (mut latin, mut cyrillic, mut greek) = (false, false, false);
    for c in label.chars() {
        latin |= c.is_ascii_alphabetic();
        cyrillic |= is_cyrillic(c);
        greek |= is_greek(c);
    }
    latin as u8 + cyrillic as u8 + greek as u8
}

/// Cyrillic letters that render like Latin letters (Chromium's IDN spoof set).
const CYRILLIC_LATIN_LOOKALIKES: &str = "асԁеһіјӏорԗԛѕԝхуъьҽпгѵѡ";

/// A whole-script spoof such as `аррӏе`: every letter is a Cyrillic lookalike.
fn is_latin_lookalike_label(label: &str) -> bool {
    let mut letters = label.chars().filter(|c| c.is_alphabetic()).peekable();
    letters.peek().is_some()
        && letters.all(|c| {
            let lower = c.to_lowercase().next().unwrap_or(c);
            CYRILLIC_LATIN_LOOKALIKES.contains(lower)
        })
}

/// Detects internationalized domain name (IDN) homograph spoofing in a host.
///
/// Each label is checked on its own. A label mixing Latin, Cyrillic or Greek
/// letters (e.g. Cyrillic `а` / `U+0430` instead of Latin `a` in
/// `pаypal.com`) is a spoof, and so is a label made only of Cyrillic letters
/// that look Latin (`аррӏе.com`) unless the top-level label is Cyrillic.
pub fn is_homograph_domain(domain: &str) -> bool {
    let labels = || domain.split(is_label_separator).filter(|l| !l.is_empty());
    if labels().any(|label| script_count(label) > 1) {
        return true;
    }
    let cyrillic_tld = labels()
        .next_back()
        .is_some_and(|tld| tld.chars().any(is_cyrillic));
    !cyrillic_tld && labels().any(is_latin_lookalike_label)
}

/// Returns `(user-info, host)` for a link with an authority, or the legacy
/// first segment (before `/`, `?`, `#` or `:`) for a reference without one.
fn link_host(url: &str) -> (&str, &str) {
    let url = url.trim_matches(|c: char| c <= ' ');
    let scheme_end = url.find("://").filter(|&end| {
        let scheme = &url[..end];
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    });
    let authority = match (scheme_end, url.strip_prefix("//")) {
        (Some(end), _) => &url[end + 3..],
        (None, Some(rest)) => rest,
        (None, None) => {
            let end = url.find(['/', '?', '#', ':']).unwrap_or(url.len());
            return ("", &url[..end]);
        }
    };
    let end = authority
        .find(['/', '?', '#', '\\'])
        .unwrap_or(authority.len());
    let authority = &authority[..end];
    let (user_info, host) = authority.rsplit_once('@').unwrap_or(("", authority));
    let host = if host.starts_with('[') {
        host.find(']').map_or(host, |end| &host[..=end])
    } else {
        host.split(':').next().unwrap_or(host)
    };
    (user_info, host)
}

/// A mail client resolves a relative reference against its page, typically
/// an HTTP(S) webmail URL, where `//host`, `\\host` and `/\host` name a host.
static RELATIVE_BASE: std::sync::LazyLock<Option<reqwest::Url>> =
    std::sync::LazyLock::new(|| reqwest::Url::parse("https://mail.invalid/").ok());

/// The `(user-info, host)` a browser navigates to, as the WHATWG URL parser
/// (the `url` crate behind `reqwest::Url`) resolves the link: it removes tabs
/// and newlines, treats `\` as `/` for special schemes, accepts
/// `https:host` and `https:\\host`, and percent-decodes the host. The host
/// is returned in Unicode form, so an A-label is inspected as its U-label.
fn resolved_authority(url: &str) -> Option<(String, String)> {
    let parsed = reqwest::Url::parse(url)
        .ok()
        .or_else(|| RELATIVE_BASE.as_ref()?.join(url).ok())?;
    let host = percent_decode(parsed.domain()?);
    let (host, _) = idna::domain_to_unicode(&host);
    let mut user_info = percent_decode(parsed.username());
    if let Some(password) = parsed.password() {
        user_info.push(':');
        user_info.push_str(&percent_decode(password));
    }
    Some((user_info, host))
}

/// Decodes `%XX` escapes; invalid UTF-8 becomes U+FFFD.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let escape = (byte == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .filter(|hex| hex.iter().all(u8::is_ascii_hexdigit))
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escape {
            Some(value) => {
                decoded.push(value);
                index += 3;
            }
            _ => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// Whether a decoded link's host or user-info is a homograph, both as the
/// text reads and as a browser resolves it.
pub(super) fn is_homograph_link(url: &str) -> bool {
    // Browsers remove every ASCII tab and newline before parsing a URL.
    let url: String = url
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let (user_info, host) = link_host(&url);
    is_homograph_domain(host)
        || is_homograph_domain(user_info)
        || resolved_authority(&url).is_some_and(|(user_info, host)| {
            is_homograph_domain(&host) || is_homograph_domain(&user_info)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::scan_content_security;

    #[test]
    fn scripts_are_compared_per_label() {
        assert!(is_homograph_domain("p\u{0430}ypal.com"));
        assert!(is_homograph_domain("\u{0501}ropbox.com"));
        assert!(is_homograph_domain("g\u{03BF}ogle.com"));
        assert!(is_homograph_domain(
            "\u{0430}\u{0440}\u{0440}\u{04CF}\u{0435}.com"
        ));
        assert!(!is_homograph_domain("paypal.com"));
        assert!(!is_homograph_domain("παράδειγμα.gr"));
        assert!(!is_homograph_domain("пример.com"));
        assert!(!is_homograph_domain("пример.рф"));
        assert!(!is_homograph_domain(
            "\u{0430}\u{0440}\u{0440}\u{04CF}\u{0435}.рф"
        ));
    }

    #[test]
    fn only_the_link_authority_is_inspected() {
        for legitimate in [
            "Visit https://παράδειγμα.gr",
            "<a href=\"https://пример.com\">пример</a>",
            "https://example.com?q=привет",
            "https://example.com#раздел",
            "https://example.com/каталог/товар",
            "<a href=\"/путь?q=привет\">x</a>",
        ] {
            assert!(scan_content_security(legitimate).is_ok(), "{legitimate}");
        }
        for spoof in [
            "https://p\u{0430}ypal.com",
            "HTTPS://p\u{0430}ypal.com/login",
            "https://p\u{0430}ypal.com:8443?next=/",
            "https://p\u{0430}ypal.com@example.com/",
            "<a href=\"//p\u{0430}ypal.com\">x</a>",
            "https://\u{0430}\u{0440}\u{0440}\u{04CF}\u{0435}.com/",
        ] {
            assert!(scan_content_security(spoof).is_err(), "{spoof}");
        }
        let message = crate::Message::new()
            .to("member@example.com")
            .subject("Ενημέρωση")
            .text("Visit https://παράδειγμα.gr and https://example.com?q=привет");
        assert!(message.validate_security().is_ok());
    }

    #[test]
    fn link_hosts_are_resolved_as_browsers_parse_them() {
        let anchor = |href: &str| format!("<a href=\"{href}\">Account</a>");
        for spoof in [
            // WHATWG URL parsing drops tabs and newlines anywhere.
            "https:&#9;//p&#x430;ypal.com/login",
            "https:&Tab;//p&#x430;ypal.com/",
            "https://p&#10;&#x430;ypal.com/",
            // Special schemes treat `\` as `/` and need no slashes at all.
            "https:\\\\p\u{0430}ypal.com/",
            "HTTP:\\\\p\u{0430}ypal.com",
            "https:/p\u{0430}ypal.com/",
            "https:p\u{0430}ypal.com",
            "https:\\\\p\u{0430}ypal.com@example.com/",
            // Scheme-relative references resolved against an HTTP(S) page.
            "\\\\p\u{0430}ypal.com/",
            "/\\p\u{0430}ypal.com/",
            // Hosts are percent-decoded, and an A-label is the same host.
            "https://p%D0%B0ypal.com/",
            "https://xn--pypal-4ve.com/",
        ] {
            let html = anchor(spoof);
            let message = crate::Message::new().to("a@example.com").html(html.clone());
            assert!(scan_content_security(&html).is_err(), "{spoof}");
            assert!(
                crate::DeliveryPipeline::prepare(&message).is_err(),
                "{spoof}"
            );
        }
        for legitimate in [
            "https:&#9;//пример.com/",
            "https://xn--e1afmkfd.xn--p1ai/",
            "https:\\\\example.com\\каталог?q=привет",
            "mailto:team@example.com",
            "/путь?q=привет",
        ] {
            assert!(
                scan_content_security(&anchor(legitimate)).is_ok(),
                "{legitimate}"
            );
        }
    }
}
