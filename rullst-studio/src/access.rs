//! The loopback boundary in front of every Studio page.
//!
//! A request reaches a page only from a loopback peer (`ConnectInfo`) that
//! names a loopback `Host`; an unsafe method also needs a same-origin
//! `Origin`. Requests with a proxy forwarding header (`Forwarded`,
//! `X-Forwarded-For`, `-Host`, `-Proto` or `-Server`, `X-Real-IP`, `Via`, …;
//! not `X-Forwarded-Port` or `X-Client-IP`) or over HTTP/1.0 are refused,
//! because a same-host reverse proxy or tunnel connects from loopback, and so
//! are requests whose `Server` resolved staging or production. A proxy or
//! tunnel that adds no forwarding header and speaks HTTP/1.1 or newer still
//! looks like a local browser, so Studio must never be published through one.
//! [`LocalStudioAccess::protect_router`] also refuses to build in a release
//! build, or when the environment `Server` resolves for the project (process
//! `RULLST_ENV`, then `APP_ENV`, then `.env`, then `[app] env`) is staging or
//! production (an invalid value counts as one of them).

use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, Request},
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::Response,
};
use rullst_core::config::{Environment, project_environment};
use std::{fmt, net::SocketAddr};

/// Explicit capability for serving Studio to verified loopback peers in a
/// debug build outside staging and production.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct LocalStudioAccess {
    _private: (),
}

/// Request-local proof that the debug and loopback Studio boundary accepted
/// the peer, host authority, and (for unsafe methods) same-origin request.
///
/// This type remains crate-private so application code cannot manufacture the
/// marker and bypass the access middleware for privileged Studio handlers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VerifiedLocalStudioAccess;

/// Response of a privileged Studio handler reached without the marker, for
/// example through a raw router mounted outside [`LocalStudioAccess`].
#[cfg(feature = "local")]
pub(crate) fn verified_local_access_required() -> Response {
    let mut response = Response::new(Body::from("Verified local Studio access is required"));
    *response.status_mut() = StatusCode::FORBIDDEN;
    response
}

impl LocalStudioAccess {
    /// Opts in to the debug-only, loopback-verified Studio boundary.
    pub const fn loopback_only() -> Self {
        Self { _private: () }
    }

    /// Applies the Studio loopback boundary to a router.
    ///
    /// Fails with [`StudioBuildError::LocalAccessRequiresDebugBuild`] in a
    /// release build, and when the environment `Server` would resolve for the
    /// working directory (process `RULLST_ENV`, then `APP_ENV`, then `.env`,
    /// then `[app] env` in `Rullst.toml`) is staging, production or invalid.
    pub fn protect_router<S>(&self, router: Router<S>) -> Result<Router<S>, StudioBuildError>
    where
        S: Clone + Send + Sync + 'static,
    {
        if !cfg!(debug_assertions) || project_environment_requires_secure_defaults() {
            return Err(StudioBuildError::LocalAccessRequiresDebugBuild);
        }
        Ok(router.layer(axum::middleware::from_fn(loopback_only_middleware)))
    }
}

#[cfg(feature = "remote")]
/// Applies the loopback boundary without the debug-build requirement.
///
/// Only the remote-projects router uses it: that mode serves no data of a
/// local application (no database, `.env`, AI or API surfaces), only remote
/// monitoring documents, so the standalone `rullst-studio` binary can run as a
/// release build while keeping the loopback, `Host`, same-origin and marker
/// checks of every Studio page.
pub(crate) fn protect_remote_router<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(axum::middleware::from_fn(loopback_only_middleware))
}

/// Errors returned while constructing a Studio access boundary.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StudioBuildError {
    /// Credential-free Studio access is unavailable in release builds, and
    /// when the project's environment is staging or production.
    LocalAccessRequiresDebugBuild,
    /// The OS could not provide the key used for opaque local action tokens.
    RandomnessUnavailable,
}

impl fmt::Display for StudioBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalAccessRequiresDebugBuild => formatter.write_str(
                "Studio local access requires a debug build outside staging and production; shared environments need an application-owned authenticated boundary",
            ),
            Self::RandomnessUnavailable => formatter.write_str(
                "Studio could not obtain operating-system randomness for local action tokens",
            ),
        }
    }
}

impl std::error::Error for StudioBuildError {}

/// Referrer policy stamped on Studio responses.
///
/// Browsers derive a form POST's `Origin` from the submitting document's
/// policy: under `no-referrer` even a same-origin POST carries `Origin: null`,
/// which the check below cannot verify. `same-origin` keeps the real origin on
/// Studio's own form submissions and still sends no referrer to other origins.
const STUDIO_REFERRER_POLICY: &str = "same-origin";

/// The environment `Server` would resolve for this project (the process
/// `RULLST_ENV`, then `APP_ENV`, then either one from `.env`, then `[app] env`
/// in `Rullst.toml`) is staging or production, or cannot be resolved, which
/// fails closed.
fn project_environment_requires_secure_defaults() -> bool {
    project_environment().map_or(true, Environment::requires_secure_defaults)
}

/// A request refused even from a loopback peer: one that a same-host proxy
/// or tunnel relayed, or one that a `Server` running in staging or
/// production received.
fn relayed_or_in_secure_environment(request: &Request) -> bool {
    rullst_core::server::is_proxied_request(request)
        || request
            .extensions()
            .get::<Environment>()
            .is_some_and(|environment| environment.requires_secure_defaults())
}

async fn loopback_only_middleware(mut request: Request, next: Next) -> Response {
    // A dual-stack (`::`) listener reports IPv4 peers as IPv4-mapped IPv6
    // addresses (`::ffff:127.0.0.1`), which `Ipv6Addr::is_loopback` rejects.
    let is_loopback = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|connection| connection.0.ip().to_canonical().is_loopback());

    let local_host = local_host_authority(request.headers());
    let origin_valid = request
        .headers()
        .get(header::ORIGIN)
        .map(|origin| {
            same_origin(origin, local_host.as_deref())
                || opaque_origin_from_same_origin_document(origin, request.headers())
        })
        .unwrap_or(true);
    let unsafe_method = !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let unsafe_origin_present = !unsafe_method || request.headers().contains_key(header::ORIGIN);

    if is_loopback
        && !relayed_or_in_secure_environment(&request)
        && local_host.is_some()
        && origin_valid
        && unsafe_origin_present
    {
        request.extensions_mut().insert(VerifiedLocalStudioAccess);
        let mut response = next.run(request).await;
        response
            .headers_mut()
            .entry(header::CACHE_CONTROL)
            .or_insert(axum::http::HeaderValue::from_static("no-store"));
        response
            .headers_mut()
            .entry(header::X_CONTENT_TYPE_OPTIONS)
            .or_insert(axum::http::HeaderValue::from_static("nosniff"));
        response
            .headers_mut()
            .entry(header::X_FRAME_OPTIONS)
            .or_insert(axum::http::HeaderValue::from_static("DENY"));
        response
            .headers_mut()
            .entry(header::REFERRER_POLICY)
            .or_insert(axum::http::HeaderValue::from_static(STUDIO_REFERRER_POLICY));
        response
    } else {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::FORBIDDEN;
        response
    }
}

fn local_host_authority(headers: &HeaderMap) -> Option<String> {
    let authority = headers
        .get(header::HOST)?
        .to_str()
        .ok()?
        .parse::<axum::http::uri::Authority>()
        .ok()?;
    let host = authority.host();
    let ip_host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    let is_local = host.eq_ignore_ascii_case("localhost")
        || ip_host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.to_canonical().is_loopback());
    is_local.then(|| authority.as_str().to_ascii_lowercase())
}

/// Accepts `Origin: null` only when Fetch Metadata proves that a document of
/// this exact origin sent the request. Browsers send that combination for a
/// same-origin form POST when a host layer replaces Studio's referrer policy
/// with `no-referrer`. `Sec-Fetch-Site` is a forbidden request header that page
/// scripts cannot set; cross-site and same-site documents (for example another
/// local port) receive `cross-site` or `same-site` and stay rejected, as does a
/// bare `Origin: null`.
fn opaque_origin_from_same_origin_document(
    origin: &axum::http::HeaderValue,
    headers: &HeaderMap,
) -> bool {
    origin.as_bytes() == b"null"
        && headers
            .get("sec-fetch-site")
            .is_some_and(|site| site.as_bytes() == b"same-origin")
}

fn same_origin(origin: &axum::http::HeaderValue, local_authority: Option<&str>) -> bool {
    let Some(local_authority) = local_authority else {
        return false;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(origin) = origin.parse::<axum::http::Uri>() else {
        return false;
    };
    let scheme_allowed = origin
        .scheme_str()
        .is_some_and(|scheme| matches!(scheme, "http" | "https"));
    scheme_allowed
        && origin
            .authority()
            .is_some_and(|authority| authority.as_str().eq_ignore_ascii_case(local_authority))
}

#[cfg(test)]
#[path = "access_tests.rs"]
mod tests;
