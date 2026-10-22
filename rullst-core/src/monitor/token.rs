//! Monitor tokens: validation at start-up and constant-time matching.

use super::{MAX_MONITOR_TOKENS, MonitorError};
use axum::http::{HeaderMap, header};
use subtle::ConstantTimeEq;

/// Base64 or base64url characters of 32 bytes, without padding.
const MIN_BASE64_CHARS: usize = 43;
/// Hexadecimal characters of 32 bytes.
const MIN_HEX_CHARS: usize = 64;
/// Longest accepted token, configured or presented.
const MAX_TOKEN_CHARS: usize = 512;
/// Fewest distinct characters accepted. A random token of the minimum length
/// has about 15 (hexadecimal) or 31 (base64) of them.
const MIN_DISTINCT_CHARS: usize = 10;
/// Fragments of documentation placeholders, compared without case.
const PLACEHOLDERS: [&str; 9] = [
    "changeme",
    "change_me",
    "change-me",
    "replace",
    "example",
    "placeholder",
    "yourtoken",
    "your_token",
    "your-token",
];

/// Checks that `token` (at `position` in the configured list, from 1) is a
/// random value of at least 32 bytes. Errors never contain the token.
pub(super) fn validate(token: &str, position: usize) -> Result<(), MonitorError> {
    let significant = token.trim_end_matches('=');
    let padding = token.len() - significant.len();
    if token.len() > MAX_TOKEN_CHARS || padding > 2 || !significant.bytes().all(is_token_byte) {
        return Err(MonitorError::InvalidToken { position });
    }
    let hexadecimal = significant.bytes().all(|byte| byte.is_ascii_hexdigit());
    let minimum = if hexadecimal {
        MIN_HEX_CHARS
    } else {
        MIN_BASE64_CHARS
    };
    if significant.len() < minimum {
        return Err(MonitorError::WeakToken { position });
    }
    let lowercase = significant.to_ascii_lowercase();
    if distinct_bytes(significant) < MIN_DISTINCT_CHARS
        || PLACEHOLDERS
            .iter()
            .any(|placeholder| lowercase.contains(placeholder))
    {
        return Err(MonitorError::PlaceholderToken { position });
    }
    Ok(())
}

/// Base64, base64url and hexadecimal characters, padding excluded.
fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'-' | b'_')
}

fn distinct_bytes(value: &str) -> usize {
    let mut seen = [false; 256];
    for byte in value.bytes() {
        seen[usize::from(byte)] = true;
    }
    seen.iter().filter(|seen| **seen).count()
}

/// The accepted tokens, kept only as SHA-256 digests.
pub(super) struct TokenSet {
    digests: Vec<ring::digest::Digest>,
}

impl TokenSet {
    /// Validates every token; the first invalid one is the error.
    pub(super) fn new(tokens: &[String]) -> Result<Self, MonitorError> {
        if tokens.len() > MAX_MONITOR_TOKENS {
            return Err(MonitorError::TooManyTokens {
                maximum: MAX_MONITOR_TOKENS,
            });
        }
        for (index, token) in tokens.iter().enumerate() {
            validate(token, index + 1)?;
        }
        Ok(Self {
            digests: tokens.iter().map(|token| digest(token)).collect(),
        })
    }

    pub(super) fn is_empty(&self) -> bool {
        self.digests.is_empty()
    }

    /// Whether `presented` is one of the tokens. Every digest is compared in
    /// constant time and the comparison does not stop at a match, so the
    /// timing reveals neither the token nor its position in the list.
    pub(super) fn matches(&self, presented: &str) -> bool {
        let presented = digest(presented);
        let matched = self
            .digests
            .iter()
            .fold(subtle::Choice::from(0), |matched, expected| {
                matched | expected.as_ref().ct_eq(presented.as_ref())
            });
        bool::from(matched)
    }
}

fn digest(token: &str) -> ring::digest::Digest {
    ring::digest::digest(&ring::digest::SHA256, token.as_bytes())
}

/// The credential a request presents in `Authorization`.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Presented<'a> {
    /// No `Authorization` header.
    Absent,
    /// One header with a scheme other than `Bearer`, which only the
    /// application authorizer may understand.
    OtherScheme,
    /// One well-formed `Bearer` token.
    Bearer(&'a str),
    /// Several `Authorization` headers, or a `Bearer` value that is empty,
    /// too long or not visible ASCII: a presented but invalid credential.
    Invalid,
}

pub(super) fn presented(headers: &HeaderMap) -> Presented<'_> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let Some(value) = values.next() else {
        return Presented::Absent;
    };
    if values.next().is_some() {
        return Presented::Invalid;
    }
    let Ok(value) = value.to_str() else {
        return Presented::Invalid;
    };
    let (scheme, token) = value.split_once(' ').unwrap_or((value, ""));
    if !scheme.eq_ignore_ascii_case("bearer") {
        return Presented::OtherScheme;
    }
    let token = token.trim_matches(' ');
    if token.is_empty()
        || token.len() > MAX_TOKEN_CHARS
        || !token.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Presented::Invalid;
    }
    Presented::Bearer(token)
}
