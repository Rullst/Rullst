//! Argon2id password hashing and verification helpers.

use super::MAX_PASSWORD_BYTES;
use crate::error::AuthError;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

/// Hashes a plain-text password using Argon2id with a cryptographically secure random salt.
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    if password.len() > MAX_PASSWORD_BYTES {
        return Err(AuthError::PasswordHashError(
            "Password exceeds maximum length of 72 characters".to_string(),
        ));
    }
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| AuthError::PasswordHashError(e.to_string()))
}

/// Asynchronously hashes a plain-text password using Argon2id offloaded to Tokio's blocking thread pool (`spawn_blocking`).
/// This ensures the Tokio async runtime worker threads are not blocked by CPU-intensive password hashing.
pub async fn hash_password_async(password: impl Into<String>) -> Result<String, AuthError> {
    let password = password.into();
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|e| AuthError::PasswordHashError(format!("spawn_blocking error: {}", e)))?
}

/// Verifies a plain-text password against a hashed Argon2 password.
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash_result = PasswordHash::new(hash);

    if password.len() > MAX_PASSWORD_BYTES {
        dummy_verify(Some(hash));
        return false;
    }

    if let Ok(parsed_hash) = parsed_hash_result {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    } else {
        false
    }
}

/// Asynchronously verifies a plain-text password against a hashed Argon2 password offloaded to Tokio's blocking thread pool (`spawn_blocking`).
/// This prevents CPU-bound verification from stalling concurrent async HTTP request handling.
pub async fn verify_password_async(password: impl Into<String>, hash: impl Into<String>) -> bool {
    let password = password.into();
    let hash = hash.into();
    tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .unwrap_or(false)
}

/// Performs a dummy hash verification to equalize execution time and prevent timing attacks.
/// If a valid hash is provided, it uses it; otherwise, it falls back to a hardcoded dummy hash.
pub fn dummy_verify(hash: Option<&str>) {
    let dummy_hash_str =
        "$argon2id$v=19$m=19456,t=2,p=1$VE9CZ2d5dHVyWldOajNXZA$M0zU6o5hE/R6B+nJ9hX8+A";
    let hash_to_use = hash.unwrap_or(dummy_hash_str);
    if let Ok(parsed_hash) = PasswordHash::new(hash_to_use) {
        let _ = Argon2::default().verify_password("dummy_password".as_bytes(), &parsed_hash);
    }
}

/// Checks if an existing Argon2 password hash needs to be rehashed (e.g. because it was generated with older or weaker parameters).
pub fn needs_rehash(hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return true;
    };

    parsed_hash.algorithm.as_str() != "argon2id"
        || parsed_hash.version != Some(0x13)
        || parsed_hash.params.get_decimal("m") != Some(argon2::Params::DEFAULT_M_COST)
        || parsed_hash.params.get_decimal("t") != Some(argon2::Params::DEFAULT_T_COST)
        || parsed_hash.params.get_decimal("p") != Some(argon2::Params::DEFAULT_P_COST)
}
