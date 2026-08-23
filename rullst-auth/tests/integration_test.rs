// tests/integration_test.rs — Comprehensive authentication, session, Argon2 and RBAC tests.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::http::HeaderMap;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand::Rng;
use rullst_auth::{
    HasRole, decrypt_session, dummy_verify, encrypt_session, extract_session_cookie, hash_password,
    hash_password_async, make_logout_cookie, needs_rehash, verify_password, verify_password_async,
};

#[test]
fn test_argon2_password_hashing_and_verification() {
    let mut rng = rand::rng();
    let mut random_bytes = [0u8; 16];
    rng.fill_bytes(&mut random_bytes);
    let password = format!("TestSecret_{}", STANDARD.encode(random_bytes));
    let hash = hash_password(&password).unwrap();

    assert!(hash.starts_with("$argon2id$"));
    assert!(verify_password(&password, &hash));
    let wrong_password = format!("Wrong_{}", STANDARD.encode(random_bytes));
    assert!(!verify_password(&wrong_password, &hash));

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

#[tokio::test]
async fn test_argon2_password_hashing_and_verification_async() {
    let password = "AsyncSuperSecretPassword123!";
    let hash = hash_password_async(password).await.unwrap();

    assert!(hash.starts_with("$argon2id$"));
    assert!(verify_password_async(password, &hash).await);
    assert!(!verify_password_async("WrongPassword456!", &hash).await);
}

#[test]
fn test_session_encryption_and_decryption() {
    let mut rng = rand::rng();
    let mut key = [0u8; 32];
    let mut bad_key = [0u8; 32];
    rng.fill_bytes(&mut key);
    rng.fill_bytes(&mut bad_key);
    let user_id = 42;

    let token = encrypt_session(user_id, &key).unwrap();
    assert!(!token.is_empty());

    let decrypted = decrypt_session(&token, &key).unwrap();
    assert_eq!(decrypted, user_id);

    // Wrong key should fail
    assert!(decrypt_session(&token, &bad_key).is_err());

    // Corrupted token should fail
    assert!(decrypt_session("corrupted_base64_payload", &key).is_err());
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
