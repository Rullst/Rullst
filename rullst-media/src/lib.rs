//! Explicit private video contracts. Provider and persistence are opt-in.
#![forbid(unsafe_code)]

mod contracts;
mod error;
mod provider;

pub use contracts::*;
pub use error::MediaError;
pub use provider::*;

#[cfg(feature = "bunny")]
pub mod bunny;
/// Dependency-free browser TUS module. Serve as an external JavaScript module
/// under the host's CSP; never interpolate this source or credentials into HTML.
#[cfg(feature = "bunny")]
pub const BUNNY_UPLOAD_MODULE: &str = include_str!("../web/bunny-upload.mjs");
#[cfg(feature = "sqlite")]
pub mod sqlite;
