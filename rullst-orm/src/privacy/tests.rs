#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::collections::BTreeMap;

struct EnvironmentGuard {
    values: BTreeMap<&'static str, Option<String>>,
}

impl EnvironmentGuard {
    fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    fn remember(&mut self, key: &'static str) {
        self.values
            .entry(key)
            .or_insert_with(|| std::env::var(key).ok());
    }

    fn set(&mut self, key: &'static str, value: &str) {
        self.remember(key);
        unsafe { std::env::set_var(key, value) };
    }

    fn clear(&mut self, key: &'static str) {
        self.remember(key);
        unsafe { std::env::remove_var(key) };
    }
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        for (key, value) in &self.values {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

fn generate_test_key_32() -> String {
    let mut bytes = [0u8; 16];
    rand::fill(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn generate_short_key() -> String {
    let mut bytes = [0u8; 4];
    rand::fill(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn secret_string_encryption_round_trips() {
    let key = generate_test_key_32();
    let encrypted = encrypt_aes_gcm("Sensitive Data 123", &key).expect("encryption should work");
    assert_ne!(encrypted, "Sensitive Data 123");
    assert_eq!(
        decrypt_aes_gcm(&encrypted, &key).expect("decryption should work"),
        "Sensitive Data 123"
    );
}

#[test]
fn secret_string_redacts_debug_and_reveals_explicitly() {
    let secret = SecretString::new("my-cpf-123");
    assert_eq!(format!("{secret:?}"), "[ENCRYPTED_SECRET]");
    assert_eq!(secret.reveal_audited(), "my-cpf-123");
}

#[test]
fn legacy_payload_requires_nonce_and_authentication_tag() {
    let key = generate_test_key_32();
    for length in [11, 12, NONCE_LENGTH + TAG_LENGTH - 1] {
        let payload = STANDARD.encode(vec![0u8; length]);
        assert_eq!(
            decrypt_aes_gcm(&payload, &key).expect_err("short payload must fail"),
            PrivacyError::PayloadTooShort
        );
    }
}

#[test]
fn versioned_envelope_authenticates_context_and_key_id() {
    let key = generate_test_key_32();
    let context = model_context("accounts", "api_token");
    let encrypted = encrypt_with_context("sensitive", key.as_bytes(), "key-2026", &context)
        .expect("versioned encryption should succeed");

    assert!(encrypted.starts_with("RULLST:v2:key-2026:"));
    let envelope = parse_envelope(&encrypted).expect("envelope should parse");
    assert_eq!(envelope.key_id, "key-2026");
    assert_eq!(
        decrypt_envelope(&envelope, key.as_bytes(), &context)
            .expect("matching context should decrypt"),
        "sensitive"
    );
    assert!(matches!(
        decrypt_envelope(
            &envelope,
            key.as_bytes(),
            &model_context("accounts", "different_column")
        ),
        Err(PrivacyError::DecryptionFailed(_))
    ));
}

#[test]
fn versioned_envelope_rejects_invalid_and_future_formats() {
    assert_eq!(
        parse_envelope("RULLST:v2:bad:key:id:extra")
            .err()
            .expect("extra envelope fields must fail"),
        PrivacyError::InvalidEnvelope
    );
    assert_eq!(
        parse_envelope("RULLST:v3:default:AA:AA")
            .err()
            .expect("future envelope versions must fail"),
        PrivacyError::UnsupportedEnvelopeVersion("v3".to_string())
    );
    assert_eq!(
        parse_envelope("ENC:v2:default:AA:AA")
            .err()
            .expect("foreign envelope prefixes must fail"),
        PrivacyError::InvalidEnvelope
    );

    for malformed in [
        "RULLST",
        "RULLST:v2",
        "RULLST:v2:default",
        "RULLST:v2:default:AA",
        "RULLST:v2::AA:AA",
        "RULLST:v2:bad/key:AA:AA",
    ] {
        assert!(parse_envelope(malformed).is_err(), "{malformed}");
    }

    let tag = URL_SAFE_NO_PAD.encode([0_u8; TAG_LENGTH]);
    assert!(matches!(
        parse_envelope(&format!("RULLST:v2:default:not-base64!:{tag}")),
        Err(PrivacyError::Base64Error(_))
    ));
    assert_eq!(
        parse_envelope(&format!("RULLST:v2:default:AA:{tag}"))
            .err()
            .unwrap(),
        PrivacyError::PayloadTooShort
    );
    let nonce = URL_SAFE_NO_PAD.encode([0_u8; NONCE_LENGTH]);
    assert_eq!(
        parse_envelope(&format!("RULLST:v2:default:{nonce}:AA"))
            .err()
            .unwrap(),
        PrivacyError::PayloadTooShort
    );
}

#[test]
fn configured_keys_accept_raw_base64_and_hex_material() {
    let key = b"0123456789abcdef0123456789abcdef";
    assert_eq!(
        decode_configured_key(std::str::from_utf8(key).expect("ASCII test key"))
            .expect("raw key should decode"),
        key
    );
    assert_eq!(
        decode_configured_key(&format!("base64:{}", STANDARD.encode(key)))
            .expect("base64 key should decode"),
        key
    );
    assert_eq!(
        decode_configured_key(
            "hex:3031323334353637383961626364656630313233343536373839616263646566"
        )
        .expect("hex key should decode"),
        key
    );
    assert_eq!(
        decode_configured_key("hex:not-hex").expect_err("invalid hex must fail"),
        PrivacyError::InvalidKeyEncoding("hex")
    );
    assert_eq!(
        decode_configured_key("base64:!").unwrap_err(),
        PrivacyError::InvalidKeyEncoding("base64")
    );
    assert_eq!(
        decode_configured_key("hex:0").unwrap_err(),
        PrivacyError::InvalidKeyEncoding("hex")
    );
    assert_eq!(
        decode_configured_key("base64:c2hvcnQ=").unwrap_err(),
        PrivacyError::InvalidKeyLength
    );
}

#[test]
fn privacy_errors_and_key_validation_are_typed() {
    let short_key = generate_short_key();
    assert_eq!(
        encrypt_aes_gcm("data", &short_key).expect_err("short key must fail"),
        PrivacyError::InvalidKeyLength
    );
    assert_eq!(
        decrypt_aes_gcm("data", &short_key).expect_err("short key must fail"),
        PrivacyError::InvalidKeyLength
    );

    let key = generate_test_key_32();
    assert!(matches!(
        decrypt_aes_gcm("invalid base64!@#", &key).expect_err("invalid base64 must fail"),
        PrivacyError::Base64Error(_)
    ));
    assert_eq!(
        PrivacyError::InvalidKeyLength.to_string(),
        "RULLST_ENCRYPTION_KEY must be exactly 32 bytes long"
    );
    assert_eq!(
        PrivacyError::PayloadTooShort.to_string(),
        "Invalid encrypted payload (too short)"
    );
    assert!(
        PrivacyError::EncryptionFailed("err".to_string())
            .to_string()
            .contains("Encryption failed")
    );
    assert!(
        PrivacyError::DecryptionFailed("err".to_string())
            .to_string()
            .contains("Decryption failed")
    );
    assert!(
        PrivacyError::Utf8Error("err".to_string())
            .to_string()
            .contains("UTF-8")
    );
    assert!(
        PrivacyError::EnvError("err".to_string())
            .to_string()
            .contains("Environment variable")
    );
}

#[test]
fn legacy_ciphertext_decrypts_and_invalid_utf8_is_typed() {
    let key = b"0123456789abcdef0123456789abcdef";
    let cipher = Aes256Gcm::new_from_slice(key).unwrap();
    let nonce_bytes = [7_u8; NONCE_LENGTH];
    let nonce = Nonce::<Aes256Gcm>::try_from(nonce_bytes.as_slice()).unwrap();

    let ciphertext = cipher.encrypt(&nonce, b"legacy secret".as_slice()).unwrap();
    let mut payload = nonce_bytes.to_vec();
    payload.extend_from_slice(&ciphertext);
    assert_eq!(
        decrypt_aes_gcm(&STANDARD.encode(payload), std::str::from_utf8(key).unwrap()).unwrap(),
        "legacy secret"
    );

    let invalid_utf8 = cipher.encrypt(&nonce, [0xff].as_slice()).unwrap();
    let mut invalid_payload = nonce_bytes.to_vec();
    invalid_payload.extend_from_slice(&invalid_utf8);
    assert!(matches!(
        decrypt_aes_gcm(
            &STANDARD.encode(invalid_payload),
            std::str::from_utf8(key).unwrap()
        ),
        Err(PrivacyError::Utf8Error(_))
    ));
}

#[test]
fn environment_key_selection_rotation_and_context_fail_closed() {
    let mut environment = EnvironmentGuard::new();
    for key in [KEY_ENV, KEY_ID_ENV, KEYRING_ENV] {
        environment.clear(key);
    }

    assert_eq!(current_key_id().unwrap(), DEFAULT_KEY_ID);
    assert!(matches!(current_key(), Err(PrivacyError::EnvError(_))));
    environment.set(KEY_ID_ENV, "bad/id");
    assert_eq!(current_key_id().unwrap_err(), PrivacyError::InvalidKeyId);

    let primary = "0123456789abcdef0123456789abcdef";
    let rotated = "abcdef0123456789abcdef0123456789";
    environment.set(KEY_ID_ENV, "primary-2026");
    environment.set(KEY_ENV, primary);
    assert_eq!(current_key().unwrap(), primary.as_bytes());

    let encrypted = encrypt_model_field("secret", "accounts", "token").unwrap();
    assert_eq!(
        decrypt_model_field(&encrypted, "accounts", "token").unwrap(),
        "secret"
    );
    assert!(decrypt_model_field(&encrypted, "accounts", "other").is_err());

    environment.set(KEY_ID_ENV, "rotated-2027");
    environment.set(KEY_ENV, rotated);
    environment.clear(KEYRING_ENV);
    assert!(matches!(
        decrypt_model_field(&encrypted, "accounts", "token"),
        Err(PrivacyError::EnvError(_))
    ));
    environment.set(KEYRING_ENV, "not-json");
    assert!(matches!(
        decrypt_model_field(&encrypted, "accounts", "token"),
        Err(PrivacyError::EnvError(_))
    ));
    environment.set(KEYRING_ENV, "{}");
    assert_eq!(
        decrypt_model_field(&encrypted, "accounts", "token").unwrap_err(),
        PrivacyError::KeyNotFound("primary-2026".to_string())
    );
    environment.set(KEYRING_ENV, &format!(r#"{{"primary-2026":"{primary}"}}"#));
    assert_eq!(
        decrypt_model_field(&encrypted, "accounts", "token").unwrap(),
        "secret"
    );

    let configured = encrypt_configured_secret("configured").unwrap();
    assert_eq!(
        decrypt_configured_secret(&configured).unwrap(),
        "configured"
    );
}
