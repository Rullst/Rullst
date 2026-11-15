//! The `v3.` login token of a registered session and the session secret it
//! carries.
//!
//! The token is the AES-256-GCM envelope of the other `rullst_session`
//! tokens, with its own prefix and associated data, around
//! `user_id|epoch|expires_at|secret`. The secret is 32 random bytes; the
//! registry stores only two purpose-separated SHA-256 digests of it, so a
//! copy of the table names no secret that a cookie could carry.

use super::super::session::{open_envelope, seal_envelope};
use super::super::session_claims::SessionClaims;
use super::{SessionRef, SessionRegistryError};
use aes_gcm::Aes256Gcm;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use zeroize::Zeroizing;

/// Version prefix of registered-session tokens.
pub(super) const REGISTERED_TOKEN_PREFIX: &str = "v3.";
/// Associated data binding the registered payload format to its prefix.
const REGISTERED_SESSION_AAD: &[u8] = b"rullst.session.v3";
/// Random bytes in a session secret.
const SECRET_BYTES: usize = 32;
/// Characters of a secret in base64url without padding.
const ENCODED_SECRET_LEN: usize = 43;
/// Separates the lookup digest from any other SHA-256 of the secret.
const LOOKUP_DOMAIN: &[u8] = b"rullst.auth.session-registry.lookup.v1\0";
/// Separates the management reference from the lookup digest.
const REFERENCE_DOMAIN: &[u8] = b"rullst.auth.session-registry.ref.v1\0";
/// Bytes of the lookup digest kept in the management reference.
const REFERENCE_BYTES: usize = 16;

/// The random secret of one session. Never stored; zeroed when dropped.
pub(super) struct SessionSecret(Zeroizing<[u8; SECRET_BYTES]>);

impl SessionSecret {
    /// Draws a new secret from the operating system's CSPRNG.
    pub(super) fn generate() -> Result<Self, SessionRegistryError> {
        let mut bytes = Zeroizing::new([0u8; SECRET_BYTES]);
        SystemRandom::new()
            .fill(bytes.as_mut())
            .map_err(|_| SessionRegistryError::Crypto)?;
        Ok(Self(bytes))
    }

    /// The secret as it travels inside the sealed token.
    fn decode(encoded: &str) -> Option<Self> {
        if encoded.len() != ENCODED_SECRET_LEN {
            return None;
        }
        let decoded = Zeroizing::new(URL_SAFE_NO_PAD.decode(encoded).ok()?);
        let bytes: [u8; SECRET_BYTES] = decoded.as_slice().try_into().ok()?;
        Some(Self(Zeroizing::new(bytes)))
    }

    pub(super) fn encode(&self) -> Zeroizing<String> {
        Zeroizing::new(URL_SAFE_NO_PAD.encode(self.0.as_ref()))
    }

    /// The stored primary key: lowercase hex SHA-256 of the domain and the
    /// secret, 64 characters.
    pub(super) fn lookup_hash(&self) -> String {
        let digest = Sha256::new()
            .chain_update(LOOKUP_DOMAIN)
            .chain_update(self.0.as_ref())
            .finalize();
        let mut hex = String::with_capacity(64);
        for byte in digest {
            let _ = write!(hex, "{byte:02x}");
        }
        hex
    }

    /// The account-visible management reference: 16 bytes of another
    /// domain's digest, in base64url (22 characters).
    pub(super) fn reference(&self) -> SessionRef {
        let digest = Sha256::new()
            .chain_update(REFERENCE_DOMAIN)
            .chain_update(self.0.as_ref())
            .finalize();
        SessionRef(URL_SAFE_NO_PAD.encode(&digest[..REFERENCE_BYTES]))
    }
}

/// Seals the registered-session token of `user_id`.
pub(super) fn seal(
    cipher: &Aes256Gcm,
    claims: &SessionClaims,
    secret: &SessionSecret,
) -> Result<Zeroizing<String>, SessionRegistryError> {
    let payload = Zeroizing::new(format!(
        "{}|{}|{}|{}",
        claims.user_id,
        claims.epoch,
        claims.expires_at,
        secret.encode().as_str()
    ));
    seal_envelope(
        cipher,
        REGISTERED_TOKEN_PREFIX,
        REGISTERED_SESSION_AAD,
        &payload,
    )
    .map(Zeroizing::new)
    .map_err(|_| SessionRegistryError::Crypto)
}

/// Opens a registered-session token. With `now`, a token past its expiry
/// fails with [`SessionRegistryError::Expired`]; signing out passes `None`,
/// so an expired cookie can still remove its record.
pub(super) fn open(
    cipher: &Aes256Gcm,
    token: &str,
    now: Option<u64>,
) -> Result<(SessionClaims, SessionSecret), SessionRegistryError> {
    let encoded = token
        .strip_prefix(REGISTERED_TOKEN_PREFIX)
        .ok_or(SessionRegistryError::InvalidToken)?;
    let payload = Zeroizing::new(
        open_envelope(cipher, encoded, REGISTERED_SESSION_AAD)
            .map_err(|_| SessionRegistryError::InvalidToken)?,
    );
    let mut fields = payload.split('|');
    let (Some(user_id), Some(epoch), Some(expires_at), Some(secret), None) = (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) else {
        return Err(SessionRegistryError::InvalidToken);
    };
    let claims = SessionClaims {
        user_id: user_id
            .parse()
            .map_err(|_| SessionRegistryError::InvalidToken)?,
        epoch: epoch
            .parse()
            .map_err(|_| SessionRegistryError::InvalidToken)?,
        expires_at: expires_at
            .parse()
            .map_err(|_| SessionRegistryError::InvalidToken)?,
    };
    let secret = SessionSecret::decode(secret).ok_or(SessionRegistryError::InvalidToken)?;
    if now.is_some_and(|now| now > claims.expires_at) {
        return Err(SessionRegistryError::Expired);
    }
    Ok((claims, secret))
}

#[cfg(test)]
#[path = "token_tests.rs"]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
