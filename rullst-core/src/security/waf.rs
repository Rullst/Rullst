//! WebAssembly-compatible WAF (Web Application Firewall) middleware.
//!
//! A coarse baseline: it refuses common injection *structures* (see the
//! `detect` submodule) in the query, `Referer`, cookies and bounded textual
//! bodies, and traversal in the path. Parameterized SQL, output encoding and
//! typed validation remain the actual defenses. The `report` submodule runs
//! the same rules in development, where it only logs what it would refuse.

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};

mod detect;
mod report;

pub(super) use report::waf_report_middleware;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod exemption_tests;
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod limits_tests;
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;

const MAX_INSPECTED_REQUEST_BYTES: usize = 1024 * 1024;

/// Path-traversal signatures, checked in the query, headers, bodies and the
/// percent-decoded request path.
static PATH_TRAVERSAL_PATTERNS: &[&str] = &["../", "..\\", "/etc/passwd", "win.ini"];

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

/// A WAF rule a request breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rule {
    /// The `User-Agent` contains a `user_agent_blocklist` entry.
    UserAgent,
    /// The percent-decoded path contains a traversal signature.
    PathTraversal,
    /// The query string carries a signature.
    Query,
    /// The `Referer` carries a signature.
    Referer,
    /// A cookie pair carries a signature.
    Cookie,
    /// The textual body carries a signature.
    Body,
    /// The body is compressed or otherwise encoded.
    BodyEncoding,
    /// The declared `Content-Length` exceeds the inspection limit.
    BodyTooLarge,
    /// The body exceeds the inspection limit or could not be read.
    BodyUnreadable,
    /// The textual body is not UTF-8.
    BodyNotUtf8,
}

impl Rule {
    /// The rule's name in development logs.
    fn name(self) -> &'static str {
        match self {
            Self::UserAgent => "user-agent-blocklist",
            Self::PathTraversal => "path-traversal",
            Self::Query => "query-signature",
            Self::Referer => "referer-signature",
            Self::Cookie => "cookie-signature",
            Self::Body => "body-signature",
            Self::BodyEncoding => "body-encoding",
            Self::BodyTooLarge | Self::BodyUnreadable => "body-too-large",
            Self::BodyNotUtf8 => "body-not-utf8",
        }
    }

    /// The response staging and production send for this rule.
    fn refusal(self) -> Response {
        let (status, message) = match self {
            Self::UserAgent => (
                StatusCode::FORBIDDEN,
                "Access Denied: Suspicious User-Agent blocked by Rullst Shield WAF.",
            ),
            Self::PathTraversal | Self::Query | Self::Referer | Self::Cookie | Self::Body => (
                StatusCode::FORBIDDEN,
                "Access Denied: Malicious pattern detected by Rullst Shield WAF.",
            ),
            Self::BodyEncoding => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Encoded request bodies cannot be inspected by the WAF.",
            ),
            Self::BodyTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "Request body exceeds the WAF inspection limit.",
            ),
            Self::BodyUnreadable => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "Request body could not be inspected within the WAF limit.",
            ),
            Self::BodyNotUtf8 => (
                StatusCode::BAD_REQUEST,
                "Declared textual request body is not valid UTF-8.",
            ),
        };
        plain_response(status, message)
    }
}

/// The application's security configuration, else the process-global one.
fn security_config(req: &Request) -> &crate::config::SecurityConfig {
    req.extensions()
        .get::<crate::config::SecurityConfig>()
        .unwrap_or(&crate::config::RullstConfig::global().security)
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
    PATH_TRAVERSAL_PATTERNS
        .iter()
        .any(|pattern| payload_lower.contains(pattern))
        || detect::is_injection(&payload_lower)
}

/// Inspects every key and string of a JSON document on its own, so JSON
/// syntax (`"`, `:`) never joins a signature. Text that is not valid JSON is
/// inspected as a whole.
fn json_contains_malicious_pattern(payload: &str) -> bool {
    let Ok(document) = serde_json::from_str::<serde_json::Value>(payload) else {
        return contains_malicious_pattern(payload);
    };
    let mut pending = vec![&document];
    while let Some(value) = pending.pop() {
        match value {
            serde_json::Value::String(text) if contains_malicious_pattern(text) => return true,
            serde_json::Value::Array(items) => pending.extend(items),
            serde_json::Value::Object(fields) => {
                for (key, field) in fields {
                    if contains_malicious_pattern(key) {
                        return true;
                    }
                    pending.push(field);
                }
            }
            _ => {}
        }
    }
    false
}

fn body_is_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(super::media_type::essence)
        .is_some_and(super::media_type::is_json)
}

/// Whether the percent-decoded request path carries a traversal signature.
///
/// Routers decode path parameters, so `/files/..%2f..%2fetc%2fpasswd` reaches
/// a `{name}` handler as `../../etc/passwd`. Only the traversal group applies
/// to the path: SQL and command words are ordinary in slugs and file names.
fn path_contains_traversal(path: &str) -> bool {
    let decoded = url_decode(path).to_lowercase();
    PATH_TRAVERSAL_PATTERNS
        .iter()
        .any(|pattern| decoded.contains(pattern))
}

/// Whether the WAF reads this request's body: a declared textual media type
/// on a path that `waf_body_exempt_paths` does not list.
fn body_is_inspected(req: &Request) -> bool {
    should_inspect_body(req.headers())
        && !security_config(req)
            .waf_body_exempt_paths
            .iter()
            .any(|path| path == req.uri().path())
}

/// The rule an inspected body breaks before it is read: an encoding or a
/// declared length the WAF cannot inspect.
fn screen_body_headers(headers: &HeaderMap) -> Option<Rule> {
    if !has_identity_encoding(headers) {
        return Some(Rule::BodyEncoding);
    }
    declared_body_is_too_large(headers).then_some(Rule::BodyTooLarge)
}

/// The rule a complete inspected body breaks.
fn screen_body_bytes(headers: &HeaderMap, bytes: &[u8]) -> Option<Rule> {
    let Ok(payload) = std::str::from_utf8(bytes) else {
        return Some(Rule::BodyNotUtf8);
    };
    let malicious = if body_is_json(headers) {
        json_contains_malicious_pattern(payload)
    } else {
        contains_malicious_pattern(payload)
    };
    malicious.then_some(Rule::Body)
}

/// The first rule that the request line or headers break: the `User-Agent`
/// blocklist, traversal in the path, then signatures in the query, the
/// `Referer` and each cookie pair.
///
/// Header values are decoded lossily: an obs-text byte (0x80-0xFF) that
/// `to_str` rejects must not hide the rest of the value from inspection.
fn screen_head(req: &Request) -> Option<Rule> {
    if let Some(ua) = req
        .headers()
        .get(header::USER_AGENT)
        .map(|value| String::from_utf8_lossy(value.as_bytes()))
    {
        let ua_lower = ua.to_lowercase();
        if security_config(req)
            .user_agent_blocklist
            .iter()
            .any(|agent| ua_lower.contains(&agent.to_lowercase()))
        {
            return Some(Rule::UserAgent);
        }
    }

    if path_contains_traversal(req.uri().path()) {
        return Some(Rule::PathTraversal);
    }
    if req.uri().query().is_some_and(contains_malicious_pattern) {
        return Some(Rule::Query);
    }
    if req
        .headers()
        .get(header::REFERER)
        .is_some_and(|value| contains_malicious_pattern(&String::from_utf8_lossy(value.as_bytes())))
    {
        return Some(Rule::Referer);
    }

    // Each cookie pair is inspected on its own: the `; ` pair separator is
    // header syntax, so a later cookie named like a command (`ls`, `id`)
    // must not read as a chained shell command.
    req.headers()
        .get_all(header::COOKIE)
        .iter()
        .any(|cookies| {
            String::from_utf8_lossy(cookies.as_bytes())
                .split(';')
                .any(|pair| contains_malicious_pattern(pair.trim()))
        })
        .then_some(Rule::Cookie)
}

/// Inspects a bounded textual/JSON/form body, then rebuilds the exact
/// request for downstream extractors. Unsupported encodings and over-limit
/// text fail closed.
async fn inspect_and_restore_body(req: Request) -> Result<Request, Rule> {
    if !body_is_inspected(&req) {
        return Ok(req);
    }
    if let Some(rule) = screen_body_headers(req.headers()) {
        return Err(rule);
    }

    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, MAX_INSPECTED_REQUEST_BYTES)
        .await
        .map_err(|_| Rule::BodyUnreadable)?;
    match screen_body_bytes(&parts.headers, &bytes) {
        Some(rule) => Err(rule),
        None => Ok(Request::from_parts(parts, Body::from(bytes))),
    }
}

/// WebAssembly-compatible WAF middleware for traffic control and malicious bot protection.
///
/// It refuses a request whose `User-Agent` matches `user_agent_blocklist`,
/// whose path carries traversal, or whose query, `Referer`, cookies or
/// bounded textual body (`text/*`, JSON, XML, URL-encoded forms, at most
/// 1 MiB) carry an injection signature. Bodies on the exact paths listed in
/// `waf_body_exempt_paths` are neither read nor inspected; everything else
/// about those requests is. `Server` mounts it in staging and production; in
/// development it mounts a report-only variant that logs, at `warn` level,
/// the rule and path of each request this middleware would refuse.
pub async fn waf_middleware(req: Request, next: Next) -> Response {
    if let Some(rule) = screen_head(&req) {
        return rule.refusal();
    }
    match inspect_and_restore_body(req).await {
        Ok(req) => next.run(req).await,
        Err(rule) => rule.refusal(),
    }
}
