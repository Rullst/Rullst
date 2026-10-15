//! Nexus access decided by the application's own sign-in.
//!
//! [`NexusAuthorizer`] is the application's check: it reads the request's
//! session (a cookie, a header or an extension inserted by an outer layer) and
//! answers with a [`NexusAccess`]. Nexus asks on every request, before any
//! data is read or rendered, so a signed-out or demoted administrator loses
//! access on their next request, fragments and direct URLs included.

use std::{fmt, future::Future, pin::Pin, sync::Arc};

use axum::{
    extract::Request,
    http::{HeaderValue, Method, StatusCode, header, request::Parts},
    middleware::Next,
    response::Response,
};

use super::{NexusBuildError, status_response};
use crate::nexus::principal::NexusPrincipal;

/// Page a signed-out request is sent to unless
/// [`NexusApplicationAuth::with_login_path`] chooses another.
const DEFAULT_LOGIN_PATH: &str = "/login";
/// Longest accepted actor ID: the width of the audit `actor_id` column.
const MAX_ACTOR_ID_BYTES: usize = 255;
const MAX_LOGIN_PATH_BYTES: usize = 512;

/// The application's answer about one Nexus request.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NexusAccess {
    /// A signed-in administrator. `actor_id` names them in Nexus's mutation
    /// audit: 1 to 255 bytes without surrounding whitespace or control
    /// characters, for example `user:42`. Any other value denies the request
    /// with `503 Service Unavailable`.
    Allow { actor_id: String },
    /// Nobody is signed in, or the session is no longer valid. A page request
    /// (`GET` or `HEAD` without `HX-Request`) is redirected to the login page
    /// with `303 See Other`; any other request gets `401 Unauthorized` with an
    /// `HX-Redirect` header naming the login page.
    SignIn,
    /// A signed-in user who may not administer Nexus: `403 Forbidden`.
    Deny,
}

impl NexusAccess {
    /// Admits the administrator identified by `actor_id`.
    pub fn allow(actor_id: impl Into<String>) -> Self {
        Self::Allow {
            actor_id: actor_id.into(),
        }
    }
}

impl fmt::Debug for NexusAccess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allow { actor_id } => formatter
                .debug_struct("Allow")
                .field("actor_id_bytes", &actor_id.len())
                .finish(),
            Self::SignIn => formatter.write_str("SignIn"),
            Self::Deny => formatter.write_str("Deny"),
        }
    }
}

/// The application could not decide (its session store or database is
/// unavailable, for example). Nexus fails closed: it logs the reason and
/// answers `503 Service Unavailable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NexusAuthorizerError {
    reason: String,
}

impl NexusAuthorizerError {
    /// An error with an operator-facing reason, which Nexus logs. Keep
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

impl fmt::Display for NexusAuthorizerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Nexus authorizer failed: {}", self.reason)
    }
}

impl std::error::Error for NexusAuthorizerError {}

/// Decides whether the user signed in to the application may use Nexus.
///
/// Nexus calls [`NexusAuthorizer::authorize`] for every request under its
/// mount point, with the request's method, URI, headers and extensions. Check
/// the session there (a revoked session must not pass) and then the
/// administrator permission. Return [`NexusAccess::SignIn`] when nobody is
/// signed in, [`NexusAccess::Deny`] for a user without the permission, and an
/// error when the decision cannot be made; Nexus never admits a request on an
/// error.
///
/// ```
/// use rullst_nexus::{NexusAccess, NexusAuthorizer, NexusAuthorizerError};
/// use axum::http::request::Parts;
///
/// /// Admits requests carrying the header an outer layer sets for administrators.
/// struct HeaderAdmins;
///
/// impl NexusAuthorizer for HeaderAdmins {
///     async fn authorize(&self, request: &Parts) -> Result<NexusAccess, NexusAuthorizerError> {
///         Ok(match request.headers.get("x-verified-admin") {
///             Some(id) => NexusAccess::allow(id.to_str().unwrap_or_default()),
///             None => NexusAccess::SignIn,
///         })
///     }
/// }
/// ```
pub trait NexusAuthorizer: Send + Sync + 'static {
    /// Decides about one request.
    fn authorize(
        &self,
        request: &Parts,
    ) -> impl Future<Output = Result<NexusAccess, NexusAuthorizerError>> + Send;
}

type AuthorizeFuture<'a> =
    Pin<Box<dyn Future<Output = Result<NexusAccess, NexusAuthorizerError>> + Send + 'a>>;

/// [`NexusAuthorizer`] behind a pointer, so the policy enum stays one type.
trait ErasedAuthorizer: Send + Sync {
    fn authorize_erased<'a>(&'a self, request: &'a Parts) -> AuthorizeFuture<'a>;
}

impl<A: NexusAuthorizer> ErasedAuthorizer for A {
    fn authorize_erased<'a>(&'a self, request: &'a Parts) -> AuthorizeFuture<'a> {
        Box::pin(self.authorize(request))
    }
}

/// Nexus access through the application's sign-in: see [`NexusAuthorizer`].
#[derive(Clone)]
#[non_exhaustive]
pub struct NexusApplicationAuth {
    authorizer: Arc<dyn ErasedAuthorizer>,
    login_path: Arc<str>,
}

impl NexusApplicationAuth {
    /// Delegates every Nexus request to `authorizer`; signed-out page
    /// requests are sent to `/login`.
    pub fn new(authorizer: impl NexusAuthorizer) -> Self {
        Self {
            authorizer: Arc::new(authorizer),
            login_path: Arc::from(DEFAULT_LOGIN_PATH),
        }
    }

    /// Sends signed-out requests to `path` instead of `/login`. It must be a
    /// local path such as `/account/sign-in?return=nexus` (at most 512
    /// visible ASCII characters, starting with a single `/`, no `\`);
    /// building the router fails with
    /// [`NexusBuildError::InvalidLoginPath`] otherwise.
    pub fn with_login_path(mut self, path: impl Into<String>) -> Self {
        self.login_path = Arc::from(path.into());
        self
    }

    /// The page signed-out requests are sent to.
    pub fn login_path(&self) -> &str {
        &self.login_path
    }
}

impl fmt::Debug for NexusApplicationAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NexusApplicationAuth")
            .field("login_path", &self.login_path)
            .finish_non_exhaustive()
    }
}

/// A login path that cannot redirect off-site or break the `Location` header.
pub(super) fn validate_login_path(path: &str) -> Result<(), NexusBuildError> {
    let local = path.len() <= MAX_LOGIN_PATH_BYTES
        && path.starts_with('/')
        && !path.starts_with("//")
        && path
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'\\');
    if local {
        Ok(())
    } else {
        Err(NexusBuildError::InvalidLoginPath)
    }
}

/// An actor ID the audit can store unchanged.
fn valid_actor_id(actor_id: &str) -> bool {
    !actor_id.is_empty()
        && actor_id.len() <= MAX_ACTOR_ID_BYTES
        && actor_id.trim().len() == actor_id.len()
        && !actor_id.chars().any(char::is_control)
}

pub(super) async fn application_middleware(
    access: NexusApplicationAuth,
    request: Request,
    next: Next,
) -> Response {
    let (mut parts, body) = request.into_parts();
    match access.authorizer.authorize_erased(&parts).await {
        Ok(NexusAccess::Allow { actor_id }) if valid_actor_id(&actor_id) => {
            parts
                .extensions
                .insert(NexusPrincipal::authenticated(actor_id));
            next.run(Request::from_parts(parts, body)).await
        }
        Ok(NexusAccess::Allow { .. }) => {
            tracing::error!("Nexus authorizer returned an invalid actor ID; request denied");
            status_response(StatusCode::SERVICE_UNAVAILABLE)
        }
        Ok(NexusAccess::SignIn) => sign_in_response(&parts, &access.login_path),
        Ok(NexusAccess::Deny) => status_response(StatusCode::FORBIDDEN),
        Err(error) => {
            tracing::error!(
                reason = error.reason(),
                "Nexus authorizer failed; request denied"
            );
            status_response(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

/// Sends a browser page request to the login page; a fragment or mutation
/// gets `401` and an `HX-Redirect` that htmx and `nexus.js` follow.
fn sign_in_response(parts: &Parts, login_path: &str) -> Response {
    let location = HeaderValue::from_str(login_path)
        .unwrap_or_else(|_| HeaderValue::from_static(DEFAULT_LOGIN_PATH));
    let page = matches!(parts.method, Method::GET | Method::HEAD)
        && !parts.headers.contains_key("hx-request");
    if page {
        let mut response = status_response(StatusCode::SEE_OTHER);
        response.headers_mut().insert(header::LOCATION, location);
        response
    } else {
        let mut response = status_response(StatusCode::UNAUTHORIZED);
        response.headers_mut().insert("hx-redirect", location);
        response
    }
}

#[cfg(test)]
#[path = "application_tests.rs"]
mod tests;
