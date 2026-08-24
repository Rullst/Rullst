//! The Rullst Connect Prelude
//!
//! A convenient module to import everything you need to authenticate users via OAuth2.
//!
//! ```rust,ignore
//! use rullst_connect::prelude::*;
//! ```

pub use crate::error::ConnectError;
pub use crate::provider::Provider;
pub use crate::providers::*;
pub use crate::user::ConnectUser;
pub use secrecy::{ExposeSecret, SecretString};

#[cfg(any(feature = "axum", feature = "actix"))]
pub use crate::extractors::AuthCallback;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_exports() {
        // Verify that core types are in scope via the prelude
        let _err = ConnectError::Provider("test".to_string());

        #[cfg(any(feature = "axum", feature = "actix"))]
        let _cb = AuthCallback {
            code: None,
            state: None,
            error: None,
            error_description: None,
        };
    }
}
