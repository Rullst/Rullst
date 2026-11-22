//! CSRF protection middleware and Double Submit Cookie validation.

use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::distr::{Alphanumeric, SampleString};

use super::csrf_mask::{csrf_proof_matches, mask_csrf_token, valid_cookie_token};
use super::waf::security_config;

/// Generates a cryptographically secure 32-character random alphanumeric string.
pub fn generate_csrf_token() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 32)
}

/// Request-scoped CSRF token made available to handlers rendering forms.
///
/// It is the `rullst_csrf` cookie's token masked for this response (see
/// [`mask_csrf_token`]): on the first `GET` or `HEAD` request, the token
/// that the middleware writes to the response cookie. Exposing it through
/// request extensions avoids rendering an empty hidden field before the
/// browser has received that cookie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsrfToken(String);

impl CsrfToken {
    /// Masks the cookie's `token` for one response, or `None` when the
    /// operating system's random source fails to seed the mask generator.
    fn for_response(token: &str) -> Option<Self> {
        mask_csrf_token(token).map(Self)
    }

    /// Returns the value to embed in a page and echo in an `X-CSRF-Token`
    /// header or `_token` form field. Since 13.0.0-alpha.6 it is masked
    /// with a fresh random mask in every request, so it differs from the
    /// cookie value and between responses (a defence against BREACH in
    /// compressed pages); the middleware accepts it, and the cookie value
    /// itself, for the request's cookie.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The `500` answer when the operating system's random source cannot mint
/// or mask the token.
const MASK_UNAVAILABLE: &str = "CSRF token unavailable";

/// Private request marker preventing nested framework/application composition
/// from applying the double-submit protocol twice.
#[derive(Clone, Copy, Debug)]
struct CsrfMiddlewareApplied;

#[derive(serde::Deserialize)]
struct CsrfForm {
    _token: Option<String>,
}

/// Helper to extract the token from form-encoded body bytes.
pub(crate) fn extract_token_from_body(bytes: &[u8]) -> Option<String> {
    serde_urlencoded::from_bytes::<CsrfForm>(bytes)
        .ok()
        .and_then(|form| form._token)
}

/// Middleware that enforces CSRF protection using the Double Submit Cookie pattern.
/// GET and HEAD requests receive the request-scoped [`CsrfToken`] and generate a
/// CSRF cookie if missing; HEAD mirrors GET's headers (RFC 9110), so a `get`
/// route extracting `Extension<CsrfToken>` also answers HEAD. `OPTIONS` and
/// `TRACE` pass through, while state-changing requests must match the
/// `rullst_csrf` cookie token with either the `X-CSRF-Token` header or form
/// `_token` field. That value is the masked [`CsrfToken`] a page embedded,
/// which unmasks to the cookie token, or the cookie value itself (a client
/// that reads the cookie); the token bytes are compared in constant time.
/// Each request's [`CsrfToken`] carries a fresh mask, so a compressed page
/// never repeats the secret (BREACH). The cookie carries `Secure` in staging and production, and
/// in any environment when the request arrived over HTTPS (an `https`
/// request URI, or `https` reported by a trusted proxy through
/// [`super::TrustedProxyLayer`] with `trust_forwarded_proto`).
///
/// An `application/x-www-form-urlencoded` body is buffered (up to 1 MiB) to
/// read `_token`. A `multipart/form-data` body, such as a file upload, is read
/// only until its first `_token` field ends, at most 64 KiB, so that field
/// must precede the file inputs; the whole body then reaches the handler
/// unchanged. A multipart body without a readable `_token` field is refused
/// with a `403` that names this rule.
///
/// A `GET` or `HEAD` for a static file never mints a token or sets the
/// cookie. Static files are the paths below a
/// [`SecurityConfig::csrf_static_prefixes`](crate::config::SecurityConfig::csrf_static_prefixes)
/// entry (default `/static/` and `/.well-known/`) and the exact
/// [`SecurityConfig::csrf_static_paths`](crate::config::SecurityConfig::csrf_static_paths)
/// (default `/favicon.ico`, `/robots.txt`, `/sitemap.xml`,
/// `/manifest.webmanifest` and `/sw.js`), read from the request's
/// `SecurityConfig` extension or the process-global configuration. Every
/// other page gets a token on its first visit whatever its extension, so a
/// dynamic `/report.json` page can render a form; until 13.0.0-alpha.5 a
/// `.json`, `.xml`, `.txt`, `.js`, `.css` or image suffix also counted as
/// static. When the browser already sends a valid `rullst_csrf` cookie, a
/// static path still receives the [`CsrfToken`] from it. These paths do not
/// affect state-changing requests, which always need the token. Nor is the
/// cookie set on a response that shared caches may store (a `Cache-Control`
/// with `public` or `s-maxage` and without `private` or `no-store`, such as
/// one labelled by [`crate::cache_control::public`]).
///
/// Applying this middleware more than once to the same request is idempotent. This
/// matters when an application router adds the explicit development layer and
/// [`crate::Server`] later composes the production security baseline around it.
pub async fn csrf_middleware(req: Request, next: Next) -> Response {
    with_csrf(req, next).await
}

/// [`csrf_middleware`] in front of `next`.
pub(crate) async fn with_csrf(mut req: Request, next: impl super::Run) -> Response {
    if let Some(policy) = req
        .extensions()
        .get::<super::MachineEndpointPolicy>()
        .cloned()
        && policy.matches(&req)
    {
        return match policy.authenticate(req).await {
            Ok(request) => next.run(request).await,
            Err(status) => crate::server::refused(
                (status, "Machine request rejected").into_response(),
                crate::server::Refusal::MACHINE_ENDPOINT,
            ),
        };
    }
    if req.extensions().get::<CsrfMiddlewareApplied>().is_some() {
        return next.run(req).await;
    }

    if is_signed_webhook_exemption(&req) {
        return next.run(req).await;
    }

    req.extensions_mut().insert(CsrfMiddlewareApplied);

    let method = req.method();

    // Axum serves HEAD through the GET handler, so HEAD gets the same token
    // and cookie as GET.
    if method == axum::http::Method::GET || method == axum::http::Method::HEAD {
        handle_csrf_get(req, next).await
    } else if method == axum::http::Method::OPTIONS || method == axum::http::Method::TRACE {
        next.run(req).await
    } else {
        handle_csrf_state_modifying(req, next).await
    }
}

fn is_signed_webhook_exemption(req: &Request) -> bool {
    if req.method() != axum::http::Method::POST {
        return false;
    }

    security_config(req)
        .csrf_signed_webhook_paths
        .iter()
        .any(|path| path == req.uri().path())
}

async fn handle_csrf_get(mut req: Request, next: impl super::Run) -> Response {
    let cookie_token = csrf_token_from_cookies(req.headers());

    if let Some(token) = cookie_token {
        let Some(csrf) = CsrfToken::for_response(&token) else {
            return refusal(StatusCode::INTERNAL_SERVER_ERROR, MASK_UNAVAILABLE);
        };
        req.extensions_mut().insert(csrf);
        return next.run(req).await;
    }

    // Static files never mint a token or set the cookie.
    let security = security_config(&req);
    if security.is_csrf_static_path(req.uri().path()) {
        return next.run(req).await;
    }
    let same_site = security.csrf_same_site.as_str();

    // The cookie's secret comes from the operating system's random source.
    let Some(token) = super::csrf_random::mint_cookie_token() else {
        return refusal(StatusCode::INTERNAL_SERVER_ERROR, MASK_UNAVAILABLE);
    };
    let Some(csrf) = CsrfToken::for_response(&token) else {
        return refusal(StatusCode::INTERNAL_SERVER_ERROR, MASK_UNAVAILABLE);
    };
    // `Secure` in staging and production, and over HTTPS everywhere.
    let secure_cookie = req
        .extensions()
        .get::<crate::config::Environment>()
        .copied()
        .map(crate::config::Environment::requires_secure_defaults)
        .unwrap_or_else(|| {
            crate::config::Environment::detect(None)
                .map(crate::config::Environment::requires_secure_defaults)
                .unwrap_or(true)
        })
        || super::secure_cookies::request_is_https(req.uri(), req.extensions());
    // Written before the handler runs, while the configuration is borrowed
    // from the request.
    let cookie = csrf_cookie(&token, same_site, secure_cookie);
    req.extensions_mut().insert(csrf);

    let mut response = next.run(req).await;
    // A response that shared caches may store would hand this visitor's
    // token to others, so it never sets the cookie.
    if crate::cache_control::allows_shared_caching(response.headers()) {
        return response;
    }
    if let Ok(cookie) = header::HeaderValue::from_maybe_shared(axum::body::Bytes::from(cookie)) {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

/// Room for a `Set-Cookie` value of [`csrf_cookie`] with a minted token.
const CSRF_COOKIE_CAPACITY: usize = 80;

/// The `Set-Cookie` value of a minted token:
/// `rullst_csrf=<token>; Path=/; SameSite=<same_site>`, then `; Secure` when
/// `secure`.
fn csrf_cookie(token: &str, same_site: &str, secure: bool) -> String {
    let mut cookie = String::with_capacity(CSRF_COOKIE_CAPACITY);
    cookie.push_str("rullst_csrf=");
    cookie.push_str(token);
    cookie.push_str("; Path=/; SameSite=");
    cookie.push_str(same_site);
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

/// A response of the CSRF check itself, which the access log names
/// ([`crate::server::Refusal::CSRF`]).
fn refusal(status: StatusCode, message: &'static str) -> Response {
    crate::server::refused(
        (status, message).into_response(),
        crate::server::Refusal::CSRF,
    )
}

/// Returns the single valid `rullst_csrf` cookie across every `Cookie` header.
///
/// Pairs are split on the raw header bytes: browsers send non-ASCII
/// `document.cookie` values as UTF-8, and such an unrelated cookie must not
/// hide the CSRF cookie. A duplicate or malformed CSRF cookie still fails.
fn csrf_token_from_cookies(headers: &header::HeaderMap) -> Option<String> {
    let mut found = None;
    for value in headers.get_all(header::COOKIE) {
        for cookie in value.as_bytes().split(|byte| *byte == b';') {
            let Some(token) = cookie.trim_ascii().strip_prefix(b"rullst_csrf=") else {
                continue;
            };
            if found.is_some() || !valid_cookie_token(token) {
                return None;
            }
            let Ok(token) = std::str::from_utf8(token) else {
                return None;
            };
            found = Some(token.to_owned());
        }
    }
    found
}

#[cfg_attr(mutants, mutants::skip)]
async fn handle_csrf_state_modifying(mut req: Request, next: impl super::Run) -> Response {
    let csrf_cookie = csrf_token_from_cookies(req.headers());

    let Some(cookie_token) = csrf_cookie else {
        return refusal(StatusCode::FORBIDDEN, "CSRF token cookie missing");
    };

    // Check header first (common for AJAX/HTMX)
    let mut header_values = req.headers().get_all("X-CSRF-Token").iter();
    let header_token = header_values.next();
    if header_values.next().is_some() {
        return refusal(StatusCode::FORBIDDEN, "Ambiguous CSRF token");
    }

    if let Some(token) = header_token {
        if csrf_proof_matches(token.as_bytes(), &cookie_token).into() {
            let Some(csrf) = CsrfToken::for_response(&cookie_token) else {
                return refusal(StatusCode::INTERNAL_SERVER_ERROR, MASK_UNAVAILABLE);
            };
            req.extensions_mut().insert(csrf);
            return next.run(req).await;
        }
        return refusal(StatusCode::FORBIDDEN, "Invalid CSRF token");
    }

    // If not in header, check if it's a form-urlencoded request before buffering the body
    let content_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.split(';').next().is_some_and(|media_type| {
        media_type
            .trim()
            .eq_ignore_ascii_case("application/x-www-form-urlencoded")
    }) {
        let (mut parts, body) = req.into_parts();

        // Read request body (limited to 1MB to prevent memory exhaustion)
        let bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
            Ok(b) => b,
            Err(_) => {
                return refusal(StatusCode::BAD_REQUEST, "Failed to read request body");
            }
        };

        let body_token = extract_token_from_body(&bytes);

        if let Some(token) = body_token {
            if csrf_proof_matches(token.as_bytes(), &cookie_token).into() {
                let Some(csrf) = CsrfToken::for_response(&cookie_token) else {
                    return refusal(StatusCode::INTERNAL_SERVER_ERROR, MASK_UNAVAILABLE);
                };
                parts.extensions.insert(csrf);
                // Reconstruct the request so it can be parsed by subsequent handlers.
                let reconstructed_req = Request::from_parts(parts, axum::body::Body::from(bytes));
                return next.run(reconstructed_req).await;
            }
        }
    } else if let Some(boundary) = super::csrf_multipart::form_data_boundary(content_type) {
        let boundary = boundary.to_owned();
        let (mut parts, body) = req.into_parts();
        let Ok((body_token, body)) = super::csrf_multipart::read_token(body, &boundary).await
        else {
            return refusal(StatusCode::BAD_REQUEST, "Failed to read request body");
        };
        let Some(token) = body_token else {
            // Name the ordering rule: the token may follow the file inputs.
            return refusal(
                StatusCode::FORBIDDEN,
                super::csrf_multipart::MULTIPART_TOKEN_ORDER_MESSAGE,
            );
        };
        if csrf_proof_matches(&token, &cookie_token).into() {
            let Some(csrf) = CsrfToken::for_response(&cookie_token) else {
                return refusal(StatusCode::INTERNAL_SERVER_ERROR, MASK_UNAVAILABLE);
            };
            parts.extensions.insert(csrf);
            return next.run(Request::from_parts(parts, body)).await;
        }
    }

    refusal(StatusCode::FORBIDDEN, "Invalid or missing CSRF token")
}

#[cfg(test)]
#[path = "csrf_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "csrf_static_path_tests.rs"]
mod static_path_tests;
