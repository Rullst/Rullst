//! Single-use password-reset tokens for an application that keeps its own
//! users table, such as the one `cargo rullst auth` generates.
//!
//! [`PasswordResetToken::generate`] draws 256 bits from the operating
//! system's CSPRNG. The application emails [`PasswordResetToken::expose`] in a
//! link and stores only [`PasswordResetToken::digest`], with an expiry. When
//! the link comes back, [`password_reset_token_digest`] turns the submitted
//! value into the digest to look up, and [`password_reset_digests_match`]
//! compares a stored digest with it in constant time. Expiry, single use and
//! one live token per account are rules of the application's store; the
//! opt-in `recovery` module (features `recovery-sqlite`/`recovery-postgres`)
//! is a complete store with its own account table.

use crate::error::AuthError;
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Random bytes in a token.
const TOKEN_BYTES: usize = 32;
/// Characters in an encoded token and in a digest: 32 bytes as lowercase hex.
const ENCODED_LEN: usize = 64;
/// Separates these digests from any other SHA-256 of the same text.
const DIGEST_DOMAIN: &[u8] = b"rullst.auth.password-reset.v1\0";

/// A random password-reset token: 32 bytes from the operating system's
/// CSPRNG, written as 64 lowercase hexadecimal characters.
///
/// Lowercase hex is safe in a URL query and passes the mail pipeline's
/// secret redaction unchanged. Send [`expose`](Self::expose) only to the
/// account's email address and store only [`digest`](Self::digest). `Debug`
/// never shows the token, and its memory is zeroed when it is dropped.
///
/// ```
/// use rullst_auth::{PasswordResetToken, password_reset_token_digest};
///
/// let token = PasswordResetToken::generate()?;
/// let stored = token.digest();
/// // Later, the value from the emailed link:
/// assert_eq!(password_reset_token_digest(token.expose()), Some(stored));
/// # Ok::<(), rullst_auth::AuthError>(())
/// ```
pub struct PasswordResetToken(Zeroizing<String>);

impl std::fmt::Debug for PasswordResetToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PasswordResetToken([REDACTED])")
    }
}

impl PasswordResetToken {
    /// Draws a new token from the operating system's CSPRNG.
    ///
    /// # Errors
    /// [`AuthError::General`] when the random number generator fails.
    pub fn generate() -> Result<Self, AuthError> {
        let mut bytes = Zeroizing::new([0u8; TOKEN_BYTES]);
        SystemRandom::new().fill(bytes.as_mut()).map_err(|_| {
            AuthError::General("secure random number generator unavailable".to_string())
        })?;
        let mut encoded = Zeroizing::new(String::with_capacity(ENCODED_LEN));
        for byte in bytes.iter() {
            // Writing to a `String` cannot fail.
            let _ = write!(encoded, "{byte:02x}");
        }
        Ok(Self(encoded))
    }

    /// The token, for the emailed link only. Never log or store it.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// The digest to store and look up: 64 lowercase hex characters of a
    /// domain-separated SHA-256 of the token.
    pub fn digest(&self) -> String {
        digest_of(&self.0)
    }
}

/// The digest of a token submitted from a reset link, or `None` when the
/// value is not 64 lowercase hexadecimal characters, so a malformed value
/// never reaches the database.
pub fn password_reset_token_digest(token: &str) -> Option<String> {
    let well_formed = token.len() == ENCODED_LEN
        && token
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
    well_formed.then(|| digest_of(token))
}

/// Whether a stored digest equals the digest of a submitted token. The bytes
/// are compared in constant time; a stored value that is not a 64-character
/// digest never matches.
pub fn password_reset_digests_match(stored: &str, candidate: &str) -> bool {
    stored.len() == ENCODED_LEN && bool::from(stored.as_bytes().ct_eq(candidate.as_bytes()))
}

fn digest_of(token: &str) -> String {
    let digest = Sha256::new()
        .chain_update(DIGEST_DOMAIN)
        .chain_update(token.as_bytes())
        .finalize();
    let mut encoded = String::with_capacity(ENCODED_LEN);
    for byte in digest {
        // Writing to a `String` cannot fail.
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "reset_token_tests.rs"]
mod tests;
