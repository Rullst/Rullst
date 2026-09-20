use super::*;
use crate::auth::passkey::test_support::{
    RegistrationOptions, assertion_fixture, encode_cbor, registration_fixture,
};

fn localhost_auth() -> PasskeyAuth {
    PasskeyAuth::new(&PasskeyConfig::new(
        "Test App",
        "localhost",
        "http://localhost",
    ))
    .expect("localhost WebAuthn configuration should be valid")
}

fn assert_passkey_error<T>(result: Result<T, AuthError>, expected: &str) {
    match result {
        Err(AuthError::PasskeyError(message)) => assert!(
            message.contains(expected),
            "expected error containing {expected:?}, got {message:?}"
        ),
        Err(other) => panic!("expected passkey error, got {other}"),
        Ok(_) => panic!("expected passkey operation to fail"),
    }
}

fn assert_mutated_attestation<F>(auth: &PasskeyAuth, expected: &str, mutation: F)
where
    F: FnOnce(&mut std::collections::HashMap<CborKey, CborValue>),
{
    let mut fixture = registration_fixture(auth, "localhost", RegistrationOptions::default());
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&fixture.credential.response.attestation_object)
        .expect("fixture attestation should be base64url");
    let (attestation, trailing) = parse_cbor(&decoded).expect("fixture CBOR should parse");
    assert!(trailing.is_empty());
    let CborValue::Map(mut attestation) = attestation else {
        panic!("fixture attestation should be a map");
    };
    mutation(&mut attestation);
    fixture.credential.response.attestation_object =
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(encode_cbor(&CborValue::Map(attestation)));
    assert_passkey_error(
        auth.finish_register(&fixture.credential, &fixture.challenge),
        expected,
    );
}

#[test]
fn relying_party_configuration_covers_loopback_and_malformed_boundaries() {
    assert_passkey_error(
        PasskeyAuth::new(&PasskeyConfig::new(" ", "localhost", "http://localhost")),
        "name cannot be empty",
    );
    assert_passkey_error(
        PasskeyAuth::new(
            &PasskeyConfig::new("RP", "localhost", "http://localhost")
                .with_max_pending_challenges(0),
        ),
        "must be greater than zero",
    );
    assert!(PasskeyAuth::new(&PasskeyConfig::new("RP", "127.0.0.1", "http://127.0.0.1")).is_ok());
    assert!(PasskeyAuth::new(&PasskeyConfig::new("RP", "[::1]", "http://[::1]")).is_ok());
    assert_passkey_error(
        PasskeyAuth::new(&PasskeyConfig::new(
            "RP",
            "example.com",
            "https://user@example.com",
        )),
        "only scheme",
    );
    assert_passkey_error(
        PasskeyAuth::new(&PasskeyConfig::new("RP", "bad id", "https://example.com")),
        "invalid relying-party ID",
    );
}

#[test]
// TM-AUTH-05: authenticator data is bound to the exact relying-party ID hash.
fn client_authenticator_and_credential_inputs_fail_closed() {
    let auth = localhost_auth();
    assert_passkey_error(
        auth.parse_client_data("%%%", "webauthn.get"),
        "decode clientDataJSON",
    );
    let invalid_json = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"not-json");
    assert_passkey_error(
        auth.parse_client_data(&invalid_json, "webauthn.get"),
        "parse clientDataJSON",
    );

    let mut valid_auth_data = sha2::Sha256::digest(b"localhost").to_vec();
    valid_auth_data.push(FLAG_USER_PRESENT | FLAG_USER_VERIFIED);
    valid_auth_data.extend_from_slice(&7_u32.to_be_bytes());
    assert_eq!(
        auth.validate_authenticator_data(&valid_auth_data).unwrap(),
        7
    );

    assert_passkey_error(auth.validate_authenticator_data(&[]), "too short");
    let mut wrong_rp = valid_auth_data.clone();
    wrong_rp[0] ^= 0xff;
    assert_passkey_error(
        auth.validate_authenticator_data(&wrong_rp),
        "rpIdHash mismatch",
    );
    let mut invalid_backup = valid_auth_data;
    invalid_backup[32] |= FLAG_BACKUP_STATE;
    invalid_backup[32] &= !FLAG_BACKUP_ELIGIBLE;
    assert_passkey_error(
        auth.validate_authenticator_data(&invalid_backup),
        "backup flags",
    );

    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([1_u8, 2, 3]);
    assert_passkey_error(
        PasskeyAuth::decode_credential_id(&encoded, &encoded, "password"),
        "public-key",
    );
    assert_passkey_error(
        PasskeyAuth::decode_credential_id(&encoded, "%%%", "public-key"),
        "raw credential ID",
    );
    assert_passkey_error(
        PasskeyAuth::decode_credential_id("%%%", &encoded, "public-key"),
        "credential ID is not valid",
    );
    assert_passkey_error(
        PasskeyAuth::decode_credential_id("", "", "public-key"),
        "length is invalid",
    );
    let oversized =
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(vec![7_u8; MAX_CREDENTIAL_ID_BYTES + 1]);
    assert_passkey_error(
        PasskeyAuth::decode_credential_id(&oversized, &oversized, "public-key"),
        "length is invalid",
    );
    let other = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([9_u8]);
    assert_passkey_error(
        PasskeyAuth::decode_credential_id(&other, &encoded, "public-key"),
        "do not match",
    );
}

#[test]
fn assertions_reject_wrong_keys_encoding_and_unexpected_flags() {
    let auth = localhost_auth();
    let registration = registration_fixture(&auth, "localhost", RegistrationOptions::default());
    let passkey = auth
        .finish_register(&registration.credential, &registration.challenge)
        .unwrap();

    let (credential, challenge) = assertion_fixture(
        &auth,
        &passkey,
        &registration.key_pair,
        "localhost",
        FLAG_USER_PRESENT | FLAG_USER_VERIFIED,
        2,
    );
    let mut mismatched = credential.clone();
    let other = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([99_u8]);
    mismatched.id = other.clone();
    mismatched.raw_id = other;
    assert_passkey_error(
        auth.finish_authenticate(&mismatched, &challenge, passkey.clone()),
        "does not match the passkey",
    );

    let malformed_key = Passkey {
        public_key: vec![0x04],
        ..passkey.clone()
    };
    assert_passkey_error(
        auth.finish_authenticate(&credential, &challenge, malformed_key),
        "stored ES256 public key is malformed",
    );

    let (mut invalid_data, challenge) = assertion_fixture(
        &auth,
        &passkey,
        &registration.key_pair,
        "localhost",
        FLAG_USER_PRESENT | FLAG_USER_VERIFIED,
        2,
    );
    invalid_data.response.authenticator_data = "%%%".to_owned();
    assert_passkey_error(
        auth.finish_authenticate(&invalid_data, &challenge, passkey.clone()),
        "decode authenticatorData",
    );

    let (unexpected_flags, challenge) = assertion_fixture(
        &auth,
        &passkey,
        &registration.key_pair,
        "localhost",
        FLAG_USER_PRESENT | FLAG_USER_VERIFIED | FLAG_EXTENSION_DATA,
        2,
    );
    assert_passkey_error(
        auth.finish_authenticate(&unexpected_flags, &challenge, passkey),
        "unexpected data",
    );
}

#[test]
fn registration_options_expose_preferred_user_verification() {
    let auth = PasskeyAuth::new(
        &PasskeyConfig::new("Test App", "localhost", "http://localhost")
            .require_user_verification(false),
    )
    .expect("localhost WebAuthn configuration should be valid");
    let (creation, _) = auth
        .start_register(7, "alice", "Alice")
        .expect("registration should start");
    assert_eq!(
        creation
            .public_key
            .authenticator_selection
            .user_verification,
        "preferred"
    );

    let passkey = Passkey {
        credential_id: vec![1, 2, 3],
        public_key: vec![0x04; 65],
        sign_count: 0,
    };
    let (request, _) = auth
        .start_authenticate(&[passkey])
        .expect("authentication should start");
    assert_eq!(request.public_key.user_verification, "preferred");
}

#[test]
fn registration_rejects_malformed_attestation_envelopes() {
    let auth = localhost_auth();

    let mut fixture = registration_fixture(&auth, "localhost", RegistrationOptions::default());
    fixture.credential.response.attestation_object = "%%%".to_owned();
    assert_passkey_error(
        auth.finish_register(&fixture.credential, &fixture.challenge),
        "decode attestationObject",
    );

    let mut fixture = registration_fixture(&auth, "localhost", RegistrationOptions::default());
    let mut encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&fixture.credential.response.attestation_object)
        .expect("fixture attestation should be base64url");
    encoded.push(0);
    fixture.credential.response.attestation_object =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(encoded);
    assert_passkey_error(
        auth.finish_register(&fixture.credential, &fixture.challenge),
        "trailing data",
    );

    let mut fixture = registration_fixture(&auth, "localhost", RegistrationOptions::default());
    fixture.credential.response.attestation_object =
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(encode_cbor(&CborValue::Array(Vec::new())));
    assert_passkey_error(
        auth.finish_register(&fixture.credential, &fixture.challenge),
        "is not a map",
    );

    assert_mutated_attestation(&auth, "empty attStmt", |attestation| {
        attestation.remove(&CborKey::TextString("attStmt".to_owned()));
    });
    assert_mutated_attestation(&auth, "authData not found", |attestation| {
        attestation.remove(&CborKey::TextString("authData".to_owned()));
    });
    assert_mutated_attestation(&auth, "invalid attestationObject", |attestation| {
        attestation.insert(
            CborKey::TextString("unexpected".to_owned()),
            CborValue::Integer(1),
        );
    });
}

#[test]
fn registration_rejects_invalid_attested_credential_structure() {
    let auth = localhost_auth();

    assert_mutated_attestation(&auth, "attested credential data flag", |attestation| {
        let Some(CborValue::ByteString(auth_data)) =
            attestation.get_mut(&CborKey::TextString("authData".to_owned()))
        else {
            panic!("fixture should contain authData");
        };
        auth_data[32] &= !FLAG_ATTESTED_CREDENTIAL_DATA;
    });
    assert_mutated_attestation(&auth, "extensions are unsupported", |attestation| {
        let Some(CborValue::ByteString(auth_data)) =
            attestation.get_mut(&CborKey::TextString("authData".to_owned()))
        else {
            panic!("fixture should contain authData");
        };
        auth_data[32] |= FLAG_EXTENSION_DATA;
    });
    assert_mutated_attestation(&auth, "credential ID length", |attestation| {
        let Some(CborValue::ByteString(auth_data)) =
            attestation.get_mut(&CborKey::TextString("authData".to_owned()))
        else {
            panic!("fixture should contain authData");
        };
        auth_data[53..55].copy_from_slice(&0_u16.to_be_bytes());
    });
    assert_mutated_attestation(&auth, "does not match attested", |attestation| {
        let Some(CborValue::ByteString(auth_data)) =
            attestation.get_mut(&CborKey::TextString("authData".to_owned()))
        else {
            panic!("fixture should contain authData");
        };
        auth_data[55] ^= 0xff;
    });
    assert_mutated_attestation(&auth, "trailing data after credential", |attestation| {
        let Some(CborValue::ByteString(auth_data)) =
            attestation.get_mut(&CborKey::TextString("authData".to_owned()))
        else {
            panic!("fixture should contain authData");
        };
        auth_data.push(0);
    });
    assert_mutated_attestation(&auth, "is not a CBOR map", |attestation| {
        let Some(CborValue::ByteString(auth_data)) =
            attestation.get_mut(&CborKey::TextString("authData".to_owned()))
        else {
            panic!("fixture should contain authData");
        };
        auth_data.truncate(59);
        auth_data.push(0);
    });
}
