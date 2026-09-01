use super::*;
use secrecy::ExposeSecret;

fn state() -> RefreshableTokenState {
    RefreshableTokenState::try_restore(
        "provider-user-42".to_string(),
        "access-secret-marker".to_string(),
        "refresh-secret-marker".to_string(),
        1_000,
        4_600,
        7,
    )
    .expect("valid restored state")
}

fn key(id: &str, byte: u8) -> TokenSnapshotKey {
    TokenSnapshotKey::try_new(id, [byte; 32]).expect("valid snapshot key")
}

fn binding(account: &str) -> TokenSnapshotBinding {
    TokenSnapshotBinding::try_new("github", account).expect("valid binding")
}

#[test]
fn encrypted_snapshot_round_trips_every_validated_field() {
    let original = state();
    let key = key("primary-2026", 7);
    let binding = binding("local-account-9");
    let encrypted = EncryptedTokenSnapshot::seal(&original, &key, &binding)
        .expect("snapshot encryption succeeds");

    assert_eq!(encrypted.key_id().expect("valid envelope"), "primary-2026");
    assert!(!encrypted.as_str().contains("access-secret-marker"));
    assert!(!encrypted.as_str().contains("refresh-secret-marker"));
    assert!(!encrypted.as_str().contains("provider-user-42"));

    let loaded = EncryptedTokenSnapshot::try_from_envelope(encrypted.as_str())
        .expect("stored envelope is valid")
        .open(&key, &binding)
        .expect("snapshot authenticates");
    assert_eq!(loaded.provider_user_id(), "provider-user-42");
    assert_eq!(
        loaded.access_token().expose_secret(),
        "access-secret-marker"
    );
    assert_eq!(
        loaded.refresh_token().expose_secret(),
        "refresh-secret-marker"
    );
    assert_eq!(loaded.issued_at(), 1_000);
    assert_eq!(loaded.expires_at(), 4_600);
    assert_eq!(loaded.generation(), 7);
}

#[test]
fn account_provider_key_and_ciphertext_changes_fail_closed() {
    let encrypted = EncryptedTokenSnapshot::seal(&state(), &key("current", 8), &binding("a"))
        .expect("snapshot encryption succeeds");

    assert!(matches!(
        encrypted.open(&key("current", 9), &binding("a")),
        Err(TokenSnapshotError::AuthenticationFailed)
    ));
    assert!(matches!(
        encrypted.open(&key("previous", 8), &binding("a")),
        Err(TokenSnapshotError::KeyIdMismatch)
    ));
    assert!(matches!(
        encrypted.open(&key("current", 8), &binding("b")),
        Err(TokenSnapshotError::AuthenticationFailed)
    ));
    let other_provider =
        TokenSnapshotBinding::try_new("google", "a").expect("valid alternate provider");
    assert!(matches!(
        encrypted.open(&key("current", 8), &other_provider),
        Err(TokenSnapshotError::AuthenticationFailed)
    ));

    let mut tampered = encrypted.as_str().to_string();
    let ciphertext_start = tampered.rfind(':').expect("ciphertext separator") + 1;
    let replacement = if tampered.as_bytes()[ciphertext_start] == b'A' {
        "B"
    } else {
        "A"
    };
    tampered.replace_range(ciphertext_start..=ciphertext_start, replacement);
    let tampered = EncryptedTokenSnapshot::try_from_envelope(tampered)
        .expect("tampering preserved envelope shape");
    assert!(matches!(
        tampered.open(&key("current", 8), &binding("a")),
        Err(TokenSnapshotError::AuthenticationFailed)
    ));
}

#[test]
fn malformed_inputs_are_bounded_and_debug_output_is_redacted() {
    assert!(TokenSnapshotKey::try_new("bad:key", [1; 32]).is_err());
    assert!(TokenSnapshotBinding::try_new("github", " padded ").is_err());
    assert!(TokenSnapshotBinding::try_new("github\n", "account").is_err());
    assert_eq!(
        EncryptedTokenSnapshot::try_from_envelope("RULLST-CONNECT:v2:key:AA:AAAA"),
        Err(TokenSnapshotError::UnsupportedVersion)
    );
    assert!(EncryptedTokenSnapshot::try_from_envelope("invalid").is_err());
    assert!(EncryptedTokenSnapshot::try_from_envelope("x".repeat(MAX_ENVELOPE_BYTES + 1)).is_err());

    let key = key("redacted", 3);
    let binding = binding("sensitive-account");
    let encrypted = EncryptedTokenSnapshot::seal(&state(), &key, &binding)
        .expect("snapshot encryption succeeds");
    assert_eq!(
        format!("{key:?}"),
        "TokenSnapshotKey { key_id: \"redacted\", key: \"[REDACTED]\" }"
    );
    assert!(!format!("{binding:?}").contains("sensitive-account"));
    assert!(!format!("{encrypted:?}").contains(encrypted.as_str()));
}

#[test]
fn restored_state_rejects_invalid_lifetimes_and_tokens() {
    assert!(
        RefreshableTokenState::try_restore(
            "provider-user".to_string(),
            "access".to_string(),
            "refresh".to_string(),
            10,
            10,
            0,
        )
        .is_err()
    );
    assert!(
        RefreshableTokenState::try_restore(
            "provider-user".to_string(),
            "".to_string(),
            "refresh".to_string(),
            10,
            20,
            0,
        )
        .is_err()
    );
}
