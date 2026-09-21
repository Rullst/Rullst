//! Trusted exercise and grading contracts. This crate never executes learner code.
//! The first Linux runner and durable integration remain implementation work.
#![forbid(unsafe_code)]

mod authorization;
mod diagnostic;
mod error;
mod exercise;
mod identity;
mod limits;
mod profile;
mod protocol;
mod receipt;
mod source;
mod submission;

pub use authorization::*;
pub use diagnostic::*;
pub use error::LabError;
pub use exercise::*;
pub use identity::*;
pub use limits::*;
pub use profile::*;
pub use protocol::*;
pub use receipt::*;
pub use source::*;
pub use submission::*;

#[cfg(feature = "sqlite")]
pub mod sqlite;

/// Wire semantics must be versioned independently from crate semver.
pub const PROTOCOL_VERSION: u16 = 1;
/// Only this bounded profile is selected. No shell/native/WASI profile is implied.
pub const PROFILE: &str = "rust-function-wasm-v1";
pub const TOOLCHAIN: &str = "rust-1.96.0-wasm32-unknown-unknown";
pub const INTERPRETER: &str = "wasmi-2.0.0-validated-deterministic-v1";
