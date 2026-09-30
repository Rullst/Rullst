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

fn should_inspect_body(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(super::media_type::essence)
        .is_some_and(super::media_type::is_inspected_request_body)
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
    // 1. Inspect User-Agent for known bots or scrapers. Header values are
    // decoded lossily: an obs-text byte (0x80-0xFF) that `to_str` rejects must
    // not hide the rest of the value from inspection.
    if let Some(ua) = req
        .headers()
        .get(header::USER_AGENT)
        .map(|value| String::from_utf8_lossy(value.as_bytes()))
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

    if let Some(referer) = req
        .headers()
        .get(header::REFERER)
        .map(|value| String::from_utf8_lossy(value.as_bytes()))
        && contains_malicious_pattern(&referer)
    {
        return forbidden_response();
    }

    // Each cookie pair is inspected on its own: the `; ` pair separator is
    // header syntax, so the `; ls` command pattern must not match a later
    // cookie whose name starts with `ls`.
    for cookies in req.headers().get_all(header::COOKIE) {
        if String::from_utf8_lossy(cookies.as_bytes())
            .split(';')
            .any(|pair| contains_malicious_pattern(pair.trim()))
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::{Router, body::Bytes, http::Request, routing::post};
    use tower::ServiceExt;

    #[tokio::test]
    async fn cookie_pair_separators_are_not_command_injection() {
        let app = Router::new()
            .route("/items", post(|body: Bytes| async move { body }))
            .route_layer(axum::middleware::from_fn(waf_middleware));
        for (case, (cookie, expected)) in [
            ("rullst_csrf=abc; lsid=1", StatusCode::OK),
            ("a=1;ls_session=2; LSKEY=3", StatusCode::OK),
            ("a=1; b=../../etc/passwd", StatusCode::FORBIDDEN),
            ("theme=dark; q=%3Cscript%3E", StatusCode::FORBIDDEN),
        ]
        .into_iter()
        .enumerate()
        {
            let request = Request::post("/items")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap();
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                expected,
                "case {case}"
            );
        }
        let split_header = Request::post("/items")
            .header(header::COOKIE, "a=1")
            .header(header::COOKIE, "b=<script>")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.oneshot(split_header).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn obs_text_bytes_cannot_hide_inspected_header_values() {
        let app = Router::new()
            .route("/items", post(|body: Bytes| async move { body }))
            .route_layer(axum::middleware::from_fn(waf_middleware));
        for (name, value) in [
            (header::REFERER, &b"https://x.example/?q=<script>\xff"[..]),
            (header::COOKIE, &b"q=../../etc/passwd\xff"[..]),
            (header::USER_AGENT, &b"GPTBot/1.0 \xff"[..]),
        ] {
            let request = Request::post("/items")
                .header(name.clone(), HeaderValue::from_bytes(value).unwrap())
                .body(Body::empty())
                .unwrap();
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                StatusCode::FORBIDDEN,
                "{name}"
            );
        }
    }

    #[tokio::test]
    async fn case_and_suffix_variants_cannot_skip_body_inspection() {
        let app = Router::new()
            .route("/items", post(|body: Bytes| async move { body }))
            .route_layer(axum::middleware::from_fn(waf_middleware));
        for (media_type, body) in [
            (
                "application/vnd.api+JSON",
                r#"{"bio":"<script>document.cookie</script>"}"#,
            ),
            (
                "Application/problem+json",
                r#"{"q":"x' union select pw from users--"}"#,
            ),
            ("application/json+patch", r#"{"path":"../../etc/passwd"}"#),
            (
                "application/x-www-form-urlencodedX",
                "q=x%27%20union%20select%20pw%20from%20users--",
            ),
        ] {
            let request = Request::post("/items")
                .header(header::CONTENT_TYPE, media_type)
                .body(Body::from(body))
                .unwrap();
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                StatusCode::FORBIDDEN,
                "{media_type} body was not inspected"
            );
        }
    }
}
