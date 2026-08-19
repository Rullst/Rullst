//! WebAssembly-compatible WAF (Web Application Firewall) middleware.

use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

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
