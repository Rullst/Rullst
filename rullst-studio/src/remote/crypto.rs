//! Encryption of stored secrets (monitor tokens, webhook URLs) at rest.
//!
//! AES-256-GCM under a 256-bit key kept in its own owner-only file, separate
//! from the project list. Each value has a random 96-bit nonce, and its
//! associated data binds it to its project name and base URL: a value copied
//! to another project, or a base URL edited in the file, no longer decrypts,
//! so the secret is never sent to an origin the developer did not save.

use super::secure_file;
use base64::Engine;
use ring::aead::{AES_256_GCM, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use std::fmt;
use std::path::Path;

const KEY_BYTES: usize = 32;
const PREFIX: &str = "v1.";
/// Longest sealed value accepted from the file.
const MAX_SEALED_CHARS: usize = 4_096;

/// A secret value that never appears in `Debug` output.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// The secret value.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

/// Why a secret could not be stored or read. Never carries the secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CryptoError {
    /// The key file is unreadable, malformed or could not be written.
    Key,
    /// The operating system gave no randomness.
    Randomness,
    /// The value was not sealed with this key and associated data.
    Sealed,
}

/// The key that seals stored secrets.
pub(crate) struct SecretKey(LessSafeKey);

impl SecretKey {
    /// Loads the key at `path`; `Ok(None)` when it does not exist yet.
    pub(crate) fn load(path: &Path) -> Result<Option<Self>, CryptoError> {
        match secure_file::read_private(path, KEY_BYTES as u64) {
            Ok(Some(bytes)) => Self::from_bytes(&bytes).map(Some),
            Ok(None) => Ok(None),
            Err(_) => Err(CryptoError::Key),
        }
    }

    /// Loads the key, creating a new random one (owner-only) when absent.
    pub(crate) fn load_or_create(path: &Path) -> Result<Self, CryptoError> {
        if let Some(key) = Self::load(path)? {
            return Ok(key);
        }
        let mut bytes = [0_u8; KEY_BYTES];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| CryptoError::Randomness)?;
        secure_file::write_private(path, &bytes).map_err(|_| CryptoError::Key)?;
        Self::from_bytes(&bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != KEY_BYTES {
            return Err(CryptoError::Key);
        }
        UnboundKey::new(&AES_256_GCM, bytes)
            .map(|key| Self(LessSafeKey::new(key)))
            .map_err(|_| CryptoError::Key)
    }

    /// Encrypts `secret` bound to `context`, as `v1.<nonce>.<ciphertext>`.
    pub(crate) fn seal(&self, secret: &str, context: &[u8]) -> Result<String, CryptoError> {
        let mut nonce = [0_u8; NONCE_LEN];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| CryptoError::Randomness)?;
        let mut in_out = secret.as_bytes().to_vec();
        self.0
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(context),
                &mut in_out,
            )
            .map_err(|_| CryptoError::Sealed)?;
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        Ok(format!(
            "{PREFIX}{}.{}",
            engine.encode(nonce),
            engine.encode(in_out)
        ))
    }

    /// Decrypts a value produced by [`Self::seal`] with the same `context`.
    pub(crate) fn open(&self, sealed: &str, context: &[u8]) -> Result<Secret, CryptoError> {
        if sealed.len() > MAX_SEALED_CHARS {
            return Err(CryptoError::Sealed);
        }
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let (nonce, ciphertext) = sealed
            .strip_prefix(PREFIX)
            .and_then(|rest| rest.split_once('.'))
            .ok_or(CryptoError::Sealed)?;
        let nonce: [u8; NONCE_LEN] = engine
            .decode(nonce)
            .ok()
            .and_then(|nonce| nonce.try_into().ok())
            .ok_or(CryptoError::Sealed)?;
        let mut in_out = engine.decode(ciphertext).map_err(|_| CryptoError::Sealed)?;
        let plaintext = self
            .0
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(context),
                &mut in_out,
            )
            .map_err(|_| CryptoError::Sealed)?;
        String::from_utf8(plaintext.to_vec())
            .map(Secret)
            .map_err(|_| CryptoError::Sealed)
    }
}
