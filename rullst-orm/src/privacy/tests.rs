use super::*;

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
