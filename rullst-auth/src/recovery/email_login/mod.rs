//! Explicit account-opt-in email login, independent from password recovery.
//! The host owns tenant authorization, MFA policy, CSRF, secure cookies and
//! no-store/no-referrer responses. GET/HEAD must never invoke redemption.

use super::connection;
mod delivery;
mod flow;
mod guard;
mod storage;
mod types;

pub use delivery::EmailLoginDelivery;

pub use types::{
    BrowserBinding, EmailLoginClock, EmailLoginConfig, EmailLoginSession, LoginRequestAccepted,
    SystemEmailLoginClock,
};

use super::{
    AuthenticatedRecoveryAccount, RecoveryError, RecoverySecrets, SecretToken, SqlRecoveryStore,
};

/// Login service sharing the authoritative account and opaque-session registry.
/// Application/tenant namespaces and callback destinations are trusted server
/// configuration, never authority supplied by unauthenticated request fields.
#[derive(Clone)]
pub struct EmailLoginService {
    store: SqlRecoveryStore,
    config: EmailLoginConfig,
    postgres: bool,
}

async fn bounded<T>(
    operation: impl std::future::Future<Output = Result<T, RecoveryError>>,
) -> Result<T, RecoveryError> {
    tokio::time::timeout(std::time::Duration::from_secs(10), operation)
        .await
        .map_err(|_| RecoveryError::Storage)?
}

fn now(clock: &impl EmailLoginClock) -> Result<i64, RecoveryError> {
    super::timestamp(clock.now()?)
}
