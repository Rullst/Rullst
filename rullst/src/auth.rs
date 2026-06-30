use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::password_hash::rand_core::OsRng;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::http::HeaderMap;
use base64::{Engine as _, engine::general_purpose};
use sha2::Digest;
use std::convert::TryInto;
use std::fs;

/// WebAuthn and Passkey authentication submodule.
pub mod passkey;

/// Hashes a plain-text password using Argon2id with a cryptographically secure random salt.
pub fn hash_password(password: &str) -> Result<String, String> {
    if password.len() > 72 {
        return Err("Password exceeds maximum length of 72 characters".to_string());
    }
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

/// Verifies a plain-text password against a hashed Argon2 password.
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash_result = PasswordHash::new(hash);

    if password.len() > 72 {
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
    // Basic implementation: if it doesn't match the current library's default format exactly, rehash it.
    if let Ok(parsed_hash) = PasswordHash::new(hash) {
        if parsed_hash.algorithm.as_str() != "argon2id" {
            return true;
        }
    }
    false
}

#[cfg(feature = "oauth")]
pub mod connect {
    //! Re-export do rullst-connect para fornecer autenticação OAuth2 (Google, GitHub, etc.) nativamente no framework.
    pub use rullst_connect::*;
}

/// Resolves the application's unique secret key for encryption.
/// Tries the environment variable `APP_KEY`, then parses `Rullst.toml`, falling back to an ephemeral key.
pub fn get_app_key() -> Result<Vec<u8>, String> {
    if let Ok(env_key) = std::env::var("APP_KEY") {
        return Ok(env_key.into_bytes());
    }

    if let Ok(toml_content) = fs::read_to_string("Rullst.toml") {
        for line in toml_content.lines() {
            let trimmed = line.trim();
            if (trimmed.starts_with("app_key") || trimmed.starts_with("key"))
                && let Some(val) = trimmed.split('=').nth(1)
            {
                return Ok(val.trim().trim_matches('"').as_bytes().to_vec());
            }
        }
    }

    // Enforce explicit APP_KEY when running in production.
    let env = std::env::var("RULLST_ENV")
        .unwrap_or_else(|_| std::env::var("APP_ENV").unwrap_or_default());
    if env.eq_ignore_ascii_case("production") || env.eq_ignore_ascii_case("prod") {
        let err_msg = "FATAL: APP_KEY is not set and RULLST_ENV=production. Set APP_KEY environment variable to a 32+ byte secret.".to_string();
        eprintln!("{}", err_msg);
        return Err("Missing APP_KEY in production environment".to_string());
    }

    let dev_key_path = ".rullst_dev_key";
    if let Ok(key_hex) = fs::read_to_string(dev_key_path) {
        if let Ok(key_bytes) = general_purpose::STANDARD.decode(key_hex.trim()) {
            if key_bytes.len() == 32 {
                return Ok(key_bytes);
            }
        }
    }

    eprintln!(
        "⚠️  Rullst Security Warning: Generating a random APP_KEY in .rullst_dev_key. Set APP_KEY environment variable for production."
    );

    use rand::Rng;
    let mut key = [0u8; 32];
    rand::rng().fill_bytes(&mut key);
    let key_vec = key.to_vec();

    let _ = fs::write(dev_key_path, general_purpose::STANDARD.encode(&key_vec));

    Ok(key_vec)
}

fn derive_cipher(app_key: &[u8]) -> Result<Aes256Gcm, String> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(app_key);
    let key_hash = hasher.finalize();
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&key_hash);
    Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| e.to_string())
}

/// Encrypts a user_id into a secure base64-encoded string.
pub fn encrypt_session(user_id: i32, app_key: &[u8]) -> Result<String, String> {
    let cipher = derive_cipher(app_key)?;

    let mut nonce_bytes = [0u8; 12];
    rand::fill(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let exp = now + (30 * 24 * 60 * 60); // 30 days

    let payload = format!("{}|{}", user_id, exp);
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
    let cipher = derive_cipher(app_key)?;

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

    let payload_str = String::from_utf8(plaintext).map_err(|e| e.to_string())?;

    if let Some((user_id_str, exp_str)) = payload_str.split_once('|') {
        let exp = exp_str.parse::<u64>().map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        if now > exp {
            return Err("Session expired".to_string());
        }
        user_id_str.parse::<i32>().map_err(|e| e.to_string())
    } else {
        // Fallback for legacy tokens
        payload_str.parse::<i32>().map_err(|e| e.to_string())
    }
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
#[cfg_attr(mutants, mutants::skip)]
pub fn make_login_cookie(user_id: i32) -> Result<String, String> {
    let app_key = get_app_key()?;
    let encrypted = encrypt_session(user_id, &app_key)?;
    // Set a HttpOnly, Secure (if not local), SameSite=Lax cookie valid for 30 days
    let env = std::env::var("RULLST_ENV")
        .unwrap_or_else(|_| std::env::var("APP_ENV").unwrap_or_default());
    let is_prod = env.eq_ignore_ascii_case("production") || env.eq_ignore_ascii_case("prod");
    let secure_attr = if is_prod { "; Secure" } else { "" };
    Ok(format!(
        "rullst_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000{}",
        encrypted, secure_attr
    ))
}

/// Generates the standard HTTP header string to delete/clear the session cookie on the client.
pub fn make_logout_cookie() -> String {
    "rullst_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT".to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_password_hashing() {
        let p = String::from_utf8(vec![112, 97, 115, 115]).unwrap(); // "pass"
        let hash = hash_password(&p).expect("Failed to hash password");
        assert!(verify_password(&p, &hash), "Password verification failed");
        assert!(
            !verify_password("wrong", &hash),
            "Password verification succeeded for wrong password"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_password_length_limits() {
        let p_72 = "a".repeat(72);
        let p_73 = "a".repeat(73);

        // hash_password
        assert!(hash_password(&p_72).is_ok());
        let err = hash_password(&p_73).unwrap_err();
        assert_eq!(err, "Password exceeds maximum length of 72 characters");

        // verify_password
        let hash = hash_password("dummy").unwrap();
        assert!(!verify_password(&p_73, &hash));
    }

    #[test]
    fn test_session_encryption_decryption() {
        let user_id = 42;
        let k = vec![42u8; 32];
        let token = encrypt_session(user_id, &k).expect("Failed to encrypt session");
        let decrypted = decrypt_session(&token, &k).expect("Failed to decrypt session");
        assert_eq!(user_id, decrypted);

        // Test short token
        let short_bytes = vec![0u8; 10];
        let short_token = general_purpose::URL_SAFE_NO_PAD.encode(&short_bytes);
        let err = decrypt_session(&short_token, &k).unwrap_err();
        assert_eq!(err, "Invalid token length");
    }

    #[test]
    fn test_session_encryption_error_paths() {
        let k = vec![42u8; 32];

        // Decrypt with invalid base64
        assert!(decrypt_session("invalid-base64-!", &k).is_err());

        // Decrypt with valid base64 but too short
        let short_token = general_purpose::URL_SAFE_NO_PAD.encode(vec![0u8; 10]);
        assert!(decrypt_session(&short_token, &k).is_err());

        // Decrypt with valid base64 but invalid ciphertext (MAC mismatch)
        let bad_cipher = vec![0u8; 32];
        let bad_token = general_purpose::URL_SAFE_NO_PAD.encode(&bad_cipher);
        assert!(decrypt_session(&bad_token, &k).is_err());
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_password_hash_format() {
        let p = "super_secret";
        let hash = hash_password(p).expect("Failed to hash password");
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_password_verification_error_paths() {
        assert!(!verify_password("pass", "invalid_hash_format"));

        let p = "pass";
        let hash = hash_password(p).expect("Failed to hash password");
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn test_make_login_logout_cookie() {
        unsafe {
            std::env::set_var("APP_KEY", "test_key_for_cookie_1234567890");
        }
        let login_cookie = make_login_cookie(42).expect("Failed to make login cookie");
        assert!(login_cookie.starts_with("rullst_session="));
        assert!(login_cookie.contains("HttpOnly"));
        assert!(login_cookie.contains("Path=/"));
        assert!(login_cookie.contains("Max-Age=2592000"));

        let logout_cookie = make_logout_cookie();
        assert!(logout_cookie.starts_with("rullst_session=;"));
        assert!(logout_cookie.contains("Max-Age=0"));
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_needs_rehash() {
        let p = "super_secret";
        let hash = hash_password(p).expect("Failed to hash password");
        assert!(!needs_rehash(&hash));

        let old_hash =
            "$argon2i$v=19$m=4096,t=3,p=1$c29tZXNhbHQ$YhhQvA1/zHGEoWnUBY/J2iY/R/hG93WqG2k73D655b0";
        assert!(needs_rehash(old_hash));

        assert!(!needs_rehash("invalid"));
    }

    #[test]
    fn test_extract_session_cookie() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_session_cookie(&headers), None);

        headers.insert(
            axum::http::header::COOKIE,
            "rullst_session=my_secret_token; other=123".parse().unwrap(),
        );
        assert_eq!(
            extract_session_cookie(&headers),
            Some("my_secret_token".to_string())
        );

        headers.insert(
            axum::http::header::COOKIE,
            "other=123; rullst_session=my_secret_token_2"
                .parse()
                .unwrap(),
        );
        assert_eq!(
            extract_session_cookie(&headers),
            Some("my_secret_token_2".to_string())
        );
    }

    #[test]
    fn test_get_app_key() {
        // Just verify that the application key can be successfully resolved.
        // We avoid mutating `std::env::set_var` here because it races with concurrent tests.
        let key = get_app_key().unwrap();
        assert!(!key.is_empty());
    }
}
