//! The values the registry hands to the application.

use super::super::session::login_cookie_value;
use super::super::session_claims::{DEFAULT_SESSION_MAX_AGE, SessionClaims};
use super::{DeviceSummary, SessionRegistryError};
use std::fmt;
use std::time::Duration;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Characters of a [`SessionRef`]: 16 bytes in base64url without padding.
const REFERENCE_LEN: usize = 22;

/// The management reference of one session, safe to show its account (for
/// example as the value of a "sign out this device" button).
///
/// It is derived from the session secret through a separate digest, so it
/// can neither authenticate as the session nor be turned into its cookie.
/// [`SessionRegistry::revoke_session`](super::SessionRegistry::revoke_session)
/// takes it together with the signed-in account's id, so a reference of
/// another account revokes nothing. Comparisons run in constant time.
#[derive(Clone, Eq)]
pub struct SessionRef(pub(super) String);

impl SessionRef {
    /// Parses a reference submitted back by a form. Parsing does not
    /// authorize anything.
    ///
    /// # Errors
    /// [`SessionRegistryError::InvalidInput`] unless `value` is 22 base64url
    /// characters.
    pub fn parse(value: &str) -> Result<Self, SessionRegistryError> {
        if valid_reference(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(SessionRegistryError::InvalidInput("session reference"))
        }
    }

    /// The reference as text: 22 base64url characters.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub(super) fn valid_reference(value: &str) -> bool {
    value.len() == REFERENCE_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

impl PartialEq for SessionRef {
    fn eq(&self, other: &Self) -> bool {
        bool::from(self.0.as_bytes().ct_eq(other.0.as_bytes()))
    }
}

impl fmt::Display for SessionRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl fmt::Debug for SessionRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("SessionRef").field(&self.0).finish()
    }
}

/// A login to register: the account, its session epoch, the cookie
/// lifetime and the device summary to list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSession {
    pub(super) user_id: i32,
    pub(super) epoch: u64,
    pub(super) max_age: Duration,
    pub(super) device: DeviceSummary,
}

impl NewSession {
    /// A login of `user_id` at its current session `epoch` (pass 0 when the
    /// application keeps no epoch), valid for
    /// [`DEFAULT_SESSION_MAX_AGE`] on an unknown device.
    pub fn new(user_id: i32, epoch: u64) -> Self {
        Self {
            user_id,
            epoch,
            max_age: DEFAULT_SESSION_MAX_AGE,
            device: DeviceSummary::unknown(),
        }
    }

    /// The cookie and record lifetime: whole seconds, from 1 second to
    /// [`MAX_SESSION_MAX_AGE`](crate::MAX_SESSION_MAX_AGE); checked when
    /// the session is created.
    #[must_use]
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    /// The device summary the session list shows, usually
    /// [`DeviceSummary::from_headers`] of the sign-in request.
    #[must_use]
    pub fn with_device(mut self, device: DeviceSummary) -> Self {
        self.device = device;
        self
    }
}

/// A newly registered session: its login token and when it expires.
pub struct IssuedSession {
    pub(super) token: Zeroizing<String>,
    pub(super) reference: SessionRef,
    pub(super) expires_at: u64,
    pub(super) max_age_seconds: u64,
}

impl IssuedSession {
    /// The `rullst_session` cookie value. Send it only in the
    /// `Set-Cookie` of [`set_cookie`](Self::set_cookie).
    pub fn token(&self) -> &str {
        &self.token
    }

    /// The session's management reference, as the list shows it.
    pub fn reference(&self) -> &SessionRef {
        &self.reference
    }

    /// Expiry as Unix time in seconds; the session is valid through it.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// The `Set-Cookie` value: `rullst_session`, `Path=/`, `HttpOnly`,
    /// `SameSite=Lax`, the session's `Max-Age` and, when `secure`, `Secure`
    /// (as [`make_login_cookie_with_epoch`](crate::make_login_cookie_with_epoch)
    /// sets it outside development).
    pub fn set_cookie(&self, secure: bool) -> String {
        login_cookie_value(&self.token, self.max_age_seconds, secure)
    }
}

impl fmt::Debug for IssuedSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IssuedSession")
            .field("token", &"[REDACTED]")
            .field("reference", &self.reference)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// The registered session of the current request, as
/// [`SessionRegistry::authenticate`](super::SessionRegistry::authenticate)
/// found it. Insert it into the request's extensions for the handlers that
/// sign out, list or revoke sessions.
#[derive(Clone, PartialEq, Eq)]
pub struct CurrentSession {
    pub(super) user_id: i32,
    pub(super) token_hash: String,
    pub(super) reference: SessionRef,
    pub(super) created_at: u64,
    pub(super) expires_at: u64,
}

impl CurrentSession {
    /// The signed-in account.
    pub fn user_id(&self) -> i32 {
        self.user_id
    }

    /// This session's management reference.
    pub fn reference(&self) -> &SessionRef {
        &self.reference
    }

    /// When the session was created, as Unix time in seconds.
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Expiry as Unix time in seconds.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
}

impl fmt::Debug for CurrentSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CurrentSession")
            .field("user_id", &self.user_id)
            .field("reference", &self.reference)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// A request whose `rullst_session` cookie names a live registered session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedSession {
    pub(super) claims: SessionClaims,
    pub(super) current: CurrentSession,
}

impl AuthenticatedSession {
    /// What the cookie says: the account, its session epoch when the cookie
    /// was issued and its expiry. Check the epoch with
    /// [`SessionClaims::verify_epoch`] when the application keeps one.
    pub fn claims(&self) -> SessionClaims {
        self.claims
    }

    /// The signed-in account.
    pub fn user_id(&self) -> i32 {
        self.claims.user_id
    }

    /// The registered session, for sign-out and the session list.
    pub fn current(&self) -> &CurrentSession {
        &self.current
    }

    /// The registered session, by value.
    pub fn into_current(self) -> CurrentSession {
        self.current
    }
}

/// One live session of an account, as
/// [`SessionRegistry::list_sessions`](super::SessionRegistry::list_sessions)
/// returns it. No IP address, raw User-Agent or activity history is stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionInfo {
    pub(super) reference: SessionRef,
    pub(super) created_at: u64,
    pub(super) last_seen_at: u64,
    pub(super) expires_at: u64,
    pub(super) device: DeviceSummary,
}

impl SessionInfo {
    /// The reference to pass to
    /// [`SessionRegistry::revoke_session`](super::SessionRegistry::revoke_session).
    pub fn reference(&self) -> &SessionRef {
        &self.reference
    }

    /// When the session was created (the sign-in), as Unix time in seconds.
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// The start of the hour in which the session was last used, as Unix
    /// time in seconds (see
    /// [`LAST_SEEN_GRANULARITY`](super::LAST_SEEN_GRANULARITY)).
    pub fn last_seen_at(&self) -> u64 {
        self.last_seen_at
    }

    /// Expiry as Unix time in seconds.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// The browser and platform recorded at sign-in.
    pub fn device(&self) -> DeviceSummary {
        self.device
    }

    /// Whether this is the request's own session.
    pub fn is_current(&self, current: &CurrentSession) -> bool {
        self.reference == current.reference
    }
}
