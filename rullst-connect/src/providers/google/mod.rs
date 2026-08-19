//! Google OAuth2 & OpenID Connect authentication provider module.

pub mod provider;
pub(crate) mod traits;
pub(crate) mod types;

#[cfg(test)]
mod tests;

pub use provider::*;
