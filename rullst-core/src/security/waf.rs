//! WebAssembly-compatible WAF (Web Application Firewall) middleware.

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};

const MAX_INSPECTED_REQUEST_BYTES: usize = 1024 * 1024;

static MALICIOUS_PATTERNS: &[&str] = &[
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

fn plain_response(status: StatusCode, message: &'static str) -> Response {
    let mut response = Response::new(Body::from(message));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn forbidden_response() -> Response {
    plain_response(
        StatusCode::FORBIDDEN,
        "Access Denied: Malicious pattern detected by Rullst Shield WAF.",
    )
}

fn body_media_type(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::CONTENT_TYPE)?
        .to_str()
        .ok()?
        .split(';')
        .next()
        .map(str::trim)
}

fn should_inspect_body(headers: &HeaderMap) -> bool {
    let Some(media_type) = body_media_type(headers) else {
        return false;
    };

    media_type
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("text/"))
        || media_type.eq_ignore_ascii_case("application/json")
        || media_type.eq_ignore_ascii_case("application/x-www-form-urlencoded")
        || media_type.eq_ignore_ascii_case("application/xml")
        || media_type
            .strip_suffix("+json")
            .is_some_and(|prefix| prefix.starts_with("application/"))
        || media_type
            .strip_suffix("+xml")
            .is_some_and(|prefix| prefix.starts_with("application/"))
}

fn has_identity_encoding(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_ENCODING)
        .is_none_or(|value| value.as_bytes().eq_ignore_ascii_case(b"identity"))
}

fn declared_body_is_too_large(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| length > MAX_INSPECTED_REQUEST_BYTES)
}

/// Helper to decode a hex char pair to a single byte.
#[cfg_attr(mutants, mutants::skip)]
fn hex_decode_char(c1: u8, c2: u8) -> Option<u8> {
    let b1 = (c1 as char).to_digit(16)?;
    let b2 = (c2 as char).to_digit(16)?;
    Some(((b1 << 4) | b2) as u8)
}

/// WebAssembly-compatible URL decoding helper.
#[cfg_attr(mutants, mutants::skip)]
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

fn contains_malicious_pattern(payload: &str) -> bool {
    let payload_decoded = url_decode(payload);
    let payload_lower = payload_decoded.to_lowercase();
    MALICIOUS_PATTERNS
        .iter()
        .any(|pattern| payload_lower.contains(pattern))
}

async fn inspect_and_restore_body(req: Request) -> Result<Request, Box<Response>> {
    if !should_inspect_body(req.headers()) {
        return Ok(req);
    }

    if !has_identity_encoding(req.headers()) {
        return Err(Box::new(plain_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Encoded request bodies cannot be inspected by the WAF.",
        )));
    }

    if declared_body_is_too_large(req.headers()) {
        return Err(Box::new(plain_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Request body exceeds the WAF inspection limit.",
        )));
    }

    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, MAX_INSPECTED_REQUEST_BYTES)
        .await
        .map_err(|_| {
            Box::new(plain_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Request body could not be inspected within the WAF limit.",
            ))
        })?;

    let payload = std::str::from_utf8(&bytes).map_err(|_| {
        Box::new(plain_response(
            StatusCode::BAD_REQUEST,
            "Declared textual request body is not valid UTF-8.",
        ))
    })?;
    if contains_malicious_pattern(payload) {
        return Err(Box::new(forbidden_response()));
    }

    Ok(Request::from_parts(parts, Body::from(bytes)))
}

/// WebAssembly-compatible WAF middleware for traffic control and malicious bot protection.
pub async fn waf_middleware(mut req: Request, next: Next) -> Response {
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
                return plain_response(
                    StatusCode::FORBIDDEN,
                    "Access Denied: Suspicious User-Agent blocked by Rullst Shield WAF.",
                );
            }
        }
    }

    // 2. Inspect query parameters and selected headers for common attack vectors.
    if let Some(query) = req.uri().query() {
        if contains_malicious_pattern(query) {
            return forbidden_response();
        }
    }

    for header_name in [header::REFERER, header::COOKIE] {
        if let Some(payload) = req
            .headers()
            .get(header_name)
            .and_then(|value| value.to_str().ok())
            && contains_malicious_pattern(payload)
        {
            return forbidden_response();
        }
    }

    // 3. Inspect bounded textual/JSON/form bodies, then reconstruct the exact request for
    // downstream extractors. Unsupported encodings and over-limit text fail closed.
    req = match inspect_and_restore_body(req).await {
        Ok(req) => req,
        Err(response) => return *response,
    };

    next.run(req).await
}
