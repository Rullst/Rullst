// tests/mfa_sri_vault_test.rs — Comprehensive MFA, SRI & Vault encryption tests.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_security::mfa::{
    build_otpauth_uri, decode_base32, generate_mfa_secret, generate_totp_code, verify_totp_code,
};
use rullst_security::sri::{compute_sri_hash, sri_link_tag, sri_script_tag};
use rullst_security::vault::{FieldEncryptor, VaultSecret};

#[test]
fn test_mfa_secret_generation_and_totp_verification() {
    let secret = generate_mfa_secret();
    assert_eq!(secret.len(), 32);

    let raw_bytes = decode_base32(&secret);
    assert!(raw_bytes.is_some());
    assert!(!raw_bytes.unwrap().is_empty());

    // Invalid base32
    assert_eq!(decode_base32("INVALID!@#$"), None);

    // Generate code
    let code_opt = generate_totp_code(&secret);
    assert!(code_opt.is_some());
    let code = code_opt.unwrap();
    assert_eq!(code.len(), 6);

    // Verify current code
    assert!(verify_totp_code(&secret, &code));
    // Verify completely wrong code
    assert!(!verify_totp_code(&secret, "999999"));

    // Otpauth URI
    let uri = build_otpauth_uri("alice@example.com", "RullstApp", &secret);
    assert!(uri.starts_with("otpauth://totp/"));
    assert!(uri.contains("alice%40example.com") || uri.contains("alice@example.com"));
    assert!(uri.contains(&secret));
}

#[test]
fn test_sri_hash_and_tag_generation() {
    let script_content = b"console.log('Rullst Security SRI');";
    let hash = compute_sri_hash(script_content);
    assert!(hash.starts_with("sha384-"));

    let script_tag = sri_script_tag("/assets/bundle.js", script_content);
    assert!(script_tag.contains("src=\"/assets/bundle.js\""));
    assert!(script_tag.contains(&hash));
    assert!(script_tag.contains("crossorigin=\"anonymous\""));

    let css_content = b"body { background: #020617; }";
    let link_tag = sri_link_tag("/assets/style.css", css_content);
    assert!(link_tag.contains("href=\"/assets/style.css\""));
    assert!(link_tag.contains("rel=\"stylesheet\""));
}

#[test]
fn test_vault_secret_zeroization_and_field_encryptor() {
    let secret_data = vec![1u8, 2, 3, 4, 5];
    let vault = VaultSecret::new(secret_data);
    assert_eq!(vault.expose_secret(), &[1u8, 2, 3, 4, 5]);

    // Debug and Display format redactions
    let debug_str = format!("{:?}", vault);
    assert!(debug_str.contains("***REDACTED***"));

    let display_str = format!("{}", vault);
    assert_eq!(display_str, "***REDACTED***");

    // FieldEncryptor encrypt and decrypt
    let encrypted = FieldEncryptor::encrypt("user_ssn_secret", "master_key_123");
    assert!(encrypted.starts_with("ENC:v1:"));

    let decrypted = FieldEncryptor::decrypt(&encrypted, "master_key_123");
    assert!(decrypted.is_ok());

    let bad_decrypt = FieldEncryptor::decrypt("INVALID_PREFIX_DATA", "key");
    assert!(bad_decrypt.is_err());
}
