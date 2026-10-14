//! Login cookies that carry a per-user session epoch, so the application can
//! revoke them.
//!
//! The cookie stays stateless: it holds the user id, the epoch the application
//! stored for that user when the cookie was issued, and its expiry. The
//! application keeps the current epoch per user and bumps it on "sign out
//! everywhere", a password change or reset, or account deactivation; every
//! cookie carrying an older epoch then fails [`SessionClaims::verify_epoch`].

use super::app_key::{detect_environment, get_app_key};
use super::session::{
    MAX_SESSION_COOKIE_BYTES, SESSION_AAD, SESSION_TOKEN_PREFIX, derive_cipher, login_cookie_value,
    open_envelope, seal_envelope,
};
use crate::error::AuthError;
use aes_gcm::Aes256Gcm;
use std::fmt::Display;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

/// Version prefix of tokens that carry a session epoch.
pub(super) const EPOCH_TOKEN_PREFIX: &str = "v2.";
/// Associated data binding the epoch payload format to its prefix.
pub(super) const EPOCH_SESSION_AAD: &[u8] = b"rullst.session.v2";

/// Lifetime of the cookie made by [`make_login_cookie`](super::make_login_cookie),
/// and the usual choice for [`make_login_cookie_with_epoch`]: 30 days.
pub const DEFAULT_SESSION_MAX_AGE: Duration = Duration::from_secs(2_592_000);

/// Longest accepted login-cookie lifetime: 400 days, the limit browsers apply
/// to a cookie's `Max-Age`.
pub const MAX_SESSION_MAX_AGE: Duration = Duration::from_secs(34_560_000);

/// What a valid login token says, as returned by [`decrypt_session_claims`].
///
/// A token made by [`encrypt_session`](super::encrypt_session) or
/// [`make_login_cookie`](super::make_login_cookie), which carries no epoch,
/// decodes with `epoch` 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SessionClaims {
    /// The signed-in user's id.
    pub user_id: i32,
    /// The user's session epoch when the token was issued.
    pub epoch: u64,
    /// Expiry as Unix time in seconds; the token is valid through this second.
    pub expires_at: u64,
}

impl SessionClaims {
    /// Returns the user id when `current_epoch`, the value the application
    /// stores for this user now, equals the token's epoch, and
    /// [`AuthError::SessionRevoked`] otherwise. The comparison runs in
    /// constant time.
    pub fn verify_epoch(&self, current_epoch: u64) -> Result<i32, AuthError> {
        if bool::from(self.epoch.ct_eq(&current_epoch)) {
            Ok(self.user_id)
        } else {
            Err(AuthError::SessionRevoked)
        }
    }
}

/// Encrypts a login token holding `user_id`, the user's current session
/// `epoch` and an expiry `max_age` from now (whole seconds, from 1 second to
/// [`MAX_SESSION_MAX_AGE`]).
pub fn encrypt_session_with_epoch(
    user_id: i32,
    epoch: u64,
    max_age: Duration,
    app_key: &[u8],
) -> Result<String, AuthError> {
    let max_age_seconds = max_age_seconds(max_age)?;
    let cipher = derive_cipher(app_key)?;
    seal_claims(
        &cipher,
        user_id,
        epoch,
        max_age_seconds,
        unix_now(AuthError::SessionEncryptionError)?,
    )
}

/// Decrypts a login token and returns its [`SessionClaims`].
///
/// Accepts tokens from [`encrypt_session_with_epoch`] and, with epoch 0, the
/// older tokens from [`encrypt_session`](super::encrypt_session). Expired,
/// tampered, wrongly keyed, unversioned and tokens over 4096 bytes fail.
/// Decoding does not check the epoch: call [`SessionClaims::verify_epoch`]
/// with the value stored for the user.
pub fn decrypt_session_claims(token: &str, app_key: &[u8]) -> Result<SessionClaims, AuthError> {
    let cipher = derive_cipher(app_key)?;
    open_claims(&cipher, token, unix_now(AuthError::SessionDecryptionError)?)
}

/// Generates the `Set-Cookie` value for a revocable login: the same
/// `rullst_session` cookie attributes as
/// [`make_login_cookie`](super::make_login_cookie), a token carrying the
/// user's current session `epoch`, and a `Max-Age` of `max_age` whole seconds
/// (1 second to [`MAX_SESSION_MAX_AGE`]; [`DEFAULT_SESSION_MAX_AGE`] is 30
/// days).
pub fn make_login_cookie_with_epoch(
    user_id: i32,
    epoch: u64,
    max_age: Duration,
) -> Result<String, AuthError> {
    let max_age_seconds = max_age_seconds(max_age)?;
    let app_key = get_app_key()?;
    let cipher = derive_cipher(&app_key)?;
    let token = seal_claims(
        &cipher,
        user_id,
        epoch,
        max_age_seconds,
        unix_now(AuthError::SessionEncryptionError)?,
    )?;
    Ok(login_cookie_value(
        &token,
        max_age_seconds,
        detect_environment()?.requires_secure_defaults(),
    ))
}

fn unix_now(error: fn(String) -> AuthError) -> Result<u64, AuthError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|e| error(e.to_string()))
}

/// Whole seconds of `max_age`, rejected outside 1 second to 400 days.
pub(super) fn max_age_seconds(max_age: Duration) -> Result<u64, AuthError> {
    let seconds = max_age.as_secs();
    if seconds == 0 || seconds > MAX_SESSION_MAX_AGE.as_secs() {
        return Err(AuthError::SessionEncryptionError(
            "session max_age must be between 1 second and 400 days".to_string(),
        ));
    }
    Ok(seconds)
}

pub(super) fn seal_claims(
    cipher: &Aes256Gcm,
    user_id: i32,
    epoch: u64,
    max_age_seconds: u64,
    now: u64,
) -> Result<String, AuthError> {
    let expires_at = now.checked_add(max_age_seconds).ok_or_else(|| {
        AuthError::SessionEncryptionError("session expiry is out of range".to_string())
    })?;
    seal_envelope(
        cipher,
        EPOCH_TOKEN_PREFIX,
        EPOCH_SESSION_AAD,
        &format!("{user_id}|{epoch}|{expires_at}"),
    )
}

pub(super) fn open_claims(
    cipher: &Aes256Gcm,
    token: &str,
    now: u64,
) -> Result<SessionClaims, AuthError> {
    if token.len() > MAX_SESSION_COOKIE_BYTES {
        return Err(AuthError::SessionDecryptionError(
            "Session token too long".to_string(),
        ));
    }
    let claims = if let Some(encoded) = token.strip_prefix(EPOCH_TOKEN_PREFIX) {
        parse_epoch_payload(&open_envelope(cipher, encoded, EPOCH_SESSION_AAD)?)?
    } else if let Some(encoded) = token.strip_prefix(SESSION_TOKEN_PREFIX) {
        parse_legacy_payload(&open_envelope(cipher, encoded, SESSION_AAD)?)?
    } else {
        return Err(AuthError::SessionDecryptionError(
            "Unsupported session token version".to_string(),
        ));
    };
    if now > claims.expires_at {
        return Err(AuthError::SessionExpired);
    }
    Ok(claims)
}

/// `user_id|epoch|expires_at`, exactly three fields.
fn parse_epoch_payload(payload: &str) -> Result<SessionClaims, AuthError> {
    let mut fields = payload.split('|');
    match (fields.next(), fields.next(), fields.next(), fields.next()) {
        (Some(user_id), Some(epoch), Some(expires_at), None) => Ok(SessionClaims {
            user_id: parse_field(user_id)?,
            epoch: parse_field(epoch)?,
            expires_at: parse_field(expires_at)?,
        }),
        _ => Err(invalid_payload()),
    }
}

/// `user_id|expires_at` from [`encrypt_session`](super::encrypt_session).
fn parse_legacy_payload(payload: &str) -> Result<SessionClaims, AuthError> {
    let (user_id, expires_at) = payload.split_once('|').ok_or_else(invalid_payload)?;
    Ok(SessionClaims {
        user_id: parse_field(user_id)?,
        epoch: 0,
        expires_at: parse_field(expires_at)?,
    })
}

fn parse_field<T>(field: &str) -> Result<T, AuthError>
where
    T: FromStr,
    T::Err: Display,
{
    field
        .parse::<T>()
        .map_err(|e| AuthError::SessionDecryptionError(e.to_string()))
}

fn invalid_payload() -> AuthError {
    AuthError::SessionDecryptionError("Invalid versioned session payload".to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
