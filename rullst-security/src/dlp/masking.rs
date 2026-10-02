//! Linear-time secret masking passes used by [`super::mask_response_payload`].
//!
//! Every pass scans its input once and copies unchanged segments into a new
//! buffer, so neither the searches nor the replacements revisit the whole
//! document per match.

const PRIVATE_KEY_MARKER: &str = "[DLP_BLOCKED_PRIVATE_KEY]";
const AWS_ACCESS_KEY_MASK: &str = "AKIA****************";
const PASSWORD_MASK: &str = "*****";
const DATABASE_SCHEMES: [&str; 4] = ["postgres://", "postgresql://", "mysql://", "redis://"];

/// Longest URL authority (`user:password@host:port`) inspected after a scheme.
///
/// Credentials must be RFC 3986 percent-encoded. The authority also ends at
/// the first `/`, `?`, `#`, whitespace, quote, angle bracket, backtick or
/// control character, so a raw delimiter inside a password ends masking there.
pub(super) const MAX_DSN_AUTHORITY_BYTES: usize = 2_048;

/// Private-key PEM blocks: PKCS#8 (plain and encrypted), PKCS#1 RSA, SEC 1
/// EC, DSA, OpenSSH and OpenPGP. Each block is masked from its begin marker
/// through the first end marker with the same label.
const PRIVATE_KEY_BLOCKS: [(&str, &str); 7] = [
    ("-----BEGIN PRIVATE KEY-----", "-----END PRIVATE KEY-----"),
    (
        "-----BEGIN ENCRYPTED PRIVATE KEY-----",
        "-----END ENCRYPTED PRIVATE KEY-----",
    ),
    (
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----END RSA PRIVATE KEY-----",
    ),
    (
        "-----BEGIN EC PRIVATE KEY-----",
        "-----END EC PRIVATE KEY-----",
    ),
    (
        "-----BEGIN DSA PRIVATE KEY-----",
        "-----END DSA PRIVATE KEY-----",
    ),
    (
        "-----BEGIN OPENSSH PRIVATE KEY-----",
        "-----END OPENSSH PRIVATE KEY-----",
    ),
    (
        "-----BEGIN PGP PRIVATE KEY BLOCK-----",
        "-----END PGP PRIVATE KEY BLOCK-----",
    ),
];

/// Applies every masking pass in order. Returns `None` when nothing matched.
pub(super) fn mask_text(text: &str) -> Option<String> {
    let mut current: Option<String> = None;
    for (begin, end) in PRIVATE_KEY_BLOCKS {
        let input = current.as_deref().unwrap_or(text);
        if let Some(masked) = mask_private_keys(input, begin, end) {
            current = Some(masked);
        }
    }
    let input = current.as_deref().unwrap_or(text);
    if let Some(masked) = mask_aws_access_keys(input) {
        current = Some(masked);
    }
    for scheme in DATABASE_SCHEMES {
        let input = current.as_deref().unwrap_or(text);
        if let Some(masked) = mask_database_passwords(input, scheme) {
            current = Some(masked);
        }
    }
    current
}

/// Incrementally built copy of `source` that is allocated on first change.
///
/// Callers replace non-overlapping ranges in increasing order; unchanged
/// segments are copied once, so a pass stays linear however many ranges it
/// replaces.
pub(crate) struct SegmentRewriter<'a> {
    source: &'a str,
    output: Option<String>,
    copied: usize,
}

impl<'a> SegmentRewriter<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            source,
            output: None,
            copied: 0,
        }
    }

    /// Replaces `source[start..end]`; `start` must not precede a prior `end`.
    pub(crate) fn replace(&mut self, start: usize, end: usize, replacement: &str) {
        let output = self
            .output
            .get_or_insert_with(|| String::with_capacity(self.source.len()));
        output.push_str(&self.source[self.copied..start]);
        output.push_str(replacement);
        self.copied = end;
    }

    /// Returns the rewritten text, or `None` when nothing was replaced.
    pub(crate) fn finish(self) -> Option<String> {
        let mut output = self.output?;
        output.push_str(&self.source[self.copied..]);
        Some(output)
    }
}

fn mask_private_keys(text: &str, begin: &str, end_marker: &str) -> Option<String> {
    let mut rewriter = SegmentRewriter::new(text);
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find(begin) {
        let start = cursor + offset;
        let search_from = start + begin.len();
        let Some(end_offset) = text[search_from..].find(end_marker) else {
            break;
        };
        let end = search_from + end_offset + end_marker.len();
        rewriter.replace(start, end, PRIVATE_KEY_MARKER);
        cursor = end;
    }
    rewriter.finish()
}

fn mask_aws_access_keys(text: &str) -> Option<String> {
    let mut rewriter = SegmentRewriter::new(text);
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find("AKIA") {
        let start = cursor + offset;
        let is_access_key = text.as_bytes().get(start..start + 20).is_some_and(|key| {
            key.iter()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        });
        if is_access_key {
            rewriter.replace(start, start + 20, AWS_ACCESS_KEY_MASK);
            cursor = start + 20;
        } else {
            cursor = start + 4;
        }
    }
    rewriter.finish()
}

fn mask_database_passwords(text: &str, scheme: &str) -> Option<String> {
    let mut rewriter = SegmentRewriter::new(text);
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find(scheme) {
        let authority_start = cursor + offset + scheme.len();
        let authority_end = authority_end(text, authority_start);
        let authority = &text[authority_start..authority_end];
        // The host cannot contain `@`, so the last one ends the userinfo.
        let Some(at_idx) = authority.rfind('@') else {
            cursor = authority_end;
            continue;
        };
        let Some(colon_idx) = authority[..at_idx].find(':') else {
            cursor = authority_end;
            continue;
        };
        let pass_start = authority_start + colon_idx + 1;
        let pass_end = authority_start + at_idx;
        if &text[pass_start..pass_end] != PASSWORD_MASK {
            rewriter.replace(pass_start, pass_end, PASSWORD_MASK);
        }
        // Continue immediately after the `@`.
        cursor = pass_end + 1;
    }
    rewriter.finish()
}

/// Returns the byte offset where a URL authority beginning at `start` ends.
fn authority_end(text: &str, start: usize) -> usize {
    for (offset, character) in text[start..].char_indices() {
        if offset >= MAX_DSN_AUTHORITY_BYTES || is_authority_delimiter(character) {
            return start + offset;
        }
    }
    text.len()
}

fn is_authority_delimiter(character: char) -> bool {
    character.is_whitespace()
        || character.is_control()
        || matches!(character, '/' | '?' | '#' | '"' | '\'' | '<' | '>' | '`')
}

#[cfg(test)]
mod tests {
    use super::super::mask_response_payload;
    use super::*;
    use std::time::{Duration, Instant};

    /// Linear passes finish each 2 MiB input in about 0.1 s in a debug build;
    /// the previous per-match rescans and in-place replacements took seconds
    /// to tens of seconds.
    const LARGE_INPUT_BUDGET: Duration = Duration::from_secs(2);

    fn assert_fast(label: &str, input: &str) -> (Vec<u8>, bool) {
        let started = Instant::now();
        let result = mask_response_payload(input.as_bytes());
        let elapsed = started.elapsed();
        assert!(
            elapsed < LARGE_INPUT_BUDGET,
            "{label} took {elapsed:?} for {} bytes",
            input.len()
        );
        result
    }

    #[test]
    fn repeated_schemes_without_credentials_are_scanned_in_linear_time() {
        let input = "redis://".repeat(262_144);
        let (masked, was_modified) = assert_fast("schemes without '@'", &input);
        assert!(!was_modified);
        assert_eq!(masked, input.as_bytes());

        // A distant '@' must not be treated as the end of any userinfo.
        let input = format!("{}user:tail@host", "postgres://x:y ".repeat(139_810));
        let (masked, was_modified) = assert_fast("distant '@'", &input);
        assert!(!was_modified);
        assert_eq!(masked, input.as_bytes());
    }

    #[test]
    fn many_replacements_are_built_in_linear_time() {
        let credentials = "redis://a:bb@h ".repeat(139_810);
        let (masked, was_modified) = assert_fast("database passwords", &credentials);
        assert!(was_modified);
        let masked = String::from_utf8(masked).unwrap();
        assert_eq!(masked.matches("redis://a:*****@h ").count(), 139_810);

        let keys = "-----BEGIN PRIVATE KEY-----k-----END PRIVATE KEY-----".repeat(39_000);
        let (masked, was_modified) = assert_fast("private keys", &keys);
        assert!(was_modified);
        let masked = String::from_utf8(masked).unwrap();
        assert_eq!(masked, PRIVATE_KEY_MARKER.repeat(39_000));
    }

    #[test]
    fn every_supported_private_key_block_label_is_masked() {
        for label in [
            "PRIVATE KEY",
            "RSA PRIVATE KEY",
            "EC PRIVATE KEY",
            "DSA PRIVATE KEY",
            "ENCRYPTED PRIVATE KEY",
            "OPENSSH PRIVATE KEY",
            "PGP PRIVATE KEY BLOCK",
        ] {
            let input = format!(
                "config: -----BEGIN {label}-----\nMHcCAQEEIFixture\n-----END {label}----- done"
            );
            assert_eq!(
                mask_text(&input).as_deref(),
                Some("config: [DLP_BLOCKED_PRIVATE_KEY] done"),
                "a supported PEM private-key label was not masked"
            );
        }
        // A block needs its own end marker; a mismatched label is not masked.
        let mismatched = "-----BEGIN EC PRIVATE KEY-----x-----END DSA PRIVATE KEY-----";
        assert_eq!(mask_text(mismatched), None);
        // Public material is not a private key.
        let public = "-----BEGIN PUBLIC KEY-----x-----END PUBLIC KEY-----";
        assert_eq!(mask_text(public), None);
    }

    #[test]
    fn authority_is_bounded_by_delimiters_and_length() {
        assert_eq!(
            mask_text("postgres://svc:p%2Fss@db/app?x=user:no@pe").as_deref(),
            Some("postgres://svc:*****@db/app?x=user:no@pe")
        );
        // The last '@' in the authority ends the userinfo.
        assert_eq!(
            mask_text("redis://user:p@ss@cache:6379").as_deref(),
            Some("redis://user:*****@cache:6379")
        );
        for unmasked in [
            "see redis://cache then mail ops:x@example.com",
            "\"redis://cache\",\"u:p@h\"",
            "<redis://cache><u:p@h>",
            "redis://cache#frag:u@h",
        ] {
            assert_eq!(mask_text(unmasked), None, "{unmasked}");
        }
        let too_long = format!("redis://u:{}@h", "p".repeat(MAX_DSN_AUTHORITY_BYTES));
        assert_eq!(mask_text(&too_long), None);
        let longest = format!("redis://u:{}@h", "p".repeat(MAX_DSN_AUTHORITY_BYTES - 4));
        assert_eq!(mask_text(&longest).as_deref(), Some("redis://u:*****@h"));
    }
}
