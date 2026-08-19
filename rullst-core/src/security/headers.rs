//! OWASP Secure Headers injection middleware.

use axum::{extract::Request, http::header, middleware::Next, response::Response};

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
    headers.insert(
        "Cross-Origin-Opener-Policy",
        header::HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        "Cross-Origin-Resource-Policy",
        header::HeaderValue::from_static("same-site"),
    );
    headers.insert(
        "Cross-Origin-Embedder-Policy",
        header::HeaderValue::from_static("unsafe-none"),
    );
    headers.insert(
        "Cache-Control",
        header::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );

    let final_csp = if csp.is_empty() {
        "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com https://unpkg.com blob:; worker-src 'self' blob:; style-src 'self' 'unsafe-inline' https://cdn.tailwindcss.com; img-src 'self' data:; connect-src 'self' ws: wss:; font-src 'self' data: https:; object-src 'none'".to_string()
    } else {
        csp
    };

    if let Ok(csp_val) = header::HeaderValue::from_str(&final_csp) {
        headers.insert("Content-Security-Policy", csp_val);
    }

    response
}
