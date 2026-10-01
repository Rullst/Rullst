//! Encrypted session tokens and the session cookie helpers.

use super::app_key::{detect_environment, get_app_key, validate_app_key};
use crate::error::AuthError;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use axum::http::HeaderMap;
use base64::{Engine as _, engine::general_purpose};
use sha2::Digest;
use std::convert::TryInto;

pub(super) const SESSION_TOKEN_PREFIX: &str = "v1.";
pub(super) const SESSION_AAD: &[u8] = b"rullst.session.v1";
const MAX_SESSION_COOKIE_BYTES: usize = 4096;
pub(super) const LOGOUT_COOKIE: &str = "rullst_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT";
pub(super) const SECURE_LOGOUT_COOKIE: &str = "rullst_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; Secure";

static CACHED_CIPHER: std::sync::OnceLock<(Vec<u8>, Aes256Gcm)> = std::sync::OnceLock::new();

pub(super) fn derive_cipher(app_key: &[u8]) -> Result<Aes256Gcm, AuthError> {
    validate_app_key(app_key)?;
    if let Some((cached_key, cipher)) = CACHED_CIPHER.get()
        && cached_key.as_slice() == app_key
    {
        return Ok(cipher.clone());
    }

    let mut hasher = sha2::Sha256::new();
    hasher.update(app_key);
    let key_hash = hasher.finalize();
    let cipher = Aes256Gcm::new_from_slice(&key_hash)
        .map_err(|e| AuthError::SessionEncryptionError(e.to_string()))?;

    let _ = CACHED_CIPHER.set((app_key.to_vec(), cipher.clone()));
    Ok(cipher)
}

/// Encrypts a user_id into a secure base64-encoded string.
#[cfg_attr(mutants, mutants::skip)]
pub fn encrypt_session(user_id: i32, app_key: &[u8]) -> Result<String, AuthError> {
    let cipher = derive_cipher(app_key)?;

    let mut nonce_bytes = [0u8; 12];
    rand::fill(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AuthError::SessionEncryptionError(e.to_string()))?
        .as_secs();
    let exp = now + (30 * 24 * 60 * 60); // 30 days

    let payload = format!("{}|{}", user_id, exp);
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: payload.as_bytes(),
                aad: SESSION_AAD,
            },
        )
        .map_err(|e| AuthError::SessionEncryptionError(e.to_string()))?;

    let mut combined = Vec::new();
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(format!(
        "{SESSION_TOKEN_PREFIX}{}",
        general_purpose::URL_SAFE_NO_PAD.encode(&combined)
    ))
}

/// Decrypts a secure base64-encoded string back into a user_id.
#[cfg_attr(mutants, mutants::skip)]
pub fn decrypt_session(token: &str, app_key: &[u8]) -> Result<i32, AuthError> {
    let cipher = derive_cipher(app_key)?;

    let encoded = token.strip_prefix(SESSION_TOKEN_PREFIX).ok_or_else(|| {
        AuthError::SessionDecryptionError("Unsupported session token version".to_string())
    })?;
    let combined = general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|e| AuthError::SessionDecryptionError(e.to_string()))?;

    if combined.len() < 28 {
        return Err(AuthError::SessionDecryptionError(
            "Invalid token length".to_string(),
        ));
    }

    let nonce_bytes: [u8; 12] = combined[..12]
        .try_into()
        .map_err(|_| AuthError::SessionDecryptionError("Invalid token length".to_string()))?;
    let nonce = Nonce::from(nonce_bytes);
    let ciphertext = &combined[12..];

    let plaintext = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad: SESSION_AAD,
            },
        )
        .map_err(|e| AuthError::SessionDecryptionError(e.to_string()))?;

    let payload_str = String::from_utf8(plaintext)
        .map_err(|e| AuthError::SessionDecryptionError(e.to_string()))?;

    let (user_id_str, exp_str) = payload_str.split_once('|').ok_or_else(|| {
        AuthError::SessionDecryptionError("Invalid versioned session payload".to_string())
    })?;
    let exp = exp_str
        .parse::<u64>()
        .map_err(|e| AuthError::SessionDecryptionError(e.to_string()))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AuthError::SessionDecryptionError(e.to_string()))?
        .as_secs();

    if now > exp {
        return Err(AuthError::SessionExpired);
    }
    user_id_str
        .parse::<i32>()
        .map_err(|e| AuthError::SessionDecryptionError(e.to_string()))
}

/// Extracts the secure session cookie value from the request's Cookie headers.
///
/// Unrelated cookies without `=` or with non-ASCII bytes are ignored. A
/// duplicate, empty, oversized or non-graphic `rullst_session` fails closed.
pub fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    let mut session = None;

    for header in headers.get_all(axum::http::header::COOKIE) {
        for cookie in header.as_bytes().split(|byte| *byte == b';') {
            let mut parts = cookie.trim_ascii().splitn(2, |byte| *byte == b'=');
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                continue;
            };
            if name != b"rullst_session" {
                continue;
            }

            if session.is_some()
                || value.is_empty()
                || value.len() > MAX_SESSION_COOKIE_BYTES
                || !value.iter().all(u8::is_ascii_graphic)
            {
                return None;
            }
            session = Some(std::str::from_utf8(value).ok()?.to_owned());
        }
    }

    session
}

/// Generates the standard HTTP header string to set the encrypted session cookie on the client.
#[cfg_attr(mutants, mutants::skip)]
pub fn make_login_cookie(user_id: i32) -> Result<String, AuthError> {
    let app_key = get_app_key()?;
    let encrypted = encrypt_session(user_id, &app_key)?;
    // Set a HttpOnly, Secure (if not local), SameSite=Lax cookie valid for 30 days
    let secure_attr = if detect_environment()?.requires_secure_defaults() {
        "; Secure"
    } else {
        ""
    };
    Ok(format!(
        "rullst_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000{}",
        encrypted, secure_attr
    ))
}

/// Generates the standard HTTP header string to delete/clear the session cookie on the client.
pub fn make_logout_cookie() -> String {
    let requires_secure_defaults = detect_environment()
        .map(rullst_core::config::Environment::requires_secure_defaults)
        .unwrap_or(true);
    logout_cookie_value(requires_secure_defaults).to_owned()
}

pub(super) fn logout_cookie_value(requires_secure_defaults: bool) -> &'static str {
    if requires_secure_defaults {
        SECURE_LOGOUT_COOKIE
    } else {
        LOGOUT_COOKIE
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_make_logout_cookie_invariants() {
        let requires_secure_defaults: bool = kani::any();
        let cookie = logout_cookie_value(requires_secure_defaults);
        assert!(!cookie.is_empty());
        assert_eq!(cookie.as_bytes()[0], b'r');
        if requires_secure_defaults {
            assert_eq!(cookie.len(), SECURE_LOGOUT_COOKIE.len());
            assert_eq!(cookie.as_bytes()[cookie.len() - 1], b'e');
        } else {
            assert_eq!(cookie.len(), LOGOUT_COOKIE.len());
            assert_eq!(cookie.as_bytes()[cookie.len() - 1], b'T');
        }
    }
}
