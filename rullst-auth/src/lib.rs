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
