//! Server-side registry of login sessions: `rullst_session` cookies that a
//! sign-out, a device list or a password reset can revoke one at a time.
//!
//! A [`make_login_cookie_with_epoch`](crate::make_login_cookie_with_epoch)
//! cookie is stateless: deleting it from the browser does not stop a copied
//! cookie, and the session epoch revokes every cookie of an account at once.
//! A registered session adds a record per login in the application's own
//! database, so one session can be listed and revoked:
//!
//! - [`SessionRegistry::create_session`] (or [`SessionRegistry::sign_in`])
//!   stores the record and returns a `v3.` cookie carrying the account, its
//!   session epoch, the expiry and a random 256-bit session secret. The
//!   table keeps only a SHA-256 digest of the secret, so a copy of the
//!   database yields no cookie; the cookie is also sealed with `APP_KEY`.
//! - [`SessionRegistry::authenticate`] (or
//!   [`authenticate_request`](SessionRegistry::authenticate_request)) runs
//!   on every request: one primary-key lookup, and at most one write per
//!   session per hour to move its coarse last-seen time. There is no cache,
//!   so a revocation takes effect on the next request of every process that
//!   shares the database.
//! - [`SessionRegistry::sign_out`] and [`SessionRegistry::revoke`] delete the
//!   current record: a copy of the cookie stops working.
//!   [`list_sessions`](SessionRegistry::list_sessions),
//!   [`revoke_session`](SessionRegistry::revoke_session),
//!   [`revoke_other_sessions`](SessionRegistry::revoke_other_sessions) and
//!   [`revoke_all_sessions`](SessionRegistry::revoke_all_sessions) manage the
//!   account's other devices; call the last one on a password reset.
//! - [`purge_expired`](SessionRegistry::purge_expired) removes expired
//!   records in bounded batches; schedule it.
//!
//! [`decrypt_session_claims`](crate::decrypt_session_claims) refuses `v3.`
//! tokens, so a route that forgot the registry cannot accept a revoked
//! session. The session epoch keeps working: the cookie carries it and
//! [`AuthenticatedSession::claims`] returns it for
//! [`SessionClaims::verify_epoch`](crate::SessionClaims::verify_epoch).
//!
//! Each record stores the account id, the creation time, the last-seen hour,
//! the expiry and a [`DeviceSummary`] (browser and platform family). No IP
//! address, raw `User-Agent` or request history is stored. Each account keeps
//! at most [`MAX_SESSIONS_PER_USER`] sessions; a sign-in beyond that signs
//! out its least recently used one. The table has no foreign key to the
//! application's accounts, so an account deleted with SQL keeps its records
//! until they expire unless the migration that owns the table adds one
//! ([`SessionRegistry::create_table`]).
//!
//! The registry uses the application's SQLx pool and the table
//! `rullst_auth_sessions`, which [`SessionRegistry::open`] creates when it is
//! missing: SQLite (`session-registry-sqlite`; processes on one host sharing
//! the database file), PostgreSQL (`session-registry-postgres`) or
//! MySQL/MariaDB (`session-registry-mysql`). Every database call is bounded
//! by a five-second deadline and fails closed with
//! [`SessionRegistryError::Unavailable`].
//!
//! ```no_run
//! # #[cfg(feature = "session-registry-sqlite")]
//! # async fn example(pool: sqlx::SqlitePool, headers: axum::http::HeaderMap)
//! # -> Result<(), rullst_auth::session_registry::SessionRegistryError> {
//! use rullst_auth::session_registry::{DeviceSummary, NewSession, SessionRegistry};
//!
//! let registry = SessionRegistry::open(pool).await?;
//! // After the password check: the `Set-Cookie` of a registered session.
//! let login = NewSession::new(42, 0).with_device(DeviceSummary::from_headers(&headers));
//! let set_cookie = registry.sign_in(&login).await?;
//! // On each request:
//! let session = registry.authenticate_request(&headers).await?;
//! assert_eq!(session.user_id(), 42);
//! // "Your devices", then "sign out of all other devices":
//! let devices = registry.list_sessions(session.user_id(), rullst_auth::session_registry::unix_now()?).await?;
//! registry.revoke_other_sessions(session.current()).await?;
//! # let _ = (set_cookie, devices);
//! # Ok(())
//! # }
//! ```

mod device;
mod registry;
mod revalidation;
pub(crate) mod sql;
mod token;
mod types;

#[cfg(feature = "session-registry-mysql")]
mod mysql;
#[cfg(feature = "session-registry-postgres")]
mod postgres;
#[cfg(feature = "session-registry-sqlite")]
mod sqlite;

pub use device::{Browser, DeviceSummary, Platform};
pub use registry::{SessionRegistry, unix_now};
pub use revalidation::SessionRevalidation;
pub use sql::SessionDatabase;
pub use types::{
    AuthenticatedSession, CurrentSession, IssuedSession, NewSession, SessionInfo, SessionRef,
};

/// A [`SessionRegistry`] over a `SqlitePool`.
#[cfg(feature = "session-registry-sqlite")]
pub type SqliteSessionRegistry = SessionRegistry<sqlx::Sqlite>;
/// A [`SessionRegistry`] over a `PgPool`.
#[cfg(feature = "session-registry-postgres")]
pub type PostgresSessionRegistry = SessionRegistry<sqlx::Postgres>;
/// A [`SessionRegistry`] over a `MySqlPool`.
#[cfg(feature = "session-registry-mysql")]
pub type MySqlSessionRegistry = SessionRegistry<sqlx::MySql>;

/// An SQLx pool type the registry can use, so an application names the
/// registry of its pool without naming the database:
/// `SessionRegistryFor<rullst::orm::RullstPool>` is the registry of the
/// ORM's strict backend.
pub trait RegistryPool {
    /// The pool's database.
    type Database: SessionDatabase;
}

impl<DB: SessionDatabase> RegistryPool for sqlx::Pool<DB> {
    type Database = DB;
}

/// The [`SessionRegistry`] over the pool type `P`, for example
/// `SessionRegistryFor<sqlx::SqlitePool>`.
pub type SessionRegistryFor<P> = SessionRegistry<<P as RegistryPool>::Database>;

/// Sessions an account keeps; the sign-in after that signs out the least
/// recently used one.
pub const MAX_SESSIONS_PER_USER: u32 = 50;

/// How precisely a session's last use is recorded: the start of the hour,
/// written at most once an hour per session.
pub const LAST_SEEN_GRANULARITY_SECONDS: u64 = 3600;

/// Largest batch [`SessionRegistry::purge_expired`] deletes at once.
pub const MAX_PURGE_BATCH: u32 = 1000;

/// Why a registry operation failed. None of the variants carries a cookie,
/// session secret, digest, account data or database message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SessionRegistryError {
    /// The cookie is not a registered-session token of this `APP_KEY`: it is
    /// absent, altered, of another version or sealed with another key.
    #[error("the session cookie is not a valid registered session")]
    InvalidToken,
    /// The session's lifetime ended.
    #[error("the session expired")]
    Expired,
    /// The session was signed out, revoked, evicted or purged, or its record
    /// belongs to another account.
    #[error("the session was signed out or revoked")]
    Revoked,
    /// A parameter is out of range; the text names it.
    #[error("invalid session registry input: {0}")]
    InvalidInput(&'static str),
    /// `APP_KEY` or the `RULLST_ENV` environment is missing or invalid.
    #[error("the application key or environment is missing or invalid")]
    Configuration,
    /// The random number generator, the clock or token sealing failed.
    #[error("the session could not be sealed")]
    Crypto,
    /// The database failed or did not answer within five seconds. A write
    /// may or may not have been applied; authentication fails closed.
    #[error("session storage is unavailable")]
    Unavailable,
    /// A stored row failed validation when it was read back.
    #[error("a stored session is corrupt")]
    Corrupt,
}

impl SessionRegistryError {
    /// Whether the request should be sent to sign in (an invalid, expired or
    /// revoked session) rather than answered with an error page.
    pub fn is_signed_out(self) -> bool {
        matches!(self, Self::InvalidToken | Self::Expired | Self::Revoked)
    }
}

#[cfg(all(test, feature = "session-registry-sqlite"))]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
