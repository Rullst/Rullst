#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{Passkey, PasskeyAuth, PasskeyConfig};
use crate::error::AuthError;

use super::test_support::{RegistrationOptions, assertion_fixture, registration_fixture};

fn localhost_auth() -> PasskeyAuth {
    PasskeyAuth::new(&PasskeyConfig::new(
        "Test App",
        "localhost",
        "http://localhost",
    ))
    .expect("localhost configuration should be valid")
}

fn error_message(result: Result<Passkey, AuthError>) -> String {
    match result.expect_err("ceremony should be rejected") {
        AuthError::PasskeyError(message) => message,
        other => other.to_string(),
    }
}

#[test]
fn relying_party_configuration_is_validated_exactly() {
    assert!(
        PasskeyAuth::new(&PasskeyConfig::new(
            "RP",
            "example.com",
            "http://example.com"
        ))
        .is_err()
    );
    assert!(
        PasskeyAuth::new(&PasskeyConfig::new(
            "RP",
            "localhost.evil",
            "http://localhost.evil"
        ))
        .is_err()
    );
    assert!(
        PasskeyAuth::new(&PasskeyConfig::new(
            "RP",
            "example.com",
            "https://example.com/path"
        ))
        .is_err()
    );
    assert!(
        PasskeyAuth::new(&PasskeyConfig::new(
            "RP",
            "example.com",
            "https://login.example.com"
        ))
        .is_ok()
    );
    assert!(
        PasskeyAuth::new(&PasskeyConfig::new(
            "RP",
            "example.com",
            "https://example.evil"
        ))
        .is_err()
    );
    assert!(
        PasskeyAuth::new(
            &PasskeyConfig::new("RP", "example.com", "https://example.com")
                .with_challenge_ttl_seconds(0)
        )
        .is_err()
    );
}

#[test]
// TM-AUTH-05: ceremony type, exact origin, and authenticator flags are bound.
fn registration_rejects_wrong_ceremony_cross_origin_and_bad_flags() {
    let cases = [
        (
            RegistrationOptions {
                client_type: "webauthn.get".to_owned(),
                ..Default::default()
            },
            "ceremony type",
        ),
        (
            RegistrationOptions {
                cross_origin: true,
                ..Default::default()
            },
            "cross-origin",
        ),
        (
            RegistrationOptions {
                flags: 0x44,
                ..Default::default()
            },
            "user presence",
        ),
        (
            RegistrationOptions {
                flags: 0x41,
                ..Default::default()
            },
            "user verification",
        ),
    ];

    for (options, expected) in cases {
        let auth = localhost_auth();
        let fixture = registration_fixture(&auth, "localhost", options);
        let message = error_message(auth.finish_register(&fixture.credential, &fixture.challenge));
        assert!(message.contains(expected), "unexpected error: {message}");
    }
}

#[test]
fn registration_rejects_unsupported_attestation_and_malformed_cose_keys() {
    let cases = [
        (
            RegistrationOptions {
                format: "packed".to_owned(),
                ..Default::default()
            },
            "none attestation",
        ),
        (
            RegistrationOptions {
                algorithm: -257,
                ..Default::default()
            },
            "COSE alg",
        ),
        (
            RegistrationOptions {
                coordinate_length: 31,
                ..Default::default()
            },
            "32 bytes",
        ),
        (
            RegistrationOptions {
                include_x: false,
                ..Default::default()
            },
            "X coordinate missing",
        ),
        (
            RegistrationOptions {
                include_y: false,
                ..Default::default()
            },
            "Y coordinate missing",
        ),
        (
            RegistrationOptions {
                invalid_curve_point: true,
                ..Default::default()
            },
            "valid P-256 point",
        ),
        (
            RegistrationOptions {
                raw_id: Some(vec![99, 98, 97]),
                ..Default::default()
            },
            "credential ID and raw ID",
        ),
    ];

    for (options, expected) in cases {
        let auth = localhost_auth();
        let fixture = registration_fixture(&auth, "localhost", options);
        let message = error_message(auth.finish_register(&fixture.credential, &fixture.challenge));
        assert!(message.contains(expected), "unexpected error: {message}");
    }
}

#[test]
// TM-AUTH-05: pending challenges are bounded, shared, and consumed once.
fn challenges_are_shared_bounded_and_single_use() {
    let bounded = PasskeyAuth::new(
        &PasskeyConfig::new("Test App", "localhost", "http://localhost")
            .with_max_pending_challenges(1),
    )
    .unwrap();
    bounded.start_register(1, "alice", "Alice").unwrap();
    let clone = bounded.clone();
    assert!(clone.start_register(2, "bob", "Bob").is_err());

    let auth = localhost_auth();
    let fixture = registration_fixture(&auth, "localhost", RegistrationOptions::default());
    let passkey = auth
        .finish_register(&fixture.credential, &fixture.challenge)
        .unwrap();
    assert_eq!(passkey.sign_count, 1);
    assert_eq!(passkey.credential_id, fixture.credential_id);
    assert_eq!(passkey.public_key, fixture.public_key);

    let replay = error_message(auth.finish_register(&fixture.credential, &fixture.challenge));
    assert!(
        replay.contains("already consumed"),
        "unexpected error: {replay}"
    );
}

#[test]
fn assertions_require_presence_verification_and_monotonic_counters() {
    let auth = localhost_auth();
    let fixture = registration_fixture(&auth, "localhost", RegistrationOptions::default());
    let passkey = auth
        .finish_register(&fixture.credential, &fixture.challenge)
        .unwrap();

    let (missing_presence, challenge) =
        assertion_fixture(&auth, &passkey, &fixture.key_pair, "localhost", 0x04, 2);
    let message =
        error_message(auth.finish_authenticate(&missing_presence, &challenge, passkey.clone()));
    assert!(message.contains("user presence"));

    let (missing_verification, challenge) =
        assertion_fixture(&auth, &passkey, &fixture.key_pair, "localhost", 0x01, 2);
    let message =
        error_message(auth.finish_authenticate(&missing_verification, &challenge, passkey.clone()));
    assert!(message.contains("user verification"));

    let (stale_counter, challenge) =
        assertion_fixture(&auth, &passkey, &fixture.key_pair, "localhost", 0x05, 1);
    let message =
        error_message(auth.finish_authenticate(&stale_counter, &challenge, passkey.clone()));
    assert!(message.contains("did not advance monotonically"));

    let (valid, challenge) =
        assertion_fixture(&auth, &passkey, &fixture.key_pair, "localhost", 0x05, 2);
    let updated = auth
        .finish_authenticate(&valid, &challenge, passkey.clone())
        .unwrap();
    assert_eq!(updated.sign_count, 2);
    let replay = error_message(auth.finish_authenticate(&valid, &challenge, passkey));
    assert!(
        replay.contains("already consumed"),
        "unexpected error: {replay}"
    );
}
