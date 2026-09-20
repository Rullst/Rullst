//! Transparent, optional supervision with explicit host authorization boundaries.
//! Context constructors validate shape, not identity or guardian relationships.
//! The host authenticates each request before calling this library.

#![forbid(unsafe_code)]

mod authority;
mod clock;
mod config;
mod error;
mod identity;

#[cfg(feature = "analysis")]
pub mod analysis;

#[cfg(feature = "exam")]
pub mod exam;
#[cfg(feature = "parental")]
pub mod parental;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use authority::{AuthorityAction, AuthorityGrant, AuthorityKey};
pub use clock::{Clock, SystemClock};
pub use config::{Limits, StoreConfig};
pub use error::SupervisionError;
pub use identity::{Context, OpaqueId, Operator, Revision, Scope};
