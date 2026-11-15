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

/// An envelope of `length` bytes in the stored format, whose ciphertext is
/// never decrypted.
fn crafted_envelope(length: usize) -> String {
    let nonce = URL_SAFE_NO_PAD.encode([0_u8; NONCE_BYTES]);
    let mut key_id = "k".to_string();
    loop {
        let head = format!("{ENVELOPE_PREFIX}:{ENVELOPE_VERSION}:{key_id}:{nonce}:");
        let ciphertext = length - head.len();
        // Unpadded base64 never has a length of 1 modulo 4.
        if ciphertext % 4 != 1 {
            return format!("{head}{}", "A".repeat(ciphertext));
        }
        key_id.push('k');
    }
}

#[test]
fn stored_envelopes_hold_at_most_192_kib_in_five_fields() {
    assert_eq!(
        EncryptedTokenSnapshot::try_from_envelope(""),
        Err(TokenSnapshotError::InvalidEnvelope)
    );
    let longest = crafted_envelope(MAX_ENVELOPE_BYTES);
    assert_eq!(longest.len(), 196_608);
    assert!(EncryptedTokenSnapshot::try_from_envelope(longest).is_ok());
    assert_eq!(
        EncryptedTokenSnapshot::try_from_envelope(crafted_envelope(MAX_ENVELOPE_BYTES + 1)),
        Err(TokenSnapshotError::InvalidEnvelope)
    );
    let nonce = |bytes: usize| URL_SAFE_NO_PAD.encode(vec![0_u8; bytes]);
    let ciphertext = |bytes: usize| URL_SAFE_NO_PAD.encode(vec![0_u8; bytes]);
    let envelope = |nonce: &str, ciphertext: &str, extra: &str| {
        format!("{ENVELOPE_PREFIX}:{ENVELOPE_VERSION}:key:{nonce}:{ciphertext}{extra}")
    };
    // The tag alone is the shortest ciphertext.
    assert!(
        EncryptedTokenSnapshot::try_from_envelope(envelope(&nonce(12), &ciphertext(16), ""))
            .is_ok()
    );
    for refused in [
        envelope(&nonce(12), &ciphertext(15), ""),
        envelope(&nonce(11), &ciphertext(16), ""),
        envelope(&nonce(12), &ciphertext(16), ":extra"),
    ] {
        assert_eq!(
            EncryptedTokenSnapshot::try_from_envelope(refused.as_str()),
            Err(TokenSnapshotError::InvalidEnvelope),
            "{refused}"
        );
    }
}

#[test]
fn a_sealed_envelope_may_reach_exactly_192_kib() {
    // Quotes double in JSON, so two full tokens of them overflow the bound.
    let refresh = "\"".repeat(65_536);
    let sealed = |quotes: usize, key_id: &str| {
        let state = RefreshableTokenState::try_restore(
            "provider-user".to_string(),
            format!("a{}", "\"".repeat(quotes)),
            refresh.clone(),
            1_000,
            4_600,
            0,
        )
        .expect("bounded tokens");
        EncryptedTokenSnapshot::seal(&state, &key(key_id, 5), &binding("account"))
    };
    let length = |quotes: usize| sealed(quotes, "k").map(|envelope| envelope.as_str().len());
    assert_eq!(length(65_535), Err(TokenSnapshotError::InvalidEnvelope));
    // The most quotes whose envelope still fits, by bisection.
    let (mut fits, mut overflows) = (0, 65_535);
    while overflows - fits > 1 {
        let middle = (fits + overflows) / 2;
        if length(middle).is_ok() {
            fits = middle;
        } else {
            overflows = middle;
        }
    }
    let shortfall = MAX_ENVELOPE_BYTES - length(fits).expect("fits");
    // The key ID is written once, so each extra character adds one byte.
    let exact = sealed(fits, &"k".repeat(1 + shortfall)).expect("exactly at the bound");
    assert_eq!(exact.as_str().len(), 196_608);
    assert_eq!(
        sealed(fits, &"k".repeat(2 + shortfall)).err(),
        Some(TokenSnapshotError::InvalidEnvelope)
    );
}

#[test]
fn key_ids_and_bindings_are_bounded_portable_labels() {
    assert!(TokenSnapshotKey::try_new("k".repeat(128), [1; 32]).is_ok());
    for key_id in [String::new(), "k".repeat(129)] {
        assert_eq!(
            TokenSnapshotKey::try_new(key_id, [1; 32]).err(),
            Some(TokenSnapshotError::InvalidKeyId)
        );
    }
    let bound = TokenSnapshotBinding::try_new("p".repeat(128), "a".repeat(512)).expect("limits");
    assert_eq!(bound.provider(), "p".repeat(128));
    for (provider, account, field) in [
        (String::new(), "account".to_string(), "provider"),
        ("p".repeat(129), "account".to_string(), "provider"),
        ("github".to_string(), String::new(), "account"),
        ("github".to_string(), "a".repeat(513), "account"),
    ] {
        assert_eq!(
            TokenSnapshotBinding::try_new(provider, account).err(),
            Some(TokenSnapshotError::InvalidBinding { field })
        );
    }
}

#[test]
fn debug_output_shows_only_the_provider() {
    let binding = TokenSnapshotBinding::try_new("github", "secret-account").expect("binding");
    assert_eq!(
        format!("{binding:?}"),
        "TokenSnapshotBinding { provider: \"github\", account_id: \"[REDACTED]\" }"
    );
    let encrypted =
        EncryptedTokenSnapshot::seal(&state(), &key("primary", 1), &binding).expect("sealed");
    assert_eq!(
        format!("{encrypted:?}"),
        "EncryptedTokenSnapshot { envelope: \"[REDACTED]\" }"
    );
}
