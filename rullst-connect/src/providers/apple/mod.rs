//! Apple OAuth2 & Sign in with Apple integration module.

pub mod provider;
pub(crate) mod traits;
pub(crate) mod types;

#[cfg(test)]
mod tests;

pub use provider::*;
