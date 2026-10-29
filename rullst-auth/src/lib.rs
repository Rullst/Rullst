//! Authentication and authorization helpers for Rullst applications.
//!
//! - Passwords: Argon2id hashing on Tokio's blocking pool with
//!   [`hash_password_async`] and [`verify_password_async`]; [`needs_rehash`]
//!   flags hashes made with other parameters.
//! - Sessions: `rullst_session` cookies sealed with AES-256-GCM under
//!   `APP_KEY`. [`make_login_cookie_with_epoch`] and [`decrypt_session_claims`]
//!   carry a per-user session epoch that the application increments to sign
//!   out every device.
//! - Password reset: [`PasswordResetToken`] secrets, stored only as digests.
//! - Passkeys: WebAuthn ES256 ceremonies with
//!   [`PasskeyAuth`](auth::passkey::PasskeyAuth).
//! - Authorization: [`RequireRoleLayer`] for user types that implement
//!   [`HasRole`], and named [`Policy`] implementations for resource decisions.
//!
//! **Maturity: Extension**
//! ([tiers](https://rullst.github.io/Rullst/book/maturity.html)).
//!
//! No Cargo feature is enabled by default. `jwt` adds the application JWT
//! policy; `sqlite` adds durable SQLite JWT revocation and a passkey device
//! registry; `passkey-postgres` shares passkey ceremonies through PostgreSQL;
//! `recovery-sqlite` or `recovery-postgres` adds password recovery and
//! revocable server-side sessions, with the `email-login-*` and
//! `api-tokens-*` features on top; `oauth` re-exports `rullst-connect` as
//! `connect`. In a Rullst application, enable the `auth` feature of `rullst`.
//!
//! Guide: <https://rullst.github.io/Rullst/book/crates/auth.html>.

pub mod error;
pub use error::*;

pub mod auth;
pub use auth::*;

pub mod rbac;
pub use rbac::*;

pub mod policy;
pub use policy::*;

#[cfg(feature = "jwt")]
pub mod jwt;
#[cfg(feature = "jwt")]
pub use jwt::*;

/// Durable opt-in password recovery, revocable sessions and encrypted notices.
#[cfg(any(feature = "recovery-sqlite", feature = "recovery-postgres"))]
pub mod recovery;
