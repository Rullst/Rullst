//! CSRF protection middleware and Double Submit Cookie validation.

use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::distr::{Alphanumeric, SampleString};
use subtle::ConstantTimeEq;

#[cfg_attr(mutants, mutants::skip)]
fn is_production() -> bool {
    let env = std::env::var("RULLST_ENV")
        .unwrap_or_else(|_| std::env::var("APP_ENV").unwrap_or_default());
    env.eq_ignore_ascii_case("production") || env.eq_ignore_ascii_case("prod")
}

/// Generates a cryptographically secure 32-character random alphanumeric string.
pub fn generate_csrf_token() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 32)
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
/// GET requests generate a CSRF cookie if missing. Non-GET requests (POST, PUT, DELETE, PATCH)
/// must match the `rullst_csrf` cookie token with either the `X-CSRF-Token` header or form `_token` field.
pub async fn csrf_middleware(req: Request, next: Next) -> Response {
    let method = req.method();

    if method == axum::http::Method::GET {
        handle_csrf_get(req, next).await
    } else {
        handle_csrf_state_modifying(req, next).await
    }
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

async fn handle_csrf_get(req: Request, next: Next) -> Response {
    if is_csrf_exempt_path(req.uri().path()) {
        return next.run(req).await;
    }

    let has_cookie = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|cookie_str| cookie_str.contains("rullst_csrf="))
        .unwrap_or(false);

    if !has_cookie {
        let token = generate_csrf_token();
        let same_site = req
            .extensions()
            .get::<crate::config::SecurityConfig>()
            .map(|cfg| cfg.csrf_same_site.clone())
            .unwrap_or_else(|| "Lax".to_string());

        let mut response = next.run(req).await;

        let secure_attr = if is_production() { "; Secure" } else { "" };
        if let Ok(cookie_val) = header::HeaderValue::from_str(&format!(
            "rullst_csrf={}; Path=/; SameSite={}{}",
            token, same_site, secure_attr
        )) {
            response
                .headers_mut()
                .append(header::SET_COOKIE, cookie_val);
        }
        return response;
    }

    next.run(req).await
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
