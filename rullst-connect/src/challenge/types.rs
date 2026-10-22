//! Challenge identifiers, stored challenges and validated return paths.

use super::{ChallengeError, MAX_CHALLENGE_LIFETIME_SECONDS};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest as _, Sha256};
use std::fmt;
use subtle::ConstantTimeEq;

const ID_BYTES: usize = 32;
/// Unpadded base64url length of [`ID_BYTES`].
const ID_LENGTH: usize = 43;
const STORAGE_KEY_DOMAIN: &[u8] = b"rullst-connect:login-challenge:v1\0";
const MAX_PROVIDER_BYTES: usize = 32;
const MIN_SECRET_BYTES: usize = 32;
const MIN_VERIFIER_BYTES: usize = 43;
const MAX_SECRET_BYTES: usize = 128;
const MAX_RETURN_PATH_BYTES: usize = 1024;
/// Largest timestamp every SQL backend stores as a signed 64-bit integer.
const MAX_TIMESTAMP: u64 = i64::MAX as u64;
const REDACTED: &str = "[REDACTED]";

/// The opaque, single-use name of a stored challenge: 256 random bits as 43
/// unpadded base64url characters.
///
/// This is the only value the browser holds. Stores persist
/// [`Self::storage_key`], a domain-separated SHA-256 digest, never the id
/// itself, so a database read does not reveal a usable cookie value.
/// Equality is constant-time and `Debug` is redacted.
#[derive(Clone)]
pub struct ChallengeId(String);

impl ChallengeId {
    /// Generates a fresh id from the operating system's secure generator.
    #[must_use]
    pub fn generate() -> Self {
        let mut bytes = [0_u8; ID_BYTES];
        rand::fill(&mut bytes);
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    /// Accepts only the exact shape [`Self::generate`] produces.
    /// Every strict unpadded encoding of `ID_LENGTH` characters decodes to
    /// exactly `ID_BYTES` bytes, so the length and the decoding suffice.
    pub fn parse(value: &str) -> Result<Self, ChallengeError> {
        if value.len() == ID_LENGTH && URL_SAFE_NO_PAD.decode(value).is_ok() {
            Ok(Self(value.to_string()))
        } else {
            Err(ChallengeError::Invalid("challenge id"))
        }
    }

    /// The cookie value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The lowercase hex SHA-256 digest under which stores keep the challenge.
    #[must_use]
    pub fn storage_key(&self) -> String {
        let mut digest = Sha256::new();
        digest.update(STORAGE_KEY_DOMAIN);
        digest.update(self.0.as_bytes());
        digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

impl PartialEq for ChallengeId {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes().ct_eq(other.0.as_bytes()).into()
    }
}

impl Eq for ChallengeId {}

impl fmt::Debug for ChallengeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ChallengeId")
            .field(&REDACTED)
            .finish()
    }
}

/// A same-site page to open after login: an absolute path on this origin.
///
/// It starts with `/` but not `//` or `/\`, and contains only visible ASCII
/// without backslashes (at most 1024 bytes), so a browser cannot resolve it
/// to another host. Percent-encode anything else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnPath(String);

impl ReturnPath {
    /// Validates a return path; anything else is [`ChallengeError::Invalid`].
    pub fn parse(value: impl Into<String>) -> Result<Self, ChallengeError> {
        let value = value.into();
        let bytes = value.as_bytes();
        let relative = bytes.first() == Some(&b'/') && !matches!(bytes.get(1), Some(b'/' | b'\\'));
        let visible = bytes
            .iter()
            .all(|byte| byte.is_ascii_graphic() && *byte != b'\\');
        if value.len() > MAX_RETURN_PATH_BYTES || !relative || !visible {
            return Err(ChallengeError::Invalid("return path"));
        }
        Ok(Self(value))
    }

    /// The validated path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The server half of one authorization attempt.
///
/// `Debug` redacts the state, nonce and PKCE verifier.
#[derive(Clone, PartialEq, Eq)]
pub struct LoginChallenge {
    provider: String,
    state: String,
    nonce: Option<String>,
    code_verifier: String,
    return_to: Option<ReturnPath>,
    created_at: u64,
    expires_at: u64,
}

impl LoginChallenge {
    /// Builds a validated challenge; stores also use it to check rows they read.
    ///
    /// `provider` is 1 to 32 bytes of `a-z`, `0-9`, `_` or `-`. `state` and
    /// `nonce` are 32 to 128 and `code_verifier` 43 to 128 RFC 7636 unreserved
    /// characters. Times are Unix seconds; the lifetime is 1 to 600 seconds.
    pub fn new(
        provider: impl Into<String>,
        state: impl Into<String>,
        nonce: Option<String>,
        code_verifier: impl Into<String>,
        return_to: Option<ReturnPath>,
        created_at: u64,
        expires_at: u64,
    ) -> Result<Self, ChallengeError> {
        let provider = provider.into();
        let state = state.into();
        let code_verifier = code_verifier.into();
        if !valid_provider(&provider) {
            return Err(ChallengeError::Invalid("provider name"));
        }
        if !unreserved(&state, MIN_SECRET_BYTES) {
            return Err(ChallengeError::Invalid("state"));
        }
        if nonce
            .as_deref()
            .is_some_and(|nonce| !unreserved(nonce, MIN_SECRET_BYTES))
        {
            return Err(ChallengeError::Invalid("nonce"));
        }
        if !unreserved(&code_verifier, MIN_VERIFIER_BYTES) {
            return Err(ChallengeError::Invalid("code verifier"));
        }
        let lifetime = expires_at.saturating_sub(created_at);
        if !(1..=MAX_CHALLENGE_LIFETIME_SECONDS).contains(&lifetime) || expires_at > MAX_TIMESTAMP {
            return Err(ChallengeError::Invalid("lifetime"));
        }
        Ok(Self {
            provider,
            state,
            nonce,
            code_verifier,
            return_to,
            created_at,
            expires_at,
        })
    }

    /// The provider name the challenge was issued for, such as `google`.
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// The OAuth `state` sent to the provider.
    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }

    /// The OIDC nonce, for flows started as OpenID Connect.
    #[must_use]
    pub fn nonce(&self) -> Option<&str> {
        self.nonce.as_deref()
    }

    /// The PKCE verifier for the token exchange.
    #[must_use]
    pub fn code_verifier(&self) -> &str {
        &self.code_verifier
    }

    /// Where to send the browser after login, when the attempt named a page.
    #[must_use]
    pub fn return_to(&self) -> Option<&ReturnPath> {
        self.return_to.as_ref()
    }

    /// Unix second the challenge was created.
    #[must_use]
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Unix second from which the challenge is expired.
    #[must_use]
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Whether the challenge is expired at Unix second `now`.
    #[must_use]
    pub fn is_expired_at(&self, now: u64) -> bool {
        now >= self.expires_at
    }
}

impl fmt::Debug for LoginChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoginChallenge")
            .field("provider", &self.provider)
            .field("state", &REDACTED)
            .field("nonce", &self.nonce.as_ref().map(|_| REDACTED))
            .field("code_verifier", &REDACTED)
            .field("return_to", &self.return_to)
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

fn valid_provider(provider: &str) -> bool {
    (1..=MAX_PROVIDER_BYTES).contains(&provider.len())
        && provider.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn unreserved(value: &str, minimum: usize) -> bool {
    (minimum..=MAX_SECRET_BYTES).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
