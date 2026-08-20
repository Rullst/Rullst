// tests/integration_test.rs — Comprehensive authentication, session, Argon2 and RBAC tests.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::http::HeaderMap;
use rullst_auth::{
    HasRole, decrypt_session, dummy_verify, encrypt_session, extract_session_cookie,
    hash_password, make_logout_cookie, needs_rehash, verify_password,
};

#[test]
fn test_argon2_password_hashing_and_verification() {
    let password = "SuperSecretPassword#2026";
    let hash = hash_password(password).unwrap();

    assert!(hash.starts_with("$argon2id$"));
    assert!(verify_password(password, &hash));
    assert!(!verify_password("WrongPassword123", &hash));

    // Overly long password rejection (> 72 bytes)
    let long_pass = "A".repeat(80);
    assert!(hash_password(&long_pass).is_err());
    assert!(!verify_password(&long_pass, &hash));

    // Dummy verify should execute without panic
    dummy_verify(None);
    dummy_verify(Some(&hash));

    // Needs rehash check
    assert!(!needs_rehash(&hash));
}

#[test]
fn test_session_encryption_and_decryption() {
    let key = b"my_super_secret_app_key_32bytes_long!";
    let user_id = 42;

    let token = encrypt_session(user_id, key).unwrap();
    assert!(!token.is_empty());

    let decrypted = decrypt_session(&token, key).unwrap();
    assert_eq!(decrypted, user_id);

    // Wrong key should fail
    let bad_key = b"wrong_key_1234567890123456789012!";
    assert!(decrypt_session(&token, bad_key).is_err());

    // Corrupted token should fail
    assert!(decrypt_session("corrupted_base64_payload", key).is_err());
}

#[test]
fn test_session_cookie_helpers() {
    let logout_cookie = make_logout_cookie();
    assert!(logout_cookie.contains("rullst_session="));
    assert!(logout_cookie.contains("Max-Age=0"));

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::COOKIE,
        "theme=dark; rullst_session=my_session_token_123; user=alice"
            .parse()
            .unwrap(),
    );

    let extracted = extract_session_cookie(&headers);
    assert_eq!(extracted, Some("my_session_token_123".to_string()));

    let empty_headers = HeaderMap::new();
    assert_eq!(extract_session_cookie(&empty_headers), None);
}

struct TestUser {
    roles: Vec<String>,
}

impl HasRole for TestUser {
    fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

#[test]
fn test_rbac_has_role() {
    let user = TestUser {
        roles: vec!["editor".to_string(), "author".to_string()],
    };

    assert!(user.has_role("editor"));
    assert!(user.has_role("author"));
    assert!(!user.has_role("admin"));
}
