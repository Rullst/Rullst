//! The Rullst Connect Prelude
//!
//! A convenient module to import everything you need to authenticate users via OAuth2.
//!
//! ```rust
//! use rullst_connect::prelude::*;
//! ```

pub use crate::error::ConnectError;
pub use crate::provider::{Provider, RevocationTokenKind};
pub use crate::providers::*;
pub use crate::refresh::{
    AccessTokenLease, AutoRefreshingSession, EncryptedTokenSnapshot, RefreshableTokenState,
    TokenSnapshotBinding, TokenSnapshotError, TokenSnapshotKey,
};
pub use crate::user::{ConnectUser, UniversalProfile};
pub use secrecy::{ExposeSecret, SecretString};

#[cfg(any(feature = "axum", feature = "actix"))]
pub use crate::extractors::AuthCallback;

#[cfg(feature = "axum-session")]
pub use crate::extractors::{
    AuthSession, OAuthAuthorization, begin_oauth_session, begin_oidc_session,
};

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

        #[cfg(feature = "axum-session")]
        let _authorization: Option<OAuthAuthorization> = None;
    }
}
