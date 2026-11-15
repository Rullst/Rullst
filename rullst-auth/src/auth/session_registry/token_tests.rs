use super::*;
use crate::auth::session::{derive_cipher, seal_envelope};

const KEY: &[u8; 32] = b"0123456789abcdefghijklmnopqrstuv";

fn claims(expires_at: u64) -> SessionClaims {
    SessionClaims {
        user_id: 42,
        epoch: 7,
        expires_at,
    }
}

fn secret(byte: u8) -> SessionSecret {
    SessionSecret(Zeroizing::new([byte; SECRET_BYTES]))
}

#[test]
fn a_sealed_token_opens_to_its_claims_and_secret() {
    let cipher = derive_cipher(KEY).unwrap();
    let token = seal(&cipher, &claims(1_000), &secret(9)).unwrap();
    assert!(token.starts_with("v3."));
    let (opened, opened_secret) = open(&cipher, &token, Some(1_000)).unwrap();
    assert_eq!(opened, claims(1_000));
    assert_eq!(opened_secret.lookup_hash(), secret(9).lookup_hash());
    // Signing out ignores the expiry.
    assert!(open(&cipher, &token, None).is_ok());
    assert_eq!(
        open(&cipher, &token, Some(1_001)).err(),
        Some(SessionRegistryError::Expired)
    );
}

#[test]
fn digests_are_fixed_purpose_separated_and_never_the_secret() {
    let zero = secret(0);
    // SHA-256 of the lookup domain and 32 zero bytes, as lowercase hex.
    assert_eq!(
        zero.lookup_hash(),
        "e18e3a20ec3ea25e21c0dd5b9eb0a1afbefee928da53cef5700fd27cbcdaf467"
    );
    // 16 bytes of the reference domain's SHA-256, in base64url.
    assert_eq!(zero.reference().as_str(), "MqZPJoS6uzx38-Ajzc2Qag");
    assert_ne!(zero.lookup_hash(), secret(1).lookup_hash());
    assert_ne!(zero.reference(), secret(1).reference());
    assert_eq!(
        zero.encode().as_str(),
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
}

#[test]
fn generated_secrets_differ() {
    let first = SessionSecret::generate().unwrap();
    let second = SessionSecret::generate().unwrap();
    assert_ne!(first.lookup_hash(), second.lookup_hash());
}

#[test]
fn malformed_tokens_are_invalid() {
    let cipher = derive_cipher(KEY).unwrap();
    let invalid = Some(SessionRegistryError::InvalidToken);
    let token = seal(&cipher, &claims(1_000), &secret(3)).unwrap();
    // Another prefix, an over-long value and an altered envelope.
    assert_eq!(
        open(&cipher, &token.replacen("v3.", "v2.", 1), None).err(),
        invalid
    );
    let long = format!("{}{}", token.as_str(), "A".repeat(4096));
    assert_eq!(open(&cipher, &long, None).err(), invalid);
    let mut altered = token.to_string();
    altered.pop();
    altered.push(if token.ends_with('A') { 'B' } else { 'A' });
    assert_eq!(open(&cipher, &altered, None).err(), invalid);
    // Another key.
    let other = derive_cipher(b"vutsrqponmlkjihgfedcba9876543210").unwrap();
    assert_eq!(open(&other, &token, None).err(), invalid);

    // Well-sealed payloads with the wrong fields.
    let seal_payload = |payload: &str| {
        seal_envelope(
            &cipher,
            REGISTERED_TOKEN_PREFIX,
            REGISTERED_SESSION_AAD,
            payload,
        )
        .unwrap()
    };
    let encoded = secret(3).encode();
    for payload in [
        "42|7|1000".to_owned(),
        format!("42|7|1000|{}|extra", encoded.as_str()),
        format!("x|7|1000|{}", encoded.as_str()),
        format!("42|-1|1000|{}", encoded.as_str()),
        format!("42|7|soon|{}", encoded.as_str()),
        "42|7|1000|short".to_owned(),
        format!("42|7|1000|{}=", &encoded[..42]),
        format!("42|7|1000|{}", "!".repeat(43)),
    ] {
        assert_eq!(
            open(&cipher, &seal_payload(&payload), None).err(),
            invalid,
            "{payload}"
        );
    }
    // The same bytes under the epoch token's associated data do not open.
    let epoch_aad = seal_envelope(
        &cipher,
        REGISTERED_TOKEN_PREFIX,
        b"rullst.session.v2",
        &format!("42|7|1000|{}", encoded.as_str()),
    )
    .unwrap();
    assert_eq!(open(&cipher, &epoch_aad, None).err(), invalid);
    // A well-formed payload opens.
    let (opened, _) = open(
        &cipher,
        &seal_payload(&format!("42|7|1000|{}", encoded.as_str())),
        None,
    )
    .unwrap();
    assert_eq!(opened, claims(1_000));
}
