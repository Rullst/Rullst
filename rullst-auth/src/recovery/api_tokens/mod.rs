//! Opaque application API credentials with authoritative revocation. Hosts own
//! tenant/MFA authorization and must recheck domain permissions on every request.
//! Provider credentials, OAuth tokens and browser cookies are different purposes.
mod http;
mod management;
mod storage;
mod types;
mod verification;

use super::{
    AuthClock, AuthenticatedRecoveryAccount, RecoveryError, RecoverySecrets, SecretToken,
    SessionLabel, SqlRecoveryStore,
};
pub use http::ApiTokenVerifier;
pub use types::{
    ApiScopes, ApiTokenConfig, ApiTokenId, ApiTokenMetadata, ApiTokenPrincipal, IssuedApiToken,
};

#[derive(Clone)]
pub struct ApiTokenService {
    store: SqlRecoveryStore,
    config: ApiTokenConfig,
    postgres: bool,
}

async fn bounded<T>(
    operation: impl std::future::Future<Output = Result<T, RecoveryError>>,
) -> Result<T, RecoveryError> {
    tokio::time::timeout(std::time::Duration::from_secs(10), operation)
        .await
        .map_err(|_| RecoveryError::Storage)?
}
fn now(clock: &impl AuthClock) -> Result<i64, RecoveryError> {
    super::timestamp(clock.now()?)
}
