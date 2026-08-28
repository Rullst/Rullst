//! CSRF protection middleware and Double Submit Cookie validation.

use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::distr::{Alphanumeric, SampleString};
use subtle::ConstantTimeEq;

/// Generates a cryptographically secure 32-character random alphanumeric string.
pub fn generate_csrf_token() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 32)
}

/// Request-scoped CSRF token made available to handlers rendering forms.
///
/// On the first safe request this is the same token that the middleware writes
/// to the response cookie. Exposing it through request extensions avoids
/// rendering an empty hidden field before the browser has received that cookie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsrfToken(String);

impl CsrfToken {
    /// Returns the token that must be echoed in an `X-CSRF-Token` header or
    /// `_token` form field.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
/// GET requests generate a CSRF cookie if missing. HTTP safe methods pass through, while
/// state-changing requests must match the `rullst_csrf` cookie token with either the
/// `X-CSRF-Token` header or form `_token` field.
pub async fn csrf_middleware(req: Request, next: Next) -> Response {
    if is_signed_webhook_exemption(&req) {
        return next.run(req).await;
    }

    let method = req.method();

    if method == axum::http::Method::GET {
        handle_csrf_get(req, next).await
    } else if method == axum::http::Method::HEAD
        || method == axum::http::Method::OPTIONS
        || method == axum::http::Method::TRACE
    {
        next.run(req).await
    } else {
        handle_csrf_state_modifying(req, next).await
    }
}

fn is_signed_webhook_exemption(req: &Request) -> bool {
    if req.method() != axum::http::Method::POST {
        return false;
    }

    let config = req
        .extensions()
        .get::<crate::config::SecurityConfig>()
        .unwrap_or(&crate::config::RullstConfig::global().security);
    config
        .csrf_signed_webhook_paths
        .iter()
        .any(|path| path == req.uri().path())
}

pub(crate) fn is_csrf_exempt_path(path: &str) -> bool {
    path == "/robots.txt"
        || path == "/sitemap.xml"
        || path == "/favicon.ico"
        || path.starts_with("/static/")
        || path.ends_with(".txt")
        || path.ends_with(".xml")
        || path.ends_with(".ico")
        || path.ends_with(".json")
        || path.ends_with(".css")
        || path.ends_with(".js")
        || path.ends_with(".png")
        || path.ends_with(".jpg")
        || path.ends_with(".jpeg")
        || path.ends_with(".svg")
        || path.ends_with(".webp")
        || path.ends_with(".wasm")
}

async fn handle_csrf_get(mut req: Request, next: Next) -> Response {
    if is_csrf_exempt_path(req.uri().path()) {
        return next.run(req).await;
    }

    let cookie_token = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(csrf_token_from_cookie_header);

    if let Some(token) = cookie_token {
        req.extensions_mut().insert(CsrfToken(token));
        return next.run(req).await;
    }

    let token = generate_csrf_token();
    req.extensions_mut().insert(CsrfToken(token.clone()));
    {
        let same_site = req
            .extensions()
            .get::<crate::config::SecurityConfig>()
            .map(|cfg| cfg.csrf_same_site.clone())
            .unwrap_or_else(|| "Lax".to_string());
        let secure_cookie = req
            .extensions()
            .get::<crate::config::Environment>()
            .copied()
            .map(crate::config::Environment::requires_secure_defaults)
            .unwrap_or_else(|| {
                crate::config::Environment::detect(None)
                    .map(crate::config::Environment::requires_secure_defaults)
                    .unwrap_or(true)
            });

        let mut response = next.run(req).await;

        let secure_attr = if secure_cookie { "; Secure" } else { "" };
        if let Ok(cookie_val) = header::HeaderValue::from_str(&format!(
            "rullst_csrf={}; Path=/; SameSite={}{}",
            token, same_site, secure_attr
        )) {
            response
                .headers_mut()
                .append(header::SET_COOKIE, cookie_val);
        }
        response
    }
}

fn csrf_token_from_cookie_header(cookie_header: &str) -> Option<String> {
    cookie_header.split(';').find_map(|cookie| {
        cookie
            .trim()
            .strip_prefix("rullst_csrf=")
            .filter(|token| !token.is_empty())
            .map(ToOwned::to_owned)
    })
}

#[cfg_attr(mutants, mutants::skip)]
async fn handle_csrf_state_modifying(req: Request, next: Next) -> Response {
    let csrf_cookie = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            for cookie in cookie_str.split(';') {
                let trimmed = cookie.trim();
                if let Some(stripped) = trimmed.strip_prefix("rullst_csrf=") {
                    return Some(stripped.to_string());
                }
            }
            None
        });

    let Some(cookie_token) = csrf_cookie else {
        return (StatusCode::FORBIDDEN, "CSRF token cookie missing").into_response();
    };

    // Check header first (common for AJAX/HTMX)
    let header_token = req
        .headers()
        .get("X-CSRF-Token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let Some(token) = header_token {
        if token.len() == cookie_token.len()
            && token.as_bytes().ct_eq(cookie_token.as_bytes()).into()
        {
            return next.run(req).await;
        }
        return (StatusCode::FORBIDDEN, "Invalid CSRF token").into_response();
    }

    // If not in header, check if it's a form-urlencoded request before buffering the body
    let content_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("application/x-www-form-urlencoded") {
        let (parts, body) = req.into_parts();

        // Read request body (limited to 1MB to prevent memory exhaustion)
        let bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
            Ok(b) => b,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
            }
        };

        let body_token = extract_token_from_body(&bytes);

        // Reconstruct the request so it can be parsed by subsequent handlers
        let reconstructed_req = Request::from_parts(parts, axum::body::Body::from(bytes));

        if let Some(token) = body_token {
            if token.len() == cookie_token.len()
                && token.as_bytes().ct_eq(cookie_token.as_bytes()).into()
            {
                return next.run(reconstructed_req).await;
            }
        }
    }

    (StatusCode::FORBIDDEN, "Invalid or missing CSRF token").into_response()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request, routing::any};
    use tower::ServiceExt;

    #[tokio::test]
    async fn safe_http_methods_do_not_require_a_token() {
        let app = Router::new()
            .route("/", any(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(csrf_middleware));

        for method in [
            axum::http::Method::HEAD,
            axum::http::Method::OPTIONS,
            axum::http::Method::TRACE,
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri("/")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_ne!(response.status(), StatusCode::FORBIDDEN);
        }
    }

    #[tokio::test]
    async fn production_like_environment_sets_secure_cookie() {
        let app = Router::new()
            .route("/", any(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(csrf_middleware))
            .layer(axum::Extension(crate::config::Environment::Staging));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cookie.contains("; Secure"));
    }

    #[tokio::test]
    async fn only_exact_configured_post_webhook_path_is_exempt() {
        let mut security = crate::config::SecurityConfig::default();
        security.csrf_signed_webhook_paths = vec!["/billing/webhook".to_owned()];
        let app = Router::new()
            .route("/billing/webhook", any(|| async { StatusCode::OK }))
            .route("/billing/webhook/extra", any(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(csrf_middleware))
            .layer(axum::Extension(security));

        let exempt = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/billing/webhook")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(exempt.status(), StatusCode::OK);

        let prefix = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/billing/webhook/extra")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(prefix.status(), StatusCode::FORBIDDEN);

        let non_post = app
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::PUT)
                    .uri("/billing/webhook")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(non_post.status(), StatusCode::FORBIDDEN);
    }
}
