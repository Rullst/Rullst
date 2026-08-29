//! Zero-Trust Secret Management & In-Memory Zeroization (Rullst Vault).

use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, Generate, KeyInit, Nonce, Payload},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::fmt;
use zeroize::Zeroize;

const ENVELOPE_PREFIX: &str = "RULLST";
const LEGACY_ENVELOPE_PREFIX: &str = "ENC";
const ENVELOPE_VERSION: &str = "v2";
const LEGACY_ENVELOPE_VERSION: &str = "v1";
const DEFAULT_KEY_ID: &str = "default";
const KEY_LENGTH: usize = 32;
const NONCE_LENGTH: usize = 12;
const TAG_LENGTH: usize = 16;
const MAX_KEY_ID_LENGTH: usize = 128;
const AAD_DOMAIN: &[u8] = b"rullst-security:field-encryption:aes-256-gcm";

/// Zero-Trust wrapper for sensitive in-memory secrets (API keys, DB passwords, private tokens).
/// Zeroizes the wrapped value on drop, reducing how long that allocation keeps
/// the secret. It cannot erase prior copies or prevent process-memory capture.
pub struct VaultSecret<T: Zeroize> {
    inner: T,
}

impl<T: Zeroize> VaultSecret<T> {
    /// Creates a new VaultSecret wrapping a value.
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    /// Accesses the inner secret value.
    pub fn expose_secret(&self) -> &T {
        &self.inner
    }
}

impl<T: Zeroize> Drop for VaultSecret<T> {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

impl<T: Zeroize> fmt::Debug for VaultSecret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VaultSecret(***REDACTED***)")
    }
}

impl<T: Zeroize> fmt::Display for VaultSecret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***REDACTED***")
    }
}

/// Strongly typed failures produced by field-level encryption.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VaultError {
    /// AES-256 requires exactly 32 bytes of key material.
    #[error("AES-256-GCM keys must be exactly {expected} bytes; received {actual}")]
    InvalidKeyLength {
        /// Required AES-256 key length.
        expected: usize,
        /// Supplied key length.
        actual: usize,
    },

    /// Key identifiers are required so encrypted fields can participate in key rotation.
    #[error("field-encryption key identifiers cannot be empty")]
    EmptyKeyId,

    /// Key identifiers have a conservative limit to keep envelopes bounded.
    #[error("field-encryption key identifier is too long (maximum {maximum}, received {actual})")]
    KeyIdTooLong {
        /// Maximum supported key identifier length.
        maximum: usize,
        /// Supplied key identifier length.
        actual: usize,
    },

    /// Key identifiers may contain only portable, delimiter-safe characters.
    #[error(
        "field-encryption key identifiers may contain only ASCII letters, digits, '.', '_' and '-'"
    )]
    InvalidKeyId,

    /// The envelope does not have the required fields or prefix.
    #[error("invalid field-encryption envelope")]
    InvalidEnvelope,

    /// The previous `v1` format was an irreversible hash and cannot be migrated by decryption.
    #[error("legacy irreversible field-encryption envelope '{version}' cannot be decrypted")]
    LegacyIrreversibleEnvelope {
        /// Legacy envelope version found in storage.
        version: String,
    },

    /// The envelope is well formed but belongs to an unknown future version.
    #[error("unsupported field-encryption envelope version '{version}'")]
    UnsupportedEnvelopeVersion {
        /// Unsupported envelope version found in storage.
        version: String,
    },

    /// A binary envelope component is not valid URL-safe base64.
    #[error("invalid base64 encoding in field-encryption envelope component '{component}'")]
    InvalidEnvelopeEncoding {
        /// Name of the malformed component.
        component: &'static str,
    },

    /// AES-GCM uses a fixed 96-bit nonce.
    #[error("invalid AES-GCM nonce length (expected {expected}, received {actual})")]
    InvalidNonceLength {
        /// Required nonce length.
        expected: usize,
        /// Supplied nonce length.
        actual: usize,
    },

    /// AES-GCM ciphertext must contain at least its authentication tag.
    #[error("encrypted payload is too short (minimum {minimum}, received {actual})")]
    CiphertextTooShort {
        /// Minimum ciphertext and tag length.
        minimum: usize,
        /// Supplied ciphertext length.
        actual: usize,
    },

    /// The operating system could not provide cryptographically secure randomness.
    #[error("operating-system randomness is unavailable")]
    RandomnessUnavailable,

    /// AES-GCM rejected an encryption request.
    #[error("AES-256-GCM encryption failed")]
    EncryptionFailed,

    /// Authentication failed because the key, AAD, nonce, tag, or ciphertext does not match.
    #[error("AES-256-GCM authentication failed")]
    AuthenticationFailed,

    /// The authenticated plaintext was not a UTF-8 string.
    #[error("decrypted field is not valid UTF-8")]
    InvalidPlaintextUtf8,

    /// No key with the envelope's key identifier exists in the supplied rotation keyring.
    #[error("field-encryption key '{key_id}' was not found in the keyring")]
    KeyNotFound {
        /// Identifier requested by the envelope.
        key_id: String,
    },
}

impl From<VaultError> for crate::error::SecurityError {
    fn from(error: VaultError) -> Self {
        Self::VaultError(error.to_string())
    }
}

/// AES-256-GCM field-level encryption helper.
///
/// Encrypted values use the versioned envelope
/// `RULLST:v2:<key-id>:<nonce>:<ciphertext-and-tag>`. The nonce and encrypted payload
/// are URL-safe base64 without padding. The envelope version, algorithm domain,
/// key identifier, and caller-provided AAD are authenticated by AES-GCM.
///
/// Keys must be exactly 32 bytes of high-entropy key material. Passwords must be
/// processed by a suitable password-based KDF before they are passed here.
pub struct FieldEncryptor;

impl FieldEncryptor {
    /// Number of bytes required for an AES-256 key.
    pub const KEY_LENGTH: usize = KEY_LENGTH;

    /// Current field-encryption envelope version.
    pub const ENVELOPE_VERSION: &'static str = ENVELOPE_VERSION;

    /// Encrypts a UTF-8 field with the default key identifier and no caller AAD.
    ///
    /// Prefer [`Self::encrypt_with_aad`] when a stable record identity is available.
    pub fn encrypt(plain_text: &str, secret_key: impl AsRef<[u8]>) -> Result<String, VaultError> {
        Self::encrypt_with_aad(plain_text, secret_key, b"")
    }

    /// Encrypts a UTF-8 field and binds it to caller-provided additional data.
    ///
    /// Suitable AAD includes a tenant, table, column, and stable record identifier.
    /// Decryption must receive the exact same bytes.
    pub fn encrypt_with_aad(
        plain_text: &str,
        secret_key: impl AsRef<[u8]>,
        aad: &[u8],
    ) -> Result<String, VaultError> {
        Self::encrypt_with_key_id(plain_text, secret_key, DEFAULT_KEY_ID, aad)
    }

    /// Encrypts a field with an explicit key identifier for key rotation.
    pub fn encrypt_with_key_id(
        plain_text: &str,
        secret_key: impl AsRef<[u8]>,
        key_id: impl Into<String>,
        aad: &[u8],
    ) -> Result<String, VaultError> {
        let key_id = key_id.into();
        validate_key_id(&key_id)?;
        let cipher = create_cipher(secret_key.as_ref())?;
        let nonce =
            Nonce::<Aes256Gcm>::try_generate().map_err(|_| VaultError::RandomnessUnavailable)?;
        let authenticated_data = build_authenticated_data(&key_id, aad);
        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plain_text.as_bytes(),
                    aad: &authenticated_data,
                },
            )
            .map_err(|_| VaultError::EncryptionFailed)?;

        Ok(format!(
            "{ENVELOPE_PREFIX}:{ENVELOPE_VERSION}:{key_id}:{}:{}",
            URL_SAFE_NO_PAD.encode(nonce),
            URL_SAFE_NO_PAD.encode(ciphertext)
        ))
    }

    /// Decrypts a field encrypted with [`Self::encrypt`].
    pub fn decrypt(cipher_text: &str, secret_key: impl AsRef<[u8]>) -> Result<String, VaultError> {
        Self::decrypt_with_aad(cipher_text, secret_key, b"")
    }

    /// Decrypts a field and authenticates the caller-provided additional data.
    pub fn decrypt_with_aad(
        cipher_text: &str,
        secret_key: impl AsRef<[u8]>,
        aad: &[u8],
    ) -> Result<String, VaultError> {
        let envelope = parse_envelope(cipher_text)?;
        decrypt_envelope(&envelope, secret_key.as_ref(), aad)
    }

    /// Decrypts using the key selected by the authenticated envelope identifier.
    ///
    /// Keep the current key and any still-readable previous keys in `keyring` while
    /// records are re-encrypted during rotation.
    pub fn decrypt_with_keyring(
        cipher_text: &str,
        keyring: &[(&str, &[u8])],
        aad: &[u8],
    ) -> Result<String, VaultError> {
        let envelope = parse_envelope(cipher_text)?;
        let key = keyring
            .iter()
            .find_map(|(key_id, key)| (*key_id == envelope.key_id).then_some(*key))
            .ok_or_else(|| VaultError::KeyNotFound {
                key_id: envelope.key_id.to_string(),
            })?;

        decrypt_envelope(&envelope, key, aad)
    }

    /// Returns the key identifier carried by a validated envelope.
    pub fn envelope_key_id(cipher_text: &str) -> Result<&str, VaultError> {
        Ok(parse_envelope(cipher_text)?.key_id)
    }
}

struct ParsedEnvelope<'a> {
    key_id: &'a str,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

fn create_cipher(secret_key: &[u8]) -> Result<Aes256Gcm, VaultError> {
    if secret_key.len() != KEY_LENGTH {
        return Err(VaultError::InvalidKeyLength {
            expected: KEY_LENGTH,
            actual: secret_key.len(),
        });
    }

    Aes256Gcm::new_from_slice(secret_key).map_err(|_| VaultError::InvalidKeyLength {
        expected: KEY_LENGTH,
        actual: secret_key.len(),
    })
}

fn validate_key_id(key_id: &str) -> Result<(), VaultError> {
    if key_id.is_empty() {
        return Err(VaultError::EmptyKeyId);
    }
    if key_id.len() > MAX_KEY_ID_LENGTH {
        return Err(VaultError::KeyIdTooLong {
            maximum: MAX_KEY_ID_LENGTH,
            actual: key_id.len(),
        });
    }
    if !key_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(VaultError::InvalidKeyId);
    }

    Ok(())
}

fn build_authenticated_data(key_id: &str, caller_aad: &[u8]) -> Vec<u8> {
    let mut aad = Vec::new();
    aad.extend_from_slice(AAD_DOMAIN);
    aad.push(0);
    aad.extend_from_slice(ENVELOPE_VERSION.as_bytes());
    aad.push(0);
    aad.extend_from_slice(key_id.as_bytes());
    aad.push(0);
    aad.extend_from_slice(caller_aad);
    aad
}

fn parse_envelope(cipher_text: &str) -> Result<ParsedEnvelope<'_>, VaultError> {
    let mut fields = cipher_text.split(':');
    if !matches!(
        fields.next(),
        Some(ENVELOPE_PREFIX | LEGACY_ENVELOPE_PREFIX)
    ) {
        return Err(VaultError::InvalidEnvelope);
    }

    let version = fields.next().ok_or(VaultError::InvalidEnvelope)?;
    if version == LEGACY_ENVELOPE_VERSION {
        return Err(VaultError::LegacyIrreversibleEnvelope {
            version: version.to_string(),
        });
    }
    if version != ENVELOPE_VERSION {
        return Err(VaultError::UnsupportedEnvelopeVersion {
            version: version.to_string(),
        });
    }

    let key_id = fields.next().ok_or(VaultError::InvalidEnvelope)?;
    let encoded_nonce = fields.next().ok_or(VaultError::InvalidEnvelope)?;
    let encoded_ciphertext = fields.next().ok_or(VaultError::InvalidEnvelope)?;
    if fields.next().is_some() {
        return Err(VaultError::InvalidEnvelope);
    }
    validate_key_id(key_id)?;

    let nonce = URL_SAFE_NO_PAD
        .decode(encoded_nonce)
        .map_err(|_| VaultError::InvalidEnvelopeEncoding { component: "nonce" })?;
    if nonce.len() != NONCE_LENGTH {
        return Err(VaultError::InvalidNonceLength {
            expected: NONCE_LENGTH,
            actual: nonce.len(),
        });
    }

    let ciphertext = URL_SAFE_NO_PAD.decode(encoded_ciphertext).map_err(|_| {
        VaultError::InvalidEnvelopeEncoding {
            component: "ciphertext",
        }
    })?;
    if ciphertext.len() < TAG_LENGTH {
        return Err(VaultError::CiphertextTooShort {
            minimum: TAG_LENGTH,
            actual: ciphertext.len(),
        });
    }

    Ok(ParsedEnvelope {
        key_id,
        nonce,
        ciphertext,
    })
}

fn decrypt_envelope(
    envelope: &ParsedEnvelope<'_>,
    secret_key: &[u8],
    aad: &[u8],
) -> Result<String, VaultError> {
    let cipher = create_cipher(secret_key)?;
    let nonce = Nonce::<Aes256Gcm>::try_from(envelope.nonce.as_slice()).map_err(|_| {
        VaultError::InvalidNonceLength {
            expected: NONCE_LENGTH,
            actual: envelope.nonce.len(),
        }
    })?;
    let authenticated_data = build_authenticated_data(envelope.key_id, aad);
    let plaintext = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: &envelope.ciphertext,
                aad: &authenticated_data,
            },
        )
        .map_err(|_| VaultError::AuthenticationFailed)?;

    match String::from_utf8(plaintext) {
        Ok(plaintext) => Ok(plaintext),
        Err(error) => {
            let mut plaintext = error.into_bytes();
            plaintext.zeroize();
            Err(VaultError::InvalidPlaintextUtf8)
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_vault_secret_invariants() {
        let val: [u8; 4] = kani::any();
        let secret = VaultSecret::new(val);
        assert_eq!(secret.expose_secret(), &val);
    }
}
