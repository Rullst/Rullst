//! Server-side OAuth/OIDC login challenges without `tower-sessions`.
//!
//! A login challenge is the server half of one authorization attempt: the
//! OAuth `state`, the PKCE verifier, the OIDC nonce, the provider name and an
//! optional same-site page to return to. [`begin_login`] stores it in a
//! [`ChallengeStore`] and returns the provider redirect plus an opaque,
//! high-entropy [`ChallengeId`]. The browser keeps only that id, in a short
//! `HttpOnly`, `SameSite=Lax` cookie ([`ChallengeCookie`]); the verifier and
//! nonce never leave the server. The callback consumes the challenge exactly
//! once and validates the provider response against it (`finish_login`, with
//! a web-framework feature).
//!
//! Stores: [`MemoryChallengeStore`] for tests and single-process development,
//! and `SqlChallengeStore` over the application's SQLx pool for SQLite
//! (`sqlite` feature), PostgreSQL (`postgres`) and MySQL (`mysql`). Every
//! store consumes a challenge with one atomic removal, so a replayed or
//! concurrent callback finds nothing.
//!
//! Login sessions remain the application's: after a successful callback, the
//! application maps the provider subject to its account and issues its own
//! session cookie (for Rullst, `make_login_cookie_with_epoch`).

mod cookie;
mod flow;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
mod sql;
mod store;
mod types;

pub use cookie::{CHALLENGE_COOKIE_NAME, ChallengeCookie};
#[cfg(any(
    feature = "axum",
    feature = "actix",
    feature = "leptos",
    feature = "rullst"
))]
pub use flow::{CompletedLogin, finish_login};
pub use flow::{LoginError, LoginOptions, LoginRedirect, begin_login};
#[cfg(feature = "mysql")]
pub use sql::MySqlChallengeStore;
#[cfg(feature = "postgres")]
pub use sql::PostgresChallengeStore;
#[cfg(feature = "sqlite")]
pub use sql::SqliteChallengeStore;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
pub use sql::{ChallengeDatabase, SqlChallengeStore};
pub use store::{ChallengeStore, MemoryChallengeStore};
pub use types::{ChallengeId, LoginChallenge, ReturnPath};

/// The longest a login challenge may live: ten minutes.
pub const MAX_CHALLENGE_LIFETIME_SECONDS: u64 = 600;

/// Typed challenge failures. None of them carries the challenge id, state,
/// nonce, verifier, database URL or provider response.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ChallengeError {
    /// A value failed validation before it was stored or used.
    #[error("invalid login challenge: {0}")]
    Invalid(&'static str),
    /// No challenge has this id: it was never issued, was already consumed,
    /// was purged, or the id was altered.
    #[error("login challenge was not found or was already used")]
    NotFound,
    /// The challenge existed but its lifetime ended; it is consumed anyway.
    #[error("login challenge expired")]
    Expired,
    /// The store holds its configured maximum of live challenges.
    #[error("login challenge capacity is exhausted")]
    CapacityExceeded,
    /// The backing store failed; the operation names what was attempted.
    #[error("login challenge storage operation failed: {0}")]
    StorageUnavailable(&'static str),
    /// A stored row failed validation when it was read back.
    #[error("login challenge storage is corrupt: {0}")]
    CorruptStorage(&'static str),
}
