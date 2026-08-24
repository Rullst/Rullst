use super::*;

const KEY: &[u8; KEY_LENGTH] = b"0123456789abcdef0123456789abcdef";
const OTHER_KEY: &[u8; KEY_LENGTH] = b"abcdef0123456789abcdef0123456789";
const AAD: &[u8] = b"tenant-7:users:ssn:42";

#[test]
fn test_vault_secret_redaction() {
    let secret = VaultSecret::new("super_secret_password".to_string());
    assert_eq!(format!("{:?}", secret), "VaultSecret(***REDACTED***)");
    assert_eq!(format!("{}", secret), "***REDACTED***");
    assert_eq!(secret.expose_secret(), "super_secret_password");
}

#[test]
fn field_encryptor_round_trips_and_uses_random_nonces() {
    let first = FieldEncryptor::encrypt_with_key_id("secret_data", KEY, "primary", AAD)
        .expect("encryption should succeed");
    let second = FieldEncryptor::encrypt_with_key_id("secret_data", KEY, "primary", AAD)
        .expect("encryption should succeed");

    assert!(first.starts_with("ENC:v2:primary:"));
    assert!(!first.contains("secret_data"));
    assert_ne!(first, second, "a fresh nonce must produce a new envelope");
    assert_eq!(
        FieldEncryptor::decrypt_with_aad(&first, KEY, AAD).expect("decryption should succeed"),
        "secret_data"
    );
    assert_eq!(
        FieldEncryptor::envelope_key_id(&first).expect("envelope should be valid"),
        "primary"
    );
}

#[test]
fn convenience_api_round_trips_empty_and_unicode_fields() {
    for plaintext in ["", "dados confidenciais 🔐"] {
        let encrypted = FieldEncryptor::encrypt(plaintext, KEY).expect("encryption should succeed");
        assert_eq!(
            FieldEncryptor::decrypt(&encrypted, KEY).expect("decryption should succeed"),
            plaintext
        );
    }
}

#[test]
fn rejects_invalid_keys_wrong_keys_and_mismatched_aad() {
    assert_eq!(
        FieldEncryptor::encrypt("secret", b"too-short"),
        Err(VaultError::InvalidKeyLength {
            expected: KEY_LENGTH,
            actual: 9,
        })
    );

    let encrypted =
        FieldEncryptor::encrypt_with_aad("secret", KEY, AAD).expect("encryption should succeed");
    assert_eq!(
        FieldEncryptor::decrypt_with_aad(&encrypted, OTHER_KEY, AAD),
        Err(VaultError::AuthenticationFailed)
    );
    assert_eq!(
        FieldEncryptor::decrypt_with_aad(&encrypted, KEY, b"tenant-8:users:ssn:42"),
        Err(VaultError::AuthenticationFailed)
    );
    assert_eq!(
        FieldEncryptor::decrypt_with_aad(&encrypted, b"too-short", AAD),
        Err(VaultError::InvalidKeyLength {
            expected: KEY_LENGTH,
            actual: 9,
        })
    );
}

#[test]
fn rejects_tampered_ciphertext_and_authenticated_key_id() {
    let encrypted = FieldEncryptor::encrypt_with_key_id("secret", KEY, "primary", AAD)
        .expect("encryption should succeed");
    let mut fields: Vec<String> = encrypted.split(':').map(str::to_string).collect();
    let mut ciphertext = URL_SAFE_NO_PAD
        .decode(&fields[4])
        .expect("test envelope should contain base64 ciphertext");
    ciphertext[0] ^= 1;
    fields[4] = URL_SAFE_NO_PAD.encode(ciphertext);
    let tampered_ciphertext = fields.join(":");

    assert_eq!(
        FieldEncryptor::decrypt_with_aad(&tampered_ciphertext, KEY, AAD),
        Err(VaultError::AuthenticationFailed)
    );

    let tampered_key_id = encrypted.replacen(":primary:", ":secondary:", 1);
    assert_eq!(
        FieldEncryptor::decrypt_with_aad(&tampered_key_id, KEY, AAD),
        Err(VaultError::AuthenticationFailed)
    );
}

#[test]
fn rejects_legacy_unknown_and_malformed_envelopes() {
    assert_eq!(
        FieldEncryptor::decrypt("ENC:v1:irreversible-hash", KEY),
        Err(VaultError::LegacyIrreversibleEnvelope {
            version: "v1".to_string(),
        })
    );
    assert_eq!(
        FieldEncryptor::decrypt("ENC:v99:default:nonce:ciphertext", KEY),
        Err(VaultError::UnsupportedEnvelopeVersion {
            version: "v99".to_string(),
        })
    );
    assert_eq!(
        FieldEncryptor::decrypt("not-an-envelope", KEY),
        Err(VaultError::InvalidEnvelope)
    );

    let short_nonce = URL_SAFE_NO_PAD.encode([0_u8; NONCE_LENGTH - 1]);
    let tag = URL_SAFE_NO_PAD.encode([0_u8; TAG_LENGTH]);
    assert_eq!(
        FieldEncryptor::decrypt(&format!("ENC:v2:default:{short_nonce}:{tag}"), KEY),
        Err(VaultError::InvalidNonceLength {
            expected: NONCE_LENGTH,
            actual: NONCE_LENGTH - 1,
        })
    );
}

#[test]
fn supports_key_rotation_with_envelope_key_ids() {
    let old_envelope = FieldEncryptor::encrypt_with_key_id("old data", KEY, "key-2025", AAD)
        .expect("encryption should succeed");
    let new_envelope = FieldEncryptor::encrypt_with_key_id("new data", OTHER_KEY, "key-2026", AAD)
        .expect("encryption should succeed");
    let keyring: [(&str, &[u8]); 2] = [
        ("key-2026", OTHER_KEY.as_slice()),
        ("key-2025", KEY.as_slice()),
    ];

    assert_eq!(
        FieldEncryptor::decrypt_with_keyring(&old_envelope, &keyring, AAD)
            .expect("old key should remain readable"),
        "old data"
    );
    assert_eq!(
        FieldEncryptor::decrypt_with_keyring(&new_envelope, &keyring, AAD)
            .expect("current key should be readable"),
        "new data"
    );
    assert_eq!(
        FieldEncryptor::decrypt_with_keyring(&new_envelope, &keyring[1..], AAD),
        Err(VaultError::KeyNotFound {
            key_id: "key-2026".to_string(),
        })
    );
}

#[test]
fn validates_rotation_key_identifiers() {
    assert_eq!(
        FieldEncryptor::encrypt_with_key_id("secret", KEY, "", AAD),
        Err(VaultError::EmptyKeyId)
    );
    assert_eq!(
        FieldEncryptor::encrypt_with_key_id("secret", KEY, "key:invalid", AAD),
        Err(VaultError::InvalidKeyId)
    );
}
