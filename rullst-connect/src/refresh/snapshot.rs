//! Authenticated encryption for application-persisted refresh-token state.

use super::RefreshableTokenState;
use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, Generate, KeyInit, Nonce, Payload},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

const ENVELOPE_PREFIX: &str = "RULLST-CONNECT";
const ENVELOPE_VERSION: &str = "v1";
const AAD_DOMAIN: &[u8] = b"rullst-connect:refresh-token-snapshot";
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;
const MAX_KEY_ID_BYTES: usize = 128;
const MAX_PROVIDER_BYTES: usize = 128;
const MAX_ACCOUNT_BYTES: usize = 512;
const MAX_ENVELOPE_BYTES: usize = 192 * 1024;

/// Typed failures for encrypted refresh-token snapshots.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum TokenSnapshotError {
    /// Key identifiers are required for explicit key rotation.
    #[error("token snapshot key ID must contain 1 to 128 portable characters")]
    InvalidKeyId,
    /// Provider/account binding is empty, oversized or contains control bytes.
    #[error("token snapshot {field} binding is invalid")]
    InvalidBinding {
        /// Stable field name; the sensitive binding value is never included.
        field: &'static str,
    },
    /// The stored value is malformed or exceeds the fixed envelope ceiling.
    #[error("token snapshot envelope is malformed or oversized")]
    InvalidEnvelope,
    /// The stored version is not implemented by this release.
    #[error("token snapshot version is unsupported")]
    UnsupportedVersion,
    /// The supplied key does not match the envelope key identifier.
    #[error("token snapshot requires a different key ID")]
    KeyIdMismatch,
    /// Random nonce generation was unavailable.
    #[error("operating-system randomness is unavailable")]
    RandomnessUnavailable,
    /// Encryption failed without exposing token contents.
    #[error("token snapshot encryption failed")]
    EncryptionFailed,
    /// Key, binding, nonce, ciphertext or authentication tag did not match.
    #[error("token snapshot authentication failed")]
    AuthenticationFailed,
    /// Authenticated plaintext did not contain valid bounded token state.
    #[error("token snapshot payload is invalid")]
    InvalidPayload,
}

/// Stable application identity bound into a token snapshot's authentication tag.
///
/// Binding prevents a valid encrypted record from being copied to a different
/// provider or local account. The application must derive both values from
/// trusted authorization state rather than request parameters.
#[derive(Clone, Eq, PartialEq)]
pub struct TokenSnapshotBinding {
    provider: String,
    account_id: String,
}

impl TokenSnapshotBinding {
    /// Creates one validated provider/account binding.
    pub fn try_new(
        provider: impl Into<String>,
        account_id: impl Into<String>,
    ) -> Result<Self, TokenSnapshotError> {
        let provider = provider.into();
        let account_id = account_id.into();
        validate_binding("provider", &provider, MAX_PROVIDER_BYTES)?;
        validate_binding("account", &account_id, MAX_ACCOUNT_BYTES)?;
        Ok(Self {
            provider,
            account_id,
        })
    }

    /// Returns the provider label selected by trusted application code.
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }
}

impl fmt::Debug for TokenSnapshotBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenSnapshotBinding")
            .field("provider", &self.provider)
            .field("account_id", &"[REDACTED]")
            .finish()
    }
}

/// AES-256-GCM key selected by an explicit rotation identifier.
///
/// Supply 32 bytes from a secret manager or CSPRNG. Human passwords are not
/// keys and must first pass through a suitable password-based KDF. The cipher
/// zeroizes its retained key material when dropped; the caller remains
/// responsible for any earlier copies.
pub struct TokenSnapshotKey {
    key_id: String,
    cipher: Aes256Gcm,
}

impl TokenSnapshotKey {
    /// Constructs a key from exactly 256 bits of high-entropy material.
    pub fn try_new(key_id: impl Into<String>, key: [u8; 32]) -> Result<Self, TokenSnapshotError> {
        let key_id = key_id.into();
        validate_key_id(&key_id)?;
        let key = Zeroizing::new(key);
        let cipher = Aes256Gcm::new_from_slice(key.as_ref())
            .map_err(|_| TokenSnapshotError::EncryptionFailed)?;
        Ok(Self { key_id, cipher })
    }

    /// Returns the non-secret identifier used to select a rotation key.
    #[must_use]
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

impl fmt::Debug for TokenSnapshotKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenSnapshotKey")
            .field("key_id", &self.key_id)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

/// Opaque, bounded encrypted token state suitable for application-owned storage.
///
/// The envelope contains only a version, rotation key ID, random nonce and
/// authenticated ciphertext. It deliberately does not implement `Display` or
/// Serde so secrets are not accidentally emitted through generic responses or
/// logs. Use [`Self::as_str`] only at the explicit persistence boundary.
#[derive(Clone, Eq, PartialEq)]
pub struct EncryptedTokenSnapshot(String);

impl EncryptedTokenSnapshot {
    /// Encrypts and authenticates a validated token generation.
    pub fn seal(
        state: &RefreshableTokenState,
        key: &TokenSnapshotKey,
        binding: &TokenSnapshotBinding,
    ) -> Result<Self, TokenSnapshotError> {
        let stored = StoredStateRef {
            provider_user_id: state.provider_user_id(),
            access_token: state.access_token().expose_secret(),
            refresh_token: state.refresh_token().expose_secret(),
            issued_at: state.issued_at(),
            expires_at: state.expires_at(),
            generation: state.generation(),
        };
        let plaintext = Zeroizing::new(
            serde_json::to_vec(&stored).map_err(|_| TokenSnapshotError::InvalidPayload)?,
        );
        let nonce = Nonce::<Aes256Gcm>::try_generate()
            .map_err(|_| TokenSnapshotError::RandomnessUnavailable)?;
        let aad = authenticated_data(&key.key_id, binding);
        let ciphertext = key
            .cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext.as_ref(),
                    aad: &aad,
                },
            )
            .map_err(|_| TokenSnapshotError::EncryptionFailed)?;
        let envelope = format!(
            "{ENVELOPE_PREFIX}:{ENVELOPE_VERSION}:{}:{}:{}",
            key.key_id,
            URL_SAFE_NO_PAD.encode(nonce),
            URL_SAFE_NO_PAD.encode(ciphertext)
        );
        if envelope.len() > MAX_ENVELOPE_BYTES {
            return Err(TokenSnapshotError::InvalidEnvelope);
        }
        Ok(Self(envelope))
    }

    /// Validates an envelope loaded from the application's durable store.
    pub fn try_from_envelope(envelope: impl Into<String>) -> Result<Self, TokenSnapshotError> {
        let envelope = envelope.into();
        parse_envelope(&envelope)?;
        Ok(Self(envelope))
    }

    /// Decrypts, authenticates and revalidates one stored token generation.
    pub fn open(
        &self,
        key: &TokenSnapshotKey,
        binding: &TokenSnapshotBinding,
    ) -> Result<RefreshableTokenState, TokenSnapshotError> {
        let parsed = parse_envelope(&self.0)?;
        if parsed.key_id != key.key_id {
            return Err(TokenSnapshotError::KeyIdMismatch);
        }
        let aad = authenticated_data(parsed.key_id, binding);
        let nonce = Nonce::<Aes256Gcm>::try_from(parsed.nonce.as_slice())
            .map_err(|_| TokenSnapshotError::InvalidEnvelope)?;
        let plaintext = key
            .cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &parsed.ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| TokenSnapshotError::AuthenticationFailed)?;
        let plaintext = Zeroizing::new(plaintext);
        let stored: StoredState = serde_json::from_slice(plaintext.as_ref())
            .map_err(|_| TokenSnapshotError::InvalidPayload)?;
        RefreshableTokenState::try_restore(
            stored.provider_user_id,
            stored.access_token,
            stored.refresh_token,
            stored.issued_at,
            stored.expires_at,
            stored.generation,
        )
        .map_err(|_| TokenSnapshotError::InvalidPayload)
    }

    /// Returns the authenticated envelope's non-secret rotation key ID.
    pub fn key_id(&self) -> Result<&str, TokenSnapshotError> {
        Ok(parse_envelope(&self.0)?.key_id)
    }

    /// Exposes the opaque envelope at an explicit persistence boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for EncryptedTokenSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncryptedTokenSnapshot")
            .field("envelope", &"[REDACTED]")
            .finish()
    }
}

#[derive(Serialize)]
struct StoredStateRef<'state> {
    provider_user_id: &'state str,
    access_token: &'state str,
    refresh_token: &'state str,
    issued_at: u64,
    expires_at: u64,
    generation: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredState {
    provider_user_id: String,
    access_token: String,
    refresh_token: String,
    issued_at: u64,
    expires_at: u64,
    generation: u64,
}

struct ParsedEnvelope<'a> {
    key_id: &'a str,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

fn parse_envelope(envelope: &str) -> Result<ParsedEnvelope<'_>, TokenSnapshotError> {
    if envelope.is_empty() || envelope.len() > MAX_ENVELOPE_BYTES {
        return Err(TokenSnapshotError::InvalidEnvelope);
    }
    let mut fields = envelope.split(':');
    if fields.next() != Some(ENVELOPE_PREFIX) {
        return Err(TokenSnapshotError::InvalidEnvelope);
    }
    if fields.next() != Some(ENVELOPE_VERSION) {
        return Err(TokenSnapshotError::UnsupportedVersion);
    }
    let key_id = fields.next().ok_or(TokenSnapshotError::InvalidEnvelope)?;
    validate_key_id(key_id)?;
    let nonce = URL_SAFE_NO_PAD
        .decode(fields.next().ok_or(TokenSnapshotError::InvalidEnvelope)?)
        .map_err(|_| TokenSnapshotError::InvalidEnvelope)?;
    let ciphertext = URL_SAFE_NO_PAD
        .decode(fields.next().ok_or(TokenSnapshotError::InvalidEnvelope)?)
        .map_err(|_| TokenSnapshotError::InvalidEnvelope)?;
    if fields.next().is_some() || nonce.len() != NONCE_BYTES || ciphertext.len() < TAG_BYTES {
        return Err(TokenSnapshotError::InvalidEnvelope);
    }
    Ok(ParsedEnvelope {
        key_id,
        nonce,
        ciphertext,
    })
}

fn validate_key_id(key_id: &str) -> Result<(), TokenSnapshotError> {
    if key_id.is_empty()
        || key_id.len() > MAX_KEY_ID_BYTES
        || !key_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(TokenSnapshotError::InvalidKeyId);
    }
    Ok(())
}

fn validate_binding(
    field: &'static str,
    value: &str,
    maximum: usize,
) -> Result<(), TokenSnapshotError> {
    if value.is_empty()
        || value.len() > maximum
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(TokenSnapshotError::InvalidBinding { field });
    }
    Ok(())
}

fn authenticated_data(key_id: &str, binding: &TokenSnapshotBinding) -> Vec<u8> {
    let mut aad = Vec::with_capacity(
        AAD_DOMAIN.len() + key_id.len() + binding.provider.len() + binding.account_id.len() + 32,
    );
    append_field(&mut aad, AAD_DOMAIN);
    append_field(&mut aad, ENVELOPE_VERSION.as_bytes());
    append_field(&mut aad, key_id.as_bytes());
    append_field(&mut aad, binding.provider.as_bytes());
    append_field(&mut aad, binding.account_id.as_bytes());
    aad
}

fn append_field(target: &mut Vec<u8>, field: &[u8]) {
    target.extend_from_slice(&(field.len() as u64).to_be_bytes());
    target.extend_from_slice(field);
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
