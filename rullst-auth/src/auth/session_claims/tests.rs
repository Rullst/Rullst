use super::*;
use crate::auth::session::seal_envelope;
use crate::auth::{decrypt_session, encrypt_session};
use base64::{Engine as _, engine::general_purpose};

fn key() -> Vec<u8> {
    (0u8..32).collect()
}

fn cipher() -> Aes256Gcm {
    derive_cipher(&key()).unwrap()
}

fn claims(user_id: i32, epoch: u64, expires_at: u64) -> SessionClaims {
    SessionClaims {
        user_id,
        epoch,
        expires_at,
    }
}

fn decryption_error(message: &str) -> AuthError {
    AuthError::SessionDecryptionError(message.to_string())
}

fn epoch_token(payload: &str) -> String {
    seal_envelope(&cipher(), "v2.", b"rullst.session.v2", payload).unwrap()
}

fn unix_now_for_test() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn epoch_tokens_round_trip_with_exact_claims() {
    let token = seal_claims(&cipher(), 42, 7, 60, 1_000).unwrap();
    assert!(token.starts_with("v2."));
    assert_eq!(
        open_claims(&cipher(), &token, 1_000),
        Ok(claims(42, 7, 1_060))
    );

    let before = unix_now_for_test();
    let token = encrypt_session_with_epoch(42, 7, Duration::from_secs(3_600), &key()).unwrap();
    let after = unix_now_for_test();
    let decoded = decrypt_session_claims(&token, &key()).unwrap();
    assert_eq!((decoded.user_id, decoded.epoch), (42, 7));
    assert!(decoded.expires_at >= before + 3_600);
    assert!(decoded.expires_at <= after + 3_600);
}

#[test]
fn boundary_ids_and_epochs_round_trip() {
    for user_id in [i32::MIN, -1, 0, 1, i32::MAX] {
        for epoch in [0, 1, u64::MAX - 1, u64::MAX] {
            let token = seal_claims(&cipher(), user_id, epoch, 1, 5).unwrap();
            assert_eq!(
                open_claims(&cipher(), &token, 5),
                Ok(claims(user_id, epoch, 6))
            );
        }
    }
}

#[test]
fn expiry_second_is_still_valid_and_the_next_is_expired() {
    let token = seal_claims(&cipher(), 42, 7, 60, 1_000).unwrap();
    assert_eq!(
        open_claims(&cipher(), &token, 1_059),
        Ok(claims(42, 7, 1_060))
    );
    assert_eq!(
        open_claims(&cipher(), &token, 1_060),
        Ok(claims(42, 7, 1_060))
    );
    assert_eq!(
        open_claims(&cipher(), &token, 1_061),
        Err(AuthError::SessionExpired)
    );

    // The latest representable expiry is accepted; one second more overflows.
    let token = seal_claims(&cipher(), 42, 7, 60, u64::MAX - 60).unwrap();
    assert_eq!(
        open_claims(&cipher(), &token, u64::MAX),
        Ok(claims(42, 7, u64::MAX))
    );
    assert_eq!(
        seal_claims(&cipher(), 42, 7, 60, u64::MAX - 59),
        Err(AuthError::SessionEncryptionError(
            "session expiry is out of range".to_string()
        ))
    );

    // The public decoder reads the real clock: a token that ended in 1970 is expired.
    let expired = seal_claims(&cipher(), 42, 7, 60, 1_000).unwrap();
    assert_eq!(
        decrypt_session_claims(&expired, &key()),
        Err(AuthError::SessionExpired)
    );
}

#[test]
fn verify_epoch_accepts_only_the_current_epoch() {
    let session = claims(42, 7, 1_060);
    assert_eq!(session.verify_epoch(7), Ok(42));
    for stale_or_future in [0, 6, 8, u64::MAX] {
        assert_eq!(
            session.verify_epoch(stale_or_future),
            Err(AuthError::SessionRevoked)
        );
    }

    let last = claims(-5, u64::MAX, 1_060);
    assert_eq!(last.verify_epoch(u64::MAX), Ok(-5));
    assert_eq!(
        last.verify_epoch(u64::MAX - 1),
        Err(AuthError::SessionRevoked)
    );
    assert_eq!(last.verify_epoch(0), Err(AuthError::SessionRevoked));
}

#[test]
fn legacy_tokens_decode_as_epoch_zero() {
    let legacy = encrypt_session(42, &key()).unwrap();
    let decoded = decrypt_session_claims(&legacy, &key()).unwrap();
    assert_eq!((decoded.user_id, decoded.epoch), (42, 0));
    assert_eq!(decoded.verify_epoch(0), Ok(42));
    // The first bump of the stored epoch revokes cookies issued before adoption.
    assert_eq!(decoded.verify_epoch(1), Err(AuthError::SessionRevoked));

    let crafted = seal_envelope(&cipher(), "v1.", b"rullst.session.v1", "42|1060").unwrap();
    assert_eq!(
        open_claims(&cipher(), &crafted, 1_060),
        Ok(claims(42, 0, 1_060))
    );
    assert_eq!(
        open_claims(&cipher(), &crafted, 1_061),
        Err(AuthError::SessionExpired)
    );
    for malformed in ["missing-separator", "42|not-a-timestamp", "not-an-id|1060"] {
        let token = seal_envelope(&cipher(), "v1.", b"rullst.session.v1", malformed).unwrap();
        assert!(matches!(
            open_claims(&cipher(), &token, 1_000),
            Err(AuthError::SessionDecryptionError(_))
        ));
    }
    let missing = seal_envelope(&cipher(), "v1.", b"rullst.session.v1", "4242").unwrap();
    assert_eq!(
        open_claims(&cipher(), &missing, 1_000),
        Err(decryption_error("Invalid versioned session payload"))
    );
}

#[test]
fn the_old_decoder_rejects_epoch_tokens() {
    let token = encrypt_session_with_epoch(42, 0, DEFAULT_SESSION_MAX_AGE, &key()).unwrap();
    assert_eq!(
        decrypt_session(&token, &key()),
        Err(decryption_error("Unsupported session token version"))
    );
}

#[test]
fn version_prefixes_are_bound_to_their_payload_format() {
    let epoch = seal_claims(&cipher(), 42, 7, 60, 1_000).unwrap();
    let relabeled = epoch.replacen("v2.", "v1.", 1);
    assert!(matches!(
        open_claims(&cipher(), &relabeled, 1_000),
        Err(AuthError::SessionDecryptionError(_))
    ));

    let legacy = seal_envelope(&cipher(), "v1.", b"rullst.session.v1", "42|1060").unwrap();
    let relabeled = legacy.replacen("v1.", "v2.", 1);
    assert!(matches!(
        open_claims(&cipher(), &relabeled, 1_000),
        Err(AuthError::SessionDecryptionError(_))
    ));
}

#[test]
fn tampered_and_wrongly_keyed_tokens_fail() {
    let token = encrypt_session_with_epoch(42, 7, DEFAULT_SESSION_MAX_AGE, &key()).unwrap();
    let mut bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(token.strip_prefix("v2.").unwrap())
        .unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    let tampered = format!("v2.{}", general_purpose::URL_SAFE_NO_PAD.encode(&bytes));
    assert!(matches!(
        decrypt_session_claims(&tampered, &key()),
        Err(AuthError::SessionDecryptionError(_))
    ));

    let mut wrong_key = key();
    wrong_key[0] ^= 0xff;
    assert!(matches!(
        decrypt_session_claims(&token, &wrong_key),
        Err(AuthError::SessionDecryptionError(_))
    ));
}

#[test]
fn malformed_epoch_payloads_fail() {
    for payload in ["42|7", "42|7|1060|9", "", "42"] {
        assert_eq!(
            open_claims(&cipher(), &epoch_token(payload), 1_000),
            Err(decryption_error("Invalid versioned session payload")),
            "{payload}"
        );
    }
    for payload in [
        "x|7|1060",
        "42|x|1060",
        "42|7|x",
        "42|-1|1060",
        "42|18446744073709551616|1060",
        "2147483648|7|1060",
        "42||1060",
    ] {
        assert!(
            matches!(
                open_claims(&cipher(), &epoch_token(payload), 1_000),
                Err(AuthError::SessionDecryptionError(_))
            ),
            "{payload}"
        );
    }
}

#[test]
fn oversized_short_and_unversioned_tokens_fail_closed() {
    let too_long = format!("v2.{}", "A".repeat(4_094));
    assert_eq!(too_long.len(), 4_097);
    assert_eq!(
        open_claims(&cipher(), &too_long, 1_000),
        Err(decryption_error("Session token too long"))
    );
    let at_limit = format!("v2.{}", "A".repeat(4_093));
    assert_eq!(at_limit.len(), 4_096);
    let error = open_claims(&cipher(), &at_limit, 1_000).unwrap_err();
    assert!(matches!(error, AuthError::SessionDecryptionError(_)));
    assert_ne!(error, decryption_error("Session token too long"));

    let short = format!("v2.{}", general_purpose::URL_SAFE_NO_PAD.encode([0u8; 27]));
    assert_eq!(
        open_claims(&cipher(), &short, 1_000),
        Err(decryption_error("Invalid token length"))
    );
    let minimum = format!("v2.{}", general_purpose::URL_SAFE_NO_PAD.encode([0u8; 28]));
    let error = open_claims(&cipher(), &minimum, 1_000).unwrap_err();
    assert!(matches!(error, AuthError::SessionDecryptionError(_)));
    assert_ne!(error, decryption_error("Invalid token length"));

    let valid = seal_claims(&cipher(), 42, 7, 60, 1_000).unwrap();
    for unversioned in [
        "".to_string(),
        valid.trim_start_matches("v2.").to_string(),
        valid.replacen("v2.", "v3.", 1),
    ] {
        assert_eq!(
            open_claims(&cipher(), &unversioned, 1_000),
            Err(decryption_error("Unsupported session token version"))
        );
    }
}

#[test]
fn max_age_is_whole_seconds_from_one_second_to_400_days() {
    assert_eq!(DEFAULT_SESSION_MAX_AGE, Duration::from_secs(2_592_000));
    assert_eq!(MAX_SESSION_MAX_AGE, Duration::from_secs(34_560_000));
    let rejected = AuthError::SessionEncryptionError(
        "session max_age must be between 1 second and 400 days".to_string(),
    );
    assert_eq!(max_age_seconds(Duration::ZERO), Err(rejected.clone()));
    assert_eq!(
        max_age_seconds(Duration::from_millis(999)),
        Err(rejected.clone())
    );
    assert_eq!(
        max_age_seconds(Duration::from_secs(34_560_001)),
        Err(rejected.clone())
    );
    assert_eq!(max_age_seconds(Duration::from_secs(1)), Ok(1));
    assert_eq!(max_age_seconds(Duration::from_millis(1_999)), Ok(1));
    assert_eq!(
        max_age_seconds(Duration::from_secs(34_560_000)),
        Ok(34_560_000)
    );
    assert_eq!(
        encrypt_session_with_epoch(42, 7, Duration::ZERO, &key()),
        Err(rejected.clone())
    );
    assert_eq!(
        make_login_cookie_with_epoch(42, 7, Duration::from_secs(34_560_001)),
        Err(rejected)
    );
}

#[test]
fn login_cookie_value_has_exact_attributes() {
    assert_eq!(
        login_cookie_value("token", 60, false),
        "rullst_session=token; Path=/; HttpOnly; SameSite=Lax; Max-Age=60"
    );
    assert_eq!(
        login_cookie_value("token", 34_560_000, true),
        "rullst_session=token; Path=/; HttpOnly; SameSite=Lax; Max-Age=34560000; Secure"
    );
}

#[test]
fn login_cookie_with_epoch_carries_the_epoch_and_max_age() {
    let before = unix_now_for_test();
    let cookie = make_login_cookie_with_epoch(42, 9, Duration::from_secs(120)).unwrap();
    let after = unix_now_for_test();
    assert!(cookie.starts_with("rullst_session=v2."));
    assert!(cookie.contains("; Path=/; HttpOnly; SameSite=Lax; Max-Age=120"));
    let token = cookie
        .strip_prefix("rullst_session=")
        .and_then(|rest| rest.split(';').next())
        .unwrap();
    let decoded = decrypt_session_claims(token, &get_app_key().unwrap()).unwrap();
    assert_eq!((decoded.user_id, decoded.epoch), (42, 9));
    assert!(decoded.expires_at >= before + 120);
    assert!(decoded.expires_at <= after + 120);
    assert_eq!(decoded.verify_epoch(9), Ok(42));
    assert_eq!(decoded.verify_epoch(10), Err(AuthError::SessionRevoked));
}
