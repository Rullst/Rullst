//! Monitor access decided by the application's own sign-in.

use axum::http::request::Parts;
use std::{fmt, future::Future, pin::Pin};

/// The application's answer about one monitor request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MonitorAccess {
    /// A signed-in user who may read the monitor.
    Allow,
    /// Nobody is signed in, or the session is no longer valid:
    /// `401 Unauthorized`, the same answer as a missing token.
    SignIn,
    /// A signed-in user without the monitoring permission: `403 Forbidden`.
    Deny,
}

/// The application could not decide (its session store is unavailable, for
/// example). The monitor fails closed: it logs the reason and answers
/// `503 Service Unavailable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorAuthorizerError {
    reason: String,
}

impl MonitorAuthorizerError {
    /// An error with an operator-facing reason, which the monitor logs. Keep
    /// secrets and personal data out of it.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    /// The reason given by the application.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl fmt::Display for MonitorAuthorizerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "monitor authorizer failed: {}", self.reason)
    }
}

impl std::error::Error for MonitorAuthorizerError {}

/// Decides whether the user signed in to the application may read the
/// monitor, the way `rullst_nexus::NexusAuthorizer` does for Nexus.
///
/// The monitor calls [`MonitorAuthorizer::authorize`] for every request that
/// does not carry a valid monitor token, with the request's method, URI,
/// headers and extensions. Check the session there (a revoked session must
/// not pass) and then the permission. Return [`MonitorAccess::SignIn`] when
/// nobody is signed in, [`MonitorAccess::Deny`] for a user without the
/// permission, and an error when the decision cannot be made; the monitor
/// never admits a request on an error.
///
/// ```
/// use rullst_core::http::request::Parts;
/// use rullst_core::monitor::{MonitorAccess, MonitorAuthorizer, MonitorAuthorizerError};
///
/// /// Admits requests carrying the header an outer layer sets for operators.
/// struct HeaderOperators;
///
/// impl MonitorAuthorizer for HeaderOperators {
///     async fn authorize(&self, request: &Parts) -> Result<MonitorAccess, MonitorAuthorizerError> {
///         Ok(match request.headers.get("x-verified-operator") {
///             Some(_) => MonitorAccess::Allow,
///             None => MonitorAccess::SignIn,
///         })
///     }
/// }
/// ```
pub trait MonitorAuthorizer: Send + Sync + 'static {
    /// Decides about one request.
    fn authorize(
        &self,
        request: &Parts,
    ) -> impl Future<Output = Result<MonitorAccess, MonitorAuthorizerError>> + Send;
}

type AuthorizeFuture<'a> =
    Pin<Box<dyn Future<Output = Result<MonitorAccess, MonitorAuthorizerError>> + Send + 'a>>;

/// [`MonitorAuthorizer`] behind a pointer, so [`super::Monitor`] stays one type.
pub(super) trait ErasedAuthorizer: Send + Sync {
    fn authorize_erased<'a>(&'a self, request: &'a Parts) -> AuthorizeFuture<'a>;
}

impl<A: MonitorAuthorizer> ErasedAuthorizer for A {
    fn authorize_erased<'a>(&'a self, request: &'a Parts) -> AuthorizeFuture<'a> {
        Box::pin(self.authorize(request))
    }
}
