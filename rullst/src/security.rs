use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::distr::{Alphanumeric, SampleString};
use subtle::ConstantTimeEq;

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

    if method == axum::http::Method::GET {
        handle_csrf_get(req, next).await
    } else {
        handle_csrf_state_modifying(req, next).await
    }
}

async fn handle_csrf_get(req: Request, next: Next) -> Response {
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
            "rullst_csrf={}; Path=/; SameSite={}; HttpOnly{}",
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

/// Middleware that injects secure-by-default HTTP headers to prevent standard web exploits.
pub async fn headers_middleware(req: Request, next: Next) -> Response {
    let csp = req
        .extensions()
        .get::<crate::config::SecurityConfig>()
        .map(|cfg| cfg.csp.clone())
        .unwrap_or_else(|| crate::config::RullstConfig::global().security.csp.clone());

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
        header::HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );
    headers.insert(
        "Permissions-Policy",
        header::HeaderValue::from_static("geolocation=(), camera=(), microphone=()"),
    );

    if !csp.is_empty() {
        if let Ok(csp_val) = header::HeaderValue::from_str(&csp) {
            headers.insert("Content-Security-Policy", csp_val);
        }
    }

    response
}

/// Helper to decode a hex char pair to a single byte.
fn hex_decode_char(c1: u8, c2: u8) -> Option<u8> {
    let b1 = (c1 as char).to_digit(16)?;
    let b2 = (c2 as char).to_digit(16)?;
    Some(((b1 << 4) | b2) as u8)
}

/// WebAssembly-compatible URL decoding helper.
fn url_decode(s: &str) -> String {
    let mut decoded_bytes = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'+' {
            decoded_bytes.push(b' ');
            i += 1;
            continue;
        }
        if b == b'%' && i + 2 < bytes.len() {
            let h1 = bytes[i + 1];
            let h2 = bytes[i + 2];
            if let Some(d) = hex_decode_char(h1, h2) {
                decoded_bytes.push(d);
                i += 3;
                continue;
            }
        }
        decoded_bytes.push(b);
        i += 1;
    }
    String::from_utf8_lossy(&decoded_bytes).into_owned()
}

/// WebAssembly-compatible WAF middleware for traffic control and malicious bot protection.
pub async fn waf_middleware(req: Request, next: Next) -> Response {
    // 1. Inspect User-Agent for known bots or scrapers
    if let Some(ua) = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
    {
        let ua_lower = ua.to_lowercase();
        let suspicious_agents = req
            .extensions()
            .get::<crate::config::SecurityConfig>()
            .map(|cfg| cfg.user_agent_blocklist.clone())
            .unwrap_or_else(|| {
                crate::config::RullstConfig::global()
                    .security
                    .user_agent_blocklist
                    .clone()
            });

        for agent in suspicious_agents {
            if ua_lower.contains(&agent.to_lowercase()) {
                match Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                    .body(axum::body::Body::from(
                        "Access Denied: Suspicious User-Agent blocked by Rullst Shield WAF.",
                    )) {
                    Ok(res) => return res,
                    Err(_) => return StatusCode::FORBIDDEN.into_response(),
                }
            }
        }
    }

    // 2. Inspect query parameters and headers for common attack vectors (SQLi, XSS, Path Traversal, CMD Injection)
    let malicious_patterns = [
        "select ",
        "union ",
        "insert ",
        "delete ",
        "drop table",
        "alter table", // SQLi
        "<script",
        "javascript:",
        "onload=",
        "onerror=",
        "document.cookie", // XSS
        "../",
        "..\\",
        "/etc/passwd",
        "win.ini", // Path Traversal
        "; ls",
        "&& cat",
        "| bash",
        "| sh",
        "wget ",
        "curl ",
        "ping -c", // Command Injection
    ];

    let mut payloads_to_check = Vec::new();

    if let Some(query) = req.uri().query() {
        payloads_to_check.push(query.to_string());
    }

    if let Some(referer) = req
        .headers()
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
    {
        payloads_to_check.push(referer.to_string());
    }

    if let Some(cookie) = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
    {
        payloads_to_check.push(cookie.to_string());
    }

    for payload in payloads_to_check {
        let payload_decoded = url_decode(&payload);
        let payload_lower = payload_decoded.to_lowercase();

        for pattern in malicious_patterns {
            if payload_lower.contains(pattern) {
                match Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                    .body(axum::body::Body::from(
                        "Access Denied: Malicious pattern detected by Rullst Shield WAF.",
                    )) {
                    Ok(res) => return res,
                    Err(_) => return StatusCode::FORBIDDEN.into_response(),
                }
            }
        }
    }

    next.run(req).await
}

/// Automatic PII (Personally Identifiable Information) masking middleware for response payloads.
pub async fn pii_masking_middleware(req: Request, next: Next) -> Response {
    let response = next.run(req).await;

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("text")
        || content_type.contains("json")
        || content_type.contains("javascript")
    {
        let (parts, body) = response.into_parts();
        if let Ok(bytes) = axum::body::to_bytes(body, 2 * 1024 * 1024).await {
            let body_str = String::from_utf8_lossy(&bytes);
            let masked_body = mask_pii(&body_str);

            let mut parts = parts;
            if parts.headers.contains_key(header::CONTENT_LENGTH) {
                if let Ok(val) = axum::http::HeaderValue::from_str(&masked_body.len().to_string()) {
                    parts.headers.insert(header::CONTENT_LENGTH, val);
                }
            }

            let new_body = axum::body::Body::from(masked_body);
            return Response::from_parts(parts, new_body);
        } else {
            match Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::empty())
            {
                Ok(res) => return res,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }

    response
}

/// Helper function to perform lightweight regex-free PII masking for emails and credit card numbers.
pub fn mask_pii(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut digit_indices = vec![i];
            let mut j = i + 1;
            let mut non_digits = 0;
            while j < chars.len() && non_digits < 3 {
                let c = chars[j];
                if c.is_ascii_digit() {
                    digit_indices.push(j);
                    non_digits = 0;
                } else if c == ' ' || c == '-' {
                    non_digits += 1;
                } else {
                    break;
                }
                j += 1;
            }

            let count = digit_indices.len();
            if (13..=19).contains(&count) {
                let mask_count = count - 4;
                for idx in 0..mask_count {
                    chars[digit_indices[idx]] = '*';
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }

    let mut idx = 0;
    while idx < chars.len() {
        if chars[idx] == '@' {
            let mut start = idx;
            while start > 0 {
                let c = chars[start - 1];
                if c.is_alphanumeric() || c == '.' || c == '_' || c == '%' || c == '+' || c == '-' {
                    start -= 1;
                } else {
                    break;
                }
            }

            let mut end = idx + 1;
            let mut dot_seen = false;
            while end < chars.len() {
                let c = chars[end];
                if c.is_alphanumeric() || c == '-' {
                    end += 1;
                } else if c == '.' {
                    dot_seen = true;
                    end += 1;
                } else {
                    break;
                }
            }

            let username_len = idx - start;
            let domain_len = end - (idx + 1);
            if username_len > 1 && domain_len > 3 && dot_seen {
                for item in chars.iter_mut().take(idx).skip(start + 1) {
                    *item = '*';
                }
                idx = end;
                continue;
            }
        }
        idx += 1;
    }

    chars.into_iter().collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_pii_credit_card() {
        let raw = "My card number is 1234-5678-1234-5678 and it is secret.";
        let masked = mask_pii(raw);
        assert!(masked.contains("****-****-****-5678"));
        assert!(!masked.contains("1234-5678-1234"));
    }

    #[test]
    fn test_mask_pii_edge_cases() {
        assert_eq!(mask_pii(""), "");
        assert_eq!(mask_pii("a@b.c"), "a@b.c");
        assert_eq!(
            mask_pii("admin123@longdomain.com"),
            "a*******@longdomain.com"
        );
        assert_eq!(mask_pii("invalid_email@"), "invalid_email@");
        assert_eq!(mask_pii("my card is 1234"), "my card is 1234");
    }

    #[test]
    fn test_mask_pii_email() {
        let raw = "Contact me at venelouis@rullst.com or admin@domain.org.";
        let masked = mask_pii(raw);
        assert!(masked.contains("v********@rullst.com"));
        assert!(masked.contains("a****@domain.org"));
    }

    #[tokio::test]
    async fn test_waf_middleware_blocks_malicious_query() {
        use axum::http::{Request, StatusCode};

        // Not currently possible to test axum middlewares easily without setting up an app.
        // We will test `waf_middleware` via a router approach.
        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "OK" }))
            .route_layer(axum::middleware::from_fn(waf_middleware));

        // Use reqwest or tower::ServiceExt to call the app
        let req = Request::builder()
            .uri("/?q=select%20")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        let req2 = Request::builder()
            .uri("/?q=hello")
            .body(axum::body::Body::empty())
            .unwrap();
        let res2 = tower::ServiceExt::oneshot(app, req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_headers_middleware_injects_security_headers() {
        use axum::http::{Request, StatusCode};

        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "OK" }))
            .route_layer(axum::middleware::from_fn(headers_middleware));

        let req = Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = tower::ServiceExt::oneshot(app, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let headers = res.headers();
        assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
        assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
        assert_eq!(headers.get("X-XSS-Protection").unwrap(), "1; mode=block");
        assert_eq!(
            headers.get("Strict-Transport-Security").unwrap(),
            "max-age=31536000; includeSubDomains; preload"
        );
        assert_eq!(
            headers.get("Permissions-Policy").unwrap(),
            "geolocation=(), camera=(), microphone=()"
        );
    }

    #[test]
    fn test_generate_csrf_token() {
        let token1 = generate_csrf_token();
        let token2 = generate_csrf_token();
        assert_eq!(token1.len(), 32);
        assert_eq!(token2.len(), 32);
        assert_ne!(token1, token2);
    }
}
