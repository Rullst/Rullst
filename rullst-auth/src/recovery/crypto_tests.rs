//! Sealing limits, notice accessors and locale wire values.
use super::*;

fn keys() -> RecoverySecrets {
    RecoverySecrets::new([3; 32], [9; 32]).unwrap()
}

#[test]
fn sealing_accepts_sixteen_kibibytes_and_opening_accepts_24000_characters() {
    let keys = keys();
    let largest = keys.seal("context", &[7; 16_384]).unwrap();
    assert_eq!(keys.open("context", &largest).unwrap().len(), 16_384);
    assert_eq!(
        keys.seal("context", &[7; 16_385]).err(),
        Some(RecoveryError::InvalidInput)
    );
    // An authentic ciphertext of exactly 24000 characters still opens.
    let cipher = Aes256Gcm::new_from_slice(&[9; 32]).unwrap();
    let nonce = [5; 12];
    let mut bytes = nonce.to_vec();
    bytes.extend(
        cipher
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &[1; 17_972],
                    aad: b"context",
                },
            )
            .unwrap(),
    );
    let encoded = URL_SAFE_NO_PAD.encode(&bytes);
    assert_eq!(encoded.len(), 24_000);
    assert_eq!(keys.open("context", &encoded).unwrap().len(), 17_972);
    assert_eq!(
        keys.open("context", &format!("{encoded}A")).err(),
        Some(RecoveryError::Crypto)
    );
    assert_eq!(
        keys.open("other", &encoded).err(),
        Some(RecoveryError::Crypto)
    );
}

#[test]
fn notices_round_trip_their_fields_and_debug_only_the_kind() {
    let keys = keys();
    let notice = RecoveryNotice {
        recipient: Zeroizing::new("learner@example.com".to_owned()),
        token: Some(SecretToken::generate().unwrap()),
        kind: RecoveryNoticeKind::PasswordReset,
        expires_at: 1_800_000_600,
        locale: Some(RecoveryLocale::PtBr),
    };
    let opened = RecoveryNotice::open(&keys, "n-1", &notice.seal(&keys, "n-1").unwrap()).unwrap();
    assert_eq!(opened.recipient(), "learner@example.com");
    assert_eq!(opened.expires_at(), 1_800_000_600);
    assert_eq!(opened.locale(), Some(RecoveryLocale::PtBr));
    assert_eq!(opened.kind(), RecoveryNoticeKind::PasswordReset);
    assert_eq!(
        opened.token().map(SecretToken::expose),
        notice.token().map(SecretToken::expose)
    );
    assert_eq!(
        format!("{opened:?}"),
        "RecoveryNotice { kind: PasswordReset, .. }"
    );
    assert_eq!(
        format!("{:?}", SecretToken::generate().unwrap()),
        "SecretToken([REDACTED])"
    );
}

#[test]
fn locales_have_exact_wire_values() {
    for (locale, wire) in [
        (RecoveryLocale::En, "en"),
        (RecoveryLocale::PtBr, "pt-BR"),
        (RecoveryLocale::Es, "es"),
    ] {
        assert_eq!(locale.as_str(), wire);
        assert_eq!(RecoveryLocale::parse(wire), Ok(Some(locale)));
    }
    assert_eq!(RecoveryLocale::parse(""), Ok(None));
    for unknown in ["EN", "pt-br", "fr", " en"] {
        assert_eq!(RecoveryLocale::parse(unknown), Err(RecoveryError::Storage));
    }
}
