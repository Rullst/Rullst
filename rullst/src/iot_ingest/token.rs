//! Device bearer tokens and the only form a directory stores: their SHA-256.

use sha2::{Digest, Sha256};
use std::fmt;

/// Prefix of every generated device token, so secret scanners can find one
/// that leaked.
const PREFIX: &str = "rdt1_";
/// Shortest and longest bearer token accepted on the wire.
pub(crate) const TOKEN_LENGTH: std::ops::RangeInclusive<usize> = 32..=200;

/// A device's bearer secret. [`Self::generate`] draws it from the operating
/// system's CSPRNG (190 random bits after the `rdt1_` prefix); store only its
/// [`Self::hash`] and hand the secret to the device once. `Debug` is
/// redacted.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, PartialEq, Eq)]
pub struct DeviceToken(String);

impl DeviceToken {
    /// A new random token, such as `rdt1_8mXq…` (37 characters).
    pub fn generate() -> Self {
        Self(format!(
            "{PREFIX}{}",
            rullst_core::security::generate_csrf_token()
        ))
    }

    /// The secret, to give to the device (for example in its gateway's
    /// configuration). Never log it.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// The digest a [`super::DeviceDirectory`] stores and looks up.
    pub fn hash(&self) -> TokenHash {
        TokenHash::of(&self.0)
    }
}

impl fmt::Debug for DeviceToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DeviceToken(<redacted>)")
    }
}

/// The lowercase hexadecimal SHA-256 of a device token (64 characters).
///
/// A token carries at least 190 random bits, so its plain digest cannot be
/// reversed or guessed, and a directory can look a device up by it with an
/// ordinary index.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TokenHash(String);

impl TokenHash {
    /// The digest of `token`.
    pub fn of(token: &str) -> Self {
        let digest = Sha256::digest(token.as_bytes());
        let mut hex = String::with_capacity(64);
        for byte in digest.iter() {
            hex.push(char::from(HEX[usize::from(byte >> 4)]));
            hex.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        Self(hex)
    }

    /// A stored digest read back, or `None` unless it is 64 lowercase
    /// hexadecimal characters.
    pub fn from_hex(hex: impl Into<String>) -> Option<Self> {
        let hex = hex.into();
        (hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
            .then_some(Self(hex))
    }

    /// The 64 hexadecimal characters.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Whether `token` has the shape of a bearer token at all (32–200 printable,
/// non-whitespace ASCII bytes), checked before it is hashed.
pub(crate) fn plausible_token(token: &str) -> bool {
    TOKEN_LENGTH.contains(&token.len()) && token.bytes().all(|b| b.is_ascii_graphic())
}
