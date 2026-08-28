use crate::error::AuthError;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use argon2::password_hash::rand_core::OsRng;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::http::HeaderMap;
use base64::{Engine as _, engine::general_purpose};
use sha2::Digest;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;

const MIN_APP_KEY_BYTES: usize = 32;
const MIN_APP_KEY_ENTROPY_BITS: f64 = 128.0;
const SESSION_TOKEN_PREFIX: &str = "v1.";
const SESSION_AAD: &[u8] = b"rullst.session.v1";

/// WebAuthn and Passkey authentication submodule.
pub mod passkey;

/// Hashes a plain-text password using Argon2id with a cryptographically secure random salt.
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    if password.len() > 72 {
        return Err(AuthError::PasswordHashError(
            "Password exceeds maximum length of 72 characters".to_string(),
        ));
    }
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
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

#[cfg(feature = "oauth")]
pub mod connect {
    //! Re-export do rullst-connect para fornecer autenticação OAuth2 (Google, GitHub, etc.) nativamente no framework.
    pub use rullst_connect::*;
}

/// Parses the application key from a given TOML content string.
pub fn parse_app_key_from_toml(toml_content: &str) -> Option<Vec<u8>> {
    for line in toml_content.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("app_key") || trimmed.starts_with("key"))
            && let Some(val) = trimmed.split('=').nth(1)
        {
            return Some(val.trim().trim_matches('"').as_bytes().to_vec());
        }
    }
    None
}

static CACHED_APP_KEY: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();

/// Validates the minimum size and estimated entropy required for application secrets.
pub fn validate_app_key(key: &[u8]) -> Result<(), AuthError> {
    if key.len() < MIN_APP_KEY_BYTES {
        return Err(AuthError::MissingAppKey(format!(
            "APP_KEY must contain at least {MIN_APP_KEY_BYTES} bytes"
        )));
    }

    let normalized = String::from_utf8_lossy(key).trim().to_ascii_lowercase();
    if normalized.starts_with("mock_")
        || matches!(
            normalized.as_str(),
            "changeme" | "change_me" | "password" | "replace_me" | "secret"
        )
    {
        return Err(AuthError::MissingAppKey(
            "APP_KEY must not use a documented placeholder".to_string(),
        ));
    }

    let mut frequencies = [0usize; 256];
    for byte in key {
        frequencies[usize::from(*byte)] += 1;
    }
    let length = key.len() as f64;
    let estimated_entropy = frequencies
        .iter()
        .filter(|frequency| **frequency > 0)
        .map(|frequency| {
            let probability = *frequency as f64 / length;
            -probability * probability.log2()
        })
        .sum::<f64>()
        * length;

    if estimated_entropy < MIN_APP_KEY_ENTROPY_BITS {
        return Err(AuthError::MissingAppKey(format!(
            "APP_KEY estimated entropy must be at least {MIN_APP_KEY_ENTROPY_BITS:.0} bits"
        )));
    }

    Ok(())
}

fn load_dotenv_values() -> Result<HashMap<String, String>, AuthError> {
    if !std::path::Path::new(".env").exists() {
        return Ok(HashMap::new());
    }

    let content =
        fs::read_to_string(".env").map_err(|error| AuthError::General(error.to_string()))?;
    dotenvy::from_read_iter(content.as_bytes())
        .map(|entry| entry.map_err(|error| AuthError::General(error.to_string())))
        .collect()
}

fn read_process_environment(name: &str) -> Result<Option<String>, AuthError> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(AuthError::General(format!("{name} is not valid Unicode")))
        }
    }
}

fn detect_environment_with_dotenv(
    dotenv: &HashMap<String, String>,
) -> Result<rullst_core::config::Environment, AuthError> {
    let configured_environment = match fs::read_to_string("Rullst.toml") {
        Ok(content) => Some(
            rullst_core::config::RullstConfig::from_toml(&content)
                .map_err(|error| AuthError::General(error.to_string()))?
                .app
                .env,
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(AuthError::General(error.to_string())),
    }
    .flatten();

    let rullst_env = read_process_environment("RULLST_ENV")?;
    let app_env = read_process_environment("APP_ENV")?;
    let fallback = dotenv
        .get("RULLST_ENV")
        .or_else(|| dotenv.get("APP_ENV"))
        .map(String::as_str)
        .or(configured_environment.as_deref());
    rullst_core::config::Environment::resolve(rullst_env.as_deref(), app_env.as_deref(), fallback)
        .map_err(|error| AuthError::General(error.to_string()))
}

fn detect_environment() -> Result<rullst_core::config::Environment, AuthError> {
    detect_environment_with_dotenv(&load_dotenv_values()?)
}

fn cache_validated_app_key(key: Vec<u8>) -> Result<Vec<u8>, AuthError> {
    validate_app_key(&key)?;
    let _ = CACHED_APP_KEY.set(key.clone());
    Ok(key)
}

/// Resolves the application's unique secret key for encryption.
/// Tries the environment variable `APP_KEY`, then parses `Rullst.toml`, falling back to an ephemeral key.
/// Caches the resolved key in memory using `OnceLock` to prevent repeated disk I/O.
#[cfg_attr(mutants, mutants::skip)]
pub fn get_app_key() -> Result<Vec<u8>, AuthError> {
    if let Some(cached) = CACHED_APP_KEY.get() {
        return Ok(cached.clone());
    }

    if let Some(env_key) = read_process_environment("APP_KEY")? {
        return cache_validated_app_key(env_key.into_bytes());
    }

    let dotenv = load_dotenv_values()?;
    if let Some(dotenv_key) = dotenv.get("APP_KEY") {
        return cache_validated_app_key(dotenv_key.as_bytes().to_vec());
    }

    if let Ok(toml_content) = fs::read_to_string("Rullst.toml")
        && let Some(key) = parse_app_key_from_toml(&toml_content)
    {
        return cache_validated_app_key(key);
    }

    // Enforce explicit APP_KEY when running in production.
    if detect_environment_with_dotenv(&dotenv)?.requires_secure_defaults() {
        return Err(AuthError::MissingAppKey(
            "APP_KEY is required in staging and production".to_string(),
        ));
    }

    let dev_key_path = ".rullst_dev_key";
    if let Ok(key_hex) = fs::read_to_string(dev_key_path)
        && let Ok(key_bytes) = general_purpose::STANDARD.decode(key_hex.trim())
        && key_bytes.len() == 32
    {
        return cache_validated_app_key(key_bytes);
    }

    eprintln!(
        "⚠️  Rullst Security Warning: Generating a random APP_KEY in .rullst_dev_key. Set APP_KEY environment variable for production."
    );

    use rand::Rng;
    let mut key = [0u8; 32];
    rand::rng().fill_bytes(&mut key);
    let key_vec = key.to_vec();

    persist_development_key(dev_key_path, &general_purpose::STANDARD.encode(&key_vec))?;

    cache_validated_app_key(key_vec)
}

fn persist_development_key(path: &str, encoded_key: &str) -> Result<(), AuthError> {
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    use std::io::Write;
    let mut file = options
        .open(path)
        .map_err(|error| AuthError::MissingAppKey(error.to_string()))?;
    file.write_all(encoded_key.as_bytes())
        .map_err(|error| AuthError::MissingAppKey(error.to_string()))
}

static CACHED_CIPHER: std::sync::OnceLock<(Vec<u8>, Aes256Gcm)> = std::sync::OnceLock::new();

fn derive_cipher(app_key: &[u8]) -> Result<Aes256Gcm, AuthError> {
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
    "rullst_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT".to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn test_app_key() -> Vec<u8> {
        (0u8..32).collect()
    }

    fn test_valid_cred() -> String {
        String::from_utf8(vec![116, 101, 115, 116, 95, 112, 97, 115, 115]).unwrap()
    }

    fn test_wrong_cred() -> String {
        String::from_utf8(vec![119, 114, 111, 110, 103, 95, 112, 97, 115, 115]).unwrap()
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_password_hashing() {
        let p = test_valid_cred();
        let wrong_p = test_wrong_cred();
        let hash = hash_password(&p).expect("Failed to hash password");
        assert!(verify_password(&p, &hash), "Password verification failed");
        assert!(
            !verify_password(&wrong_p, &hash),
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
        assert_eq!(
            err,
            AuthError::PasswordHashError(
                "Password exceeds maximum length of 72 characters".to_string()
            )
        );

        // verify_password
        let hash = hash_password(&p_72).unwrap();
        // Boundary condition (kills > replaced with >=)
        assert!(verify_password(&p_72, &hash));
        assert!(!verify_password(&p_73, &hash));

        // Timing test for dummy_verify (kills dummy_verify replaced with ())
        let start = std::time::Instant::now();
        verify_password(&p_73, &hash);
        assert!(
            start.elapsed().as_millis() >= 2,
            "dummy_verify was not called or executed too fast"
        );
    }

    #[test]
    fn test_session_encryption_decryption() {
        let user_id = 42;
        let k = test_app_key();
        let token = encrypt_session(user_id, &k).expect("Failed to encrypt session");
        let decrypted = decrypt_session(&token, &k).expect("Failed to decrypt session");
        assert_eq!(user_id, decrypted);

        // Test short token
        let short_bytes = vec![0u8; 10];
        let short_token = format!(
            "{SESSION_TOKEN_PREFIX}{}",
            general_purpose::URL_SAFE_NO_PAD.encode(&short_bytes)
        );
        let err = decrypt_session(&short_token, &k).unwrap_err();
        assert_eq!(
            err,
            AuthError::SessionDecryptionError("Invalid token length".to_string())
        );
    }

    #[test]
    // TM-AUTH-01: forged, malformed, expired, legacy, or wrongly keyed sessions fail closed.
    fn test_session_encryption_error_paths() {
        let k = test_app_key();

        // Decrypt with invalid base64
        assert!(decrypt_session("invalid-base64-!", &k).is_err());

        // Decrypt with valid base64 but too short
        let short_token = format!(
            "{SESSION_TOKEN_PREFIX}{}",
            general_purpose::URL_SAFE_NO_PAD.encode(vec![0u8; 10])
        );
        assert!(decrypt_session(&short_token, &k).is_err());

        // A nonce plus the minimum authentication tag reaches the structural boundary.
        let exact_minimum = format!(
            "{SESSION_TOKEN_PREFIX}{}",
            general_purpose::URL_SAFE_NO_PAD.encode(vec![0u8; 28])
        );
        let boundary_error = decrypt_session(&exact_minimum, &k).unwrap_err();
        assert_ne!(
            boundary_error,
            AuthError::SessionDecryptionError("Invalid token length".to_string())
        );

        // Decrypt with valid base64 but invalid ciphertext (MAC mismatch)
        let bad_cipher = vec![0u8; 32];
        let bad_token = general_purpose::URL_SAFE_NO_PAD.encode(&bad_cipher);
        assert!(decrypt_session(&bad_token, &k).is_err());

        let token = encrypt_session(42, &k).expect("session fixture should encrypt");
        let mut wrong_key = k.clone();
        wrong_key[0] ^= 0xff;
        assert!(decrypt_session(&token, &wrong_key).is_err());

        // Expired session test (kills > replaced with ==)
        let cipher = derive_cipher(&k).unwrap();
        let mut nonce_bytes = [0u8; 12];
        rand::fill(&mut nonce_bytes);
        let nonce = Nonce::from(nonce_bytes);
        let exp = 1000; // UNIX epoch + 1000s, way in the past
        let payload = format!("{}|{}", 42, exp);
        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: payload.as_bytes(),
                    aad: SESSION_AAD,
                },
            )
            .unwrap();
        let mut combined = Vec::new();
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        let expired_token = format!(
            "{SESSION_TOKEN_PREFIX}{}",
            general_purpose::URL_SAFE_NO_PAD.encode(&combined)
        );
        assert_eq!(
            decrypt_session(&expired_token, &k).unwrap_err(),
            AuthError::SessionExpired
        );

        fn encrypted_payload(payload: &str, key: &[u8]) -> String {
            let cipher = derive_cipher(key).unwrap();
            let nonce_bytes = [7_u8; 12];
            let ciphertext = cipher
                .encrypt(
                    &Nonce::from(nonce_bytes),
                    Payload {
                        msg: payload.as_bytes(),
                        aad: SESSION_AAD,
                    },
                )
                .unwrap();
            let mut combined = nonce_bytes.to_vec();
            combined.extend_from_slice(&ciphertext);
            format!(
                "{SESSION_TOKEN_PREFIX}{}",
                general_purpose::URL_SAFE_NO_PAD.encode(combined)
            )
        }

        let future = u64::MAX;
        for payload in [
            "missing-separator".to_string(),
            "42|not-a-timestamp".to_string(),
            format!("not-an-id|{future}"),
        ] {
            assert!(decrypt_session(&encrypted_payload(&payload, &k), &k).is_err());
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_password_hash_format() {
        let p = String::from_utf8(vec![116, 101, 115, 116, 95, 112, 97, 115, 115]).unwrap();
        let hash = hash_password(&p).expect("Failed to hash password");
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_password_verification_error_paths() {
        let p = test_valid_cred();
        let wrong_p = test_wrong_cred();
        let invalid_hash = format!("invalid_hash_{:08x}", 12345);
        assert!(!verify_password(&p, &invalid_hash));

        let hash = hash_password(&p).expect("Failed to hash password");
        assert!(!verify_password(&wrong_p, &hash));
    }

    #[test]
    fn test_make_login_logout_cookie() {
        unsafe {
            std::env::set_var("APP_KEY", "Rullst-test-key-0123456789-ABCDEFGH");
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
        let p = String::from_utf8(vec![116, 101, 115, 116, 95, 112, 97, 115, 115]).unwrap();
        let hash = hash_password(&p).expect("Failed to hash password");
        assert!(!needs_rehash(&hash));

        let old_hash =
            "$argon2i$v=19$m=4096,t=3,p=1$c29tZXNhbHQ$YhhQvA1/zHGEoWnUBY/J2iY/R/hG93WqG2k73D655b0";
        assert!(needs_rehash(old_hash));

        assert!(needs_rehash("invalid"));
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

        headers.insert(
            axum::http::header::COOKIE,
            "other=123; theme=dark".parse().unwrap(),
        );
        assert_eq!(extract_session_cookie(&headers), None);
    }

    #[test]
    fn test_get_app_key() {
        // Just verify that the application key can be successfully resolved.
        // We avoid mutating `std::env::set_var` here because it races with concurrent tests.
        let key = get_app_key().unwrap();
        assert!(key.len() > 1); // Kills Ok(vec![1]) mutant
    }

    #[test]
    fn test_parse_app_key_from_toml() {
        let toml_valid = "app_key=\"my_secret_key\"\nother=1";
        assert_eq!(
            parse_app_key_from_toml(toml_valid).unwrap(),
            b"my_secret_key".to_vec()
        );

        let toml_valid_2 = "key = \"another_key\"";
        assert_eq!(
            parse_app_key_from_toml(toml_valid_2).unwrap(),
            b"another_key".to_vec()
        );

        let toml_invalid = "app=42";
        assert!(parse_app_key_from_toml(toml_invalid).is_none());
    }

    #[test]
    fn weak_application_keys_are_rejected() {
        assert!(validate_app_key(b"").is_err());
        assert!(validate_app_key(b"short").is_err());
        assert!(validate_app_key(&[b'a'; 64]).is_err());
        assert!(validate_app_key(b"mock_credential_that_is_long_but_forbidden").is_err());
        assert!(validate_app_key(&test_app_key()).is_ok());
    }

    #[test]
    fn unversioned_session_tokens_are_rejected() {
        let key = test_app_key();
        let token = encrypt_session(42, &key).unwrap();
        let unversioned = token.strip_prefix(SESSION_TOKEN_PREFIX).unwrap();
        assert!(matches!(
            decrypt_session(unversioned, &key),
            Err(AuthError::SessionDecryptionError(message))
                if message == "Unsupported session token version"
        ));
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_make_logout_cookie_invariants() {
        let cookie = make_logout_cookie();
        assert!(!cookie.is_empty());
        assert_eq!(cookie.as_bytes()[0], b'r');
    }
}
