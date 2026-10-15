//! The documented revocation pattern: the application stores a session epoch
//! per user, puts it in the login cookie and compares it on every request.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::http::HeaderMap;
use rullst_auth::{
    AuthError, DEFAULT_SESSION_MAX_AGE, decrypt_session_claims, encrypt_session,
    encrypt_session_with_epoch, extract_session_cookie,
};
use std::collections::HashMap;

const KEY: &[u8; 32] = b"0123456789abcdefghijklmnopqrstuv";

fn request_with(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::COOKIE,
        format!("theme=dark; rullst_session={token}")
            .parse()
            .unwrap(),
    );
    headers
}

/// What an application's authentication middleware does with the request.
fn signed_in_user(headers: &HeaderMap, epochs: &HashMap<i32, u64>) -> Result<i32, AuthError> {
    let cookie = extract_session_cookie(headers)
        .ok_or_else(|| AuthError::Unauthorized("no session cookie".to_string()))?;
    let claims = decrypt_session_claims(&cookie, KEY)?;
    let current = epochs
        .get(&claims.user_id)
        .ok_or_else(|| AuthError::Unauthorized("unknown account".to_string()))?;
    claims.verify_epoch(*current)
}

#[test]
fn bumping_the_stored_epoch_revokes_every_earlier_cookie() {
    let mut epochs = HashMap::from([(7, 0_u64), (8, 0_u64)]);
    let laptop = encrypt_session_with_epoch(7, 0, DEFAULT_SESSION_MAX_AGE, KEY).unwrap();
    let phone = encrypt_session_with_epoch(7, 0, DEFAULT_SESSION_MAX_AGE, KEY).unwrap();
    let other_user = encrypt_session_with_epoch(8, 0, DEFAULT_SESSION_MAX_AGE, KEY).unwrap();
    assert_eq!(signed_in_user(&request_with(&laptop), &epochs), Ok(7));
    assert_eq!(signed_in_user(&request_with(&phone), &epochs), Ok(7));

    // "Sign out everywhere", a password change or a deactivation bumps the epoch.
    epochs.insert(7, 1);
    for revoked in [&laptop, &phone] {
        assert_eq!(
            signed_in_user(&request_with(revoked), &epochs),
            Err(AuthError::SessionRevoked)
        );
    }
    assert_eq!(signed_in_user(&request_with(&other_user), &epochs), Ok(8));

    // A cookie issued after the bump carries the new epoch.
    let fresh = encrypt_session_with_epoch(7, 1, DEFAULT_SESSION_MAX_AGE, KEY).unwrap();
    assert_eq!(signed_in_user(&request_with(&fresh), &epochs), Ok(7));
}

#[test]
fn cookies_from_before_adoption_count_as_epoch_zero() {
    let mut epochs = HashMap::from([(7, 0_u64)]);
    let legacy = encrypt_session(7, KEY).unwrap();
    assert_eq!(signed_in_user(&request_with(&legacy), &epochs), Ok(7));
    epochs.insert(7, 1);
    assert_eq!(
        signed_in_user(&request_with(&legacy), &epochs),
        Err(AuthError::SessionRevoked)
    );
}
