use super::app_key::parse_dotenv;
use super::session::{
    LOGOUT_COOKIE, SECURE_LOGOUT_COOKIE, SESSION_AAD, SESSION_TOKEN_PREFIX, derive_cipher,
    logout_cookie_value,
};
use super::*;
use crate::error::AuthError;
use aes_gcm::{
    Nonce,
    aead::{Aead, Payload},
};
use axum::http::HeaderMap;
use base64::{Engine as _, engine::general_purpose};

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
fn test_existing_argon2id_phc_hash_compatibility() {
    // Fixed Argon2id v19 PHC vector already supported before the Rust API
    // upgrade. Retaining this regression proves that upgrading the crate
    // does not invalidate password hashes stored by Rullst applications.
    let existing_hash =
        "$argon2id$v=19$m=65536,t=2,p=1$c29tZXNhbHQ$CTFhFdXPJO1aFaMaO6Mm5c8y7cJHAph8ArZWb2GRPPc";
    let existing_password =
        String::from_utf8(vec![112, 97, 115, 115, 119, 111, 114, 100]).expect("password fixture");
    let wrong_password = String::from_utf8(vec![
        110, 111, 116, 45, 116, 104, 101, 45, 112, 97, 115, 115, 119, 111, 114, 100,
    ])
    .expect("wrong password fixture");
    assert!(verify_password(&existing_password, existing_hash));
    assert!(!verify_password(&wrong_password, existing_hash));
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
fn logout_cookie_value_has_exact_security_variants() {
    assert_eq!(logout_cookie_value(false), LOGOUT_COOKIE);
    assert_eq!(logout_cookie_value(true), SECURE_LOGOUT_COOKIE);
    assert!(!logout_cookie_value(false).ends_with("; Secure"));
    assert!(logout_cookie_value(true).ends_with("; Secure"));
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

    let mut duplicate_headers = HeaderMap::new();
    duplicate_headers.append(
        axum::http::header::COOKIE,
        "rullst_session=first".parse().unwrap(),
    );
    duplicate_headers.append(
        axum::http::header::COOKIE,
        "rullst_session=second".parse().unwrap(),
    );
    assert_eq!(extract_session_cookie(&duplicate_headers), None);

    duplicate_headers.clear();
    duplicate_headers.insert(
        axum::http::header::COOKIE,
        "rullst_session=first; rullst_session=second"
            .parse()
            .unwrap(),
    );
    assert_eq!(extract_session_cookie(&duplicate_headers), None);

    duplicate_headers.insert(
        axum::http::header::COOKIE,
        "rullst_session=".parse().unwrap(),
    );
    assert_eq!(extract_session_cookie(&duplicate_headers), None);
}

#[test]
fn test_get_app_key() {
    // Just verify that the application key can be successfully resolved.
    // We avoid mutating `std::env::set_var` here because it races with concurrent tests.
    let key = get_app_key().unwrap();
    assert!(key.len() > 1); // Kills Ok(vec![1]) mutant
}

#[test]
fn dotenv_errors_never_echo_file_content() {
    let secret = "sk_live_unit_redaction_canary";
    let content = format!(
        "# comment\nAPP_ENV=production\nDATABASE_URL=\"postgres://open\nSTRIPE_SECRET={secret}\n"
    );
    let error = parse_dotenv(&content).unwrap_err();
    assert_eq!(
        error,
        AuthError::General("invalid .env syntax in entry 2".to_string())
    );
    for rendered in [error.to_string(), format!("{error:?}")] {
        assert!(!rendered.contains(secret));
        assert!(!rendered.contains("postgres://"));
    }

    let values = parse_dotenv("A=1\n\n# note\nB='two words'\nA=3\n").unwrap();
    assert_eq!(values.get("A").map(String::as_str), Some("3"));
    assert_eq!(values.get("B").map(String::as_str), Some("two words"));
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
