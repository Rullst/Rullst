//! Boundary contracts of the shared post-signature claim policy and of the
//! input checks that run before any ID-token I/O.
use crate::ConnectError;
use crate::provider::id_token::{
    validate_authorized_presenters, validate_claims, validate_verification_input,
};
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn claims(subject: &str, issued_at: u64, expires_at: u64) -> Value {
    json!({"aud": "client-id", "sub": subject, "iat": issued_at, "exp": expires_at,
        "nonce": "nonce-1"})
}

fn check(claims: &Value) -> Result<(), ConnectError> {
    validate_claims(claims, "client-id", Some("nonce-1"))
}

#[test]
fn subjects_are_one_to_255_bytes_of_ascii_without_controls() {
    let now = now();
    let longest = "s".repeat(255);
    assert!(check(&claims(&longest, now, now + 3_600)).is_ok());
    for subject in [
        String::new(),
        "s".repeat(256),
        "sujeito-ç".to_string(),
        "tab\tsubject".to_string(),
        "line\nbreak".to_string(),
    ] {
        assert!(
            matches!(
                check(&claims(&subject, now, now + 3_600)),
                Err(ConnectError::Provider(_))
            ),
            "accepted subject of {} bytes",
            subject.len()
        );
    }
}

#[test]
fn issuance_may_lead_the_clock_by_one_minute_only() {
    let now = now();
    // The server clock can only move forward, so `now + 60` stays within the
    // allowance however long the call takes.
    assert!(check(&claims("subject-1", now + 60, now + 3_600)).is_ok());
    assert!(check(&claims("subject-1", now + 3_600, now + 7_200)).is_err());
}

#[test]
fn expiry_must_follow_issuance() {
    let now = now();
    assert!(check(&claims("subject-1", now, now + 1)).is_ok());
    for expires_at in [now, now - 1] {
        assert!(
            check(&claims("subject-1", now, expires_at)).is_err(),
            "accepted exp {} seconds after iat",
            expires_at as i64 - now as i64
        );
    }
}

#[test]
fn verification_input_bounds_the_token_and_the_nonce() {
    let token = "t".repeat(16_384);
    let nonce = "n".repeat(512);
    assert!(validate_verification_input(&token, &nonce).is_ok());
    for (token, nonce) in [(String::new(), "nonce"), ("t".repeat(16_385), "nonce")] {
        assert!(
            matches!(
                validate_verification_input(&token, nonce),
                Err(ConnectError::Token(_))
            ),
            "accepted a token of {} bytes",
            token.len()
        );
    }
    for nonce in [String::new(), "n".repeat(513)] {
        assert!(
            matches!(
                validate_verification_input("header.payload.signature", &nonce),
                Err(ConnectError::Provider(_))
            ),
            "accepted a nonce of {} bytes",
            nonce.len()
        );
    }
}

#[test]
fn a_255_byte_presenter_is_accepted() {
    let longest = "p".repeat(255);
    assert_eq!(
        validate_authorized_presenters([longest.clone()]).unwrap(),
        vec![longest]
    );
}
