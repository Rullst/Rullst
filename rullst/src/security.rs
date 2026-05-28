use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::distr::{Alphanumeric, SampleString};

/// Generates a cryptographically secure 32-character random alphanumeric string.
pub fn generate_csrf_token() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 32)
}

#[derive(serde::Deserialize)]
struct CsrfForm {
    _token: Option<String>,
}

/// Helper to extract the token from form-encoded body bytes.
fn extract_token_from_body(bytes: &[u8]) -> Option<String> {
    serde_urlencoded::from_bytes::<CsrfForm>(bytes)
        .ok()
        .and_then(|form| form._token)
}

/// Middleware that enforces CSRF protection using the Double Submit Cookie pattern.
/// GET requests generate a CSRF cookie if missing. Non-GET requests (POST, PUT, DELETE, PATCH)
/// must match the `rullst_csrf` cookie token with either the `X-CSRF-Token` header or form `_token` field.
pub async fn csrf_middleware(req: Request, next: Next) -> Response {
    let method = req.method();

    // 1. GET requests: bypass validation but ensure CSRF token cookie is set
    if method == axum::http::Method::GET {
        let has_cookie = req
            .headers()
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .map(|cookie_str| cookie_str.contains("rullst_csrf="))
            .unwrap_or(false);

        if !has_cookie {
            let token = generate_csrf_token();
            let mut response = next.run(req).await;

            // Set cookie for Strict mode
            if let Ok(cookie_val) = header::HeaderValue::from_str(&format!(
                "rullst_csrf={}; Path=/; SameSite=Strict; HttpOnly",
                token
            )) {
                response
                    .headers_mut()
                    .append(header::SET_COOKIE, cookie_val);
            }
            return response;
        }

        return next.run(req).await;
    }

    // 2. State-modifying requests: validate the token
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
        if token == cookie_token {
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
            Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response(),
        };

        let body_token = extract_token_from_body(&bytes);

        // Reconstruct the request so it can be parsed by subsequent handlers
        let reconstructed_req = Request::from_parts(parts, axum::body::Body::from(bytes));

        if let Some(token) = body_token
            && token == cookie_token
        {
            return next.run(reconstructed_req).await;
        }
    }

    (StatusCode::FORBIDDEN, "Invalid or missing CSRF token").into_response()
}

/// Middleware that injects secure-by-default HTTP headers to prevent standard web exploits.
pub async fn headers_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert("X-Frame-Options", header::HeaderValue::from_static("DENY"));
    headers.insert(
        "X-Content-Type-Options",
        header::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "X-XSS-Protection",
        header::HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        "Referrer-Policy",
        header::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "Strict-Transport-Security",
        header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    response
}
