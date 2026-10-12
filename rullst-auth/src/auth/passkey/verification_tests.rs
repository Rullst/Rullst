//! Attested-credential lengths, coordinate sizes and counter rules at their limits.
use super::*;
use crate::auth::passkey::test_support::{
    RegistrationFixture, RegistrationOptions, assertion_fixture, encode_cbor, registration_fixture,
};
use std::collections::HashMap;

fn localhost_auth() -> PasskeyAuth {
    PasskeyAuth::new(&PasskeyConfig::new(
        "Test App",
        "localhost",
        "http://localhost",
    ))
    .expect("localhost WebAuthn configuration should be valid")
}

fn cose_key(x: &[u8], y: &[u8]) -> Vec<u8> {
    let mut key = HashMap::new();
    key.insert(CborKey::Integer(1), CborValue::Integer(2));
    key.insert(CborKey::Integer(3), CborValue::Integer(-7));
    key.insert(CborKey::Integer(-1), CborValue::Integer(1));
    key.insert(CborKey::Integer(-2), CborValue::ByteString(x.to_vec()));
    key.insert(CborKey::Integer(-3), CborValue::ByteString(y.to_vec()));
    encode_cbor(&CborValue::Map(key))
}

/// authData with flags UP|UV|AT, a declared credential-ID length and a tail.
fn auth_data(declared: u16, credential_id: &[u8], tail: &[u8]) -> Vec<u8> {
    let mut data = sha2::Sha256::digest(b"localhost").to_vec();
    data.push(0x45);
    data.extend_from_slice(&1_u32.to_be_bytes());
    data.extend_from_slice(&[0; 16]);
    data.extend_from_slice(&declared.to_be_bytes());
    data.extend_from_slice(credential_id);
    data.extend_from_slice(tail);
    data
}

/// Finishes the fixture's ceremony with replaced authData and response ID.
fn finish(
    auth: &PasskeyAuth,
    mut fixture: RegistrationFixture,
    auth_data: Vec<u8>,
    response_id: &[u8],
) -> Result<Passkey, AuthError> {
    let mut attestation = HashMap::new();
    attestation.insert(
        CborKey::TextString("fmt".to_owned()),
        CborValue::TextString("none".to_owned()),
    );
    attestation.insert(
        CborKey::TextString("attStmt".to_owned()),
        CborValue::Map(HashMap::new()),
    );
    attestation.insert(
        CborKey::TextString("authData".to_owned()),
        CborValue::ByteString(auth_data),
    );
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    fixture.credential.response.attestation_object =
        engine.encode(encode_cbor(&CborValue::Map(attestation)));
    fixture.credential.id = engine.encode(response_id);
    fixture.credential.raw_id = engine.encode(response_id);
    auth.finish_register(&fixture.credential, &fixture.challenge)
}

fn fixture(auth: &PasskeyAuth) -> RegistrationFixture {
    registration_fixture(auth, "localhost", RegistrationOptions::default())
}

fn coordinates(fixture: &RegistrationFixture) -> (Vec<u8>, Vec<u8>) {
    (
        fixture.public_key[1..33].to_vec(),
        fixture.public_key[33..65].to_vec(),
    )
}

fn passkey_error<T: std::fmt::Debug>(result: Result<T, AuthError>, expected: &str) {
    match result {
        Err(AuthError::PasskeyError(message)) => assert!(
            message.contains(expected),
            "expected {expected:?}, got {message:?}"
        ),
        other => panic!("expected a passkey error containing {expected:?}, got {other:?}"),
    }
}

#[test]
fn attested_credential_ids_may_use_all_1023_bytes() {
    let auth = localhost_auth();
    let fixture = fixture(&auth);
    let (x, y) = coordinates(&fixture);
    let id = vec![7; 1_023];
    let data = auth_data(1_023, &id, &cose_key(&x, &y));
    let passkey = finish(&auth, fixture, data, &id).expect("1023-byte credential ID");
    assert_eq!(passkey.credential_id.len(), 1_023);
}

#[test]
fn attested_lengths_are_checked_before_slicing() {
    let auth = localhost_auth();
    // 55 bytes is a complete header that declares an empty credential ID.
    let header = auth_data(0, &[], &[]);
    assert_eq!(header.len(), 55);
    passkey_error(
        finish(&auth, fixture(&auth), header, &[1, 2, 3, 4]),
        "invalid attested credential ID length",
    );
    // A declared ID longer than the remaining bytes is rejected, not sliced.
    let fixture_long = fixture(&auth);
    let (x, y) = coordinates(&fixture_long);
    let truncated = auth_data(100, &[10, 20, 30, 40], &cose_key(&x, &y));
    assert!(truncated.len() < 155);
    passkey_error(
        finish(&auth, fixture_long, truncated, &[10, 20, 30, 40]),
        "invalid attested credential ID length",
    );
    // An ID that ends exactly at the end of authData leaves no public key.
    let exact = auth_data(4, &[10, 20, 30, 40], &[]);
    assert!(matches!(
        finish(&auth, fixture(&auth), exact, &[10, 20, 30, 40]),
        Err(AuthError::CborParseError(_))
    ));
}

#[test]
fn each_coordinate_must_have_32_bytes() {
    let auth = localhost_auth();
    for short_x in [true, false] {
        let fixture = fixture(&auth);
        let (mut x, mut y) = coordinates(&fixture);
        if short_x {
            x.pop();
        } else {
            y.pop();
        }
        let data = auth_data(4, &[10, 20, 30, 40], &cose_key(&x, &y));
        passkey_error(
            finish(&auth, fixture, data, &[10, 20, 30, 40]),
            "coordinates must each contain 32 bytes",
        );
    }
}

#[test]
fn counters_may_stay_at_zero_but_never_fall_back_or_repeat() {
    let auth = localhost_auth();
    let fixture = fixture(&auth);
    let registered = auth
        .finish_register(&fixture.credential, &fixture.challenge)
        .expect("registration");
    let verify = |stored: u32, reported: u32| {
        let mut passkey = registered.clone();
        passkey.sign_count = stored;
        let (assertion, challenge) = assertion_fixture(
            &auth,
            &passkey,
            &fixture.key_pair,
            "localhost",
            0x05,
            reported,
        );
        auth.finish_authenticate(&assertion, &challenge, passkey)
            .map(|updated| updated.sign_count)
    };
    assert_eq!(verify(0, 0).ok(), Some(0), "counterless authenticators");
    assert_eq!(verify(2, 3).ok(), Some(3));
    for (stored, reported) in [(2, 0), (2, 2), (2, 1)] {
        passkey_error(verify(stored, reported), "did not advance monotonically");
    }
}
