//! Transparent, optional supervision with explicit host authorization boundaries.
//! Context constructors validate shape, not identity or guardian relationships.
//! The host authenticates each request before calling this library.
//!
//! **Maturity: Experimental**
//! ([tiers](https://rullst.github.io/Rullst/book/maturity.html)). The API may
//! change between 13.x releases. This crate is for consented, transparent exams
//! and parental controls over a learning application; it has no covert capture
//! mode.
//!
//! No feature is enabled by default; the default build holds only the shared
//! identity, authority and configuration types such as [`Context`] and
//! [`AuthorityGrant`]. Enable `exam` for typed exam sessions and observations,
//! `parental` for course/window policies, `sqlite` for both plus the
//! shared-local durable store (`sqlite::SqliteSupervision`), and `analysis`
//! for bounded camera-presence/audio-activity adapter contracts. Read
//! [Responsible use](https://rullst.github.io/Rullst/book/supervision.html#responsible-use)
//! before integrating it.
#![cfg_attr(docsrs, feature(doc_cfg))]
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
