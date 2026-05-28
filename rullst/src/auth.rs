use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use axum::http::HeaderMap;
use base64::{Engine as _, engine::general_purpose};
use sha2::Digest;
use std::convert::TryInto;
use std::fs;

pub mod passkey;

use std::sync::OnceLock;

static DEV_APP_KEY: OnceLock<Vec<u8>> = OnceLock::new();

/// Hashes a plain-text password using Argon2id with a cryptographically secure random salt.
pub fn hash_password(password: &str) -> Result<String, String> {
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

/// Verifies a plain-text password against a hashed Argon2 password.
pub fn verify_password(password: &str, hash: &str) -> bool {
    if let Ok(parsed_hash) = PasswordHash::new(hash) {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    } else {
        false
    }
}

/// Resolves the application's unique secret key for encryption.
/// Tries the environment variable `APP_KEY`, then parses `Rullst.toml`, falling back to an ephemeral key.
pub fn get_app_key() -> Vec<u8> {
    if let Ok(env_key) = std::env::var("APP_KEY") {
        return env_key.into_bytes();
    }

    if let Ok(toml_content) = fs::read_to_string("Rullst.toml") {
        for line in toml_content.lines() {
            let trimmed = line.trim();
            if (trimmed.starts_with("app_key") || trimmed.starts_with("key"))
                && let Some(val) = trimmed.split('=').nth(1)
            {
                return val.trim().trim_matches('"').as_bytes().to_vec();
            }
        }
    }

    // Enforce explicit APP_KEY when running in production.
    let env = std::env::var("RULLST_ENV").unwrap_or_default();
    if env.eq_ignore_ascii_case("production") || env.eq_ignore_ascii_case("prod") {
        eprintln!(
            "FATAL: APP_KEY is not set and RULLST_ENV=production. Set APP_KEY environment variable to a 32+ byte secret."
        );
        panic!("Missing APP_KEY in production environment");
    }

    eprintln!(
        "⚠️  Rullst Security Warning: Using an ephemeral random APP_KEY. Sessions will invalidate on restart. Set APP_KEY to avoid this."
    );
    DEV_APP_KEY
        .get_or_init(|| {
            use rand::Rng;
            let mut key = [0u8; 32];
            rand::rng().fill_bytes(&mut key);
            key.to_vec()
        })
        .clone()
}

/// Encrypts a user_id into a secure base64-encoded string.
pub fn encrypt_session(user_id: i32, app_key: &[u8]) -> Result<String, String> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(app_key);
    let key_hash = hasher.finalize();
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&key_hash);

    let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| e.to_string())?;

    let mut nonce_bytes = [0u8; 12];
    rand::fill(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let payload = user_id.to_string();
    let ciphertext = cipher
        .encrypt(&nonce, payload.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut combined = Vec::new();
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(general_purpose::URL_SAFE_NO_PAD.encode(&combined))
}

/// Decrypts a secure base64-encoded string back into a user_id.
pub fn decrypt_session(token: &str, app_key: &[u8]) -> Result<i32, String> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(app_key);
    let key_hash = hasher.finalize();
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&key_hash);

    let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| e.to_string())?;

    let combined = general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|e| e.to_string())?;

    if combined.len() < 12 {
        return Err("Invalid token length".to_string());
    }

    let nonce_bytes: [u8; 12] = combined[..12]
        .try_into()
        .map_err(|_| "Invalid token length".to_string())?;
    let nonce = Nonce::from(nonce_bytes);
    let ciphertext = &combined[12..];

    let plaintext = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| e.to_string())?;

    let user_id_str = String::from_utf8(plaintext).map_err(|e| e.to_string())?;
    user_id_str.parse::<i32>().map_err(|e| e.to_string())
}

/// Extracts the secure session cookie value from the request's Cookie headers.
pub fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie_str| {
            for cookie in cookie_str.split(';') {
                let trimmed = cookie.trim();
                if let Some(stripped) = trimmed.strip_prefix("rullst_session=") {
                    return Some(stripped.to_string());
                }
            }
            None
        })
}

/// Generates the standard HTTP header string to set the encrypted session cookie on the client.
pub fn make_login_cookie(user_id: i32) -> Result<String, String> {
    let app_key = get_app_key();
    let encrypted = encrypt_session(user_id, &app_key)?;
    // Set a HttpOnly, Secure (if not local), SameSite=Lax cookie valid for 30 days
    Ok(format!(
        "rullst_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000",
        encrypted
    ))
}

/// Generates the standard HTTP header string to delete/clear the session cookie on the client.
pub fn make_logout_cookie() -> String {
    "rullst_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "my-secure-password";
        let hash = hash_password(password).expect("Failed to hash password");
        assert!(
            verify_password(password, &hash),
            "Password verification failed"
        );
        assert!(
            !verify_password("wrong-password", &hash),
            "Password verification succeeded for wrong password"
        );
    }

    #[test]
    fn test_session_encryption_decryption() {
        let user_id = 42;
        let key = b"my-custom-encryption-key-for-test!!!";
        let token = encrypt_session(user_id, key).expect("Failed to encrypt session");
        let decrypted = decrypt_session(&token, key).expect("Failed to decrypt session");
        assert_eq!(
            user_id, decrypted,
            "Decrypted user ID does not match original"
        );
    }
}
