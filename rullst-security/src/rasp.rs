//! Signature-based request inspection ("RASP") for bounded HTTP input.
//!
//! **Risk reduced:** common SQL injection, path traversal, SSRF to cloud
//! metadata hosts, shell execution and JNDI lookup payloads reaching a
//! handler.
//!
//! **How:** case-insensitive substring signatures (a heuristic, not a parser)
//! applied to the request target, every header except `Cookie` and
//! `Authorization`, and identity-encoded UTF-8 bodies of at most 1 MiB with a
//! `text/*`, JSON, XML or URL-encoded form media type. Each value is checked
//! raw and after one percent-decoding pass; JSON keys and strings are also
//! checked after JSON decoding. Matches return `403`; bodies with a
//! non-identity `Content-Encoding` return `415`, oversized bodies `413` and
//! invalid UTF-8 `400`.
//!
//! **Known limits:** `multipart/form-data` and binary bodies, cookies and
//! credentials are not inspected. Double encoding, HTML entities, SQL
//! comments between keywords, alternative syntax and payloads split across
//! fields are not normalized, so they bypass the signatures. Ordinary text
//! containing a signature (`../`, `sleep(`, `/bin/sh`) is refused, so expect
//! false positives on free-text fields. Body inspection buffers the body.
//!
//! **Operator duties:** keep parameterized SQL, typed validation, URL
//! allowlists for outgoing requests and shell-free process APIs. Put a request
//! body limit outside this layer, test your own routes for false positives,
//! and note that `Server` already runs the overlapping Core WAF in staging and
//! production.

use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue, Request, Response, StatusCode, header},
    response::IntoResponse,
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

use crate::media_type;

mod user_agent;

const MAX_INSPECTED_REQUEST_BYTES: usize = 1024 * 1024;

/// Heuristic RASP (Runtime Application Self-Protection) inspector.
///
/// It is a defense-in-depth signal for bounded HTTP metadata and textual bodies. It does not
/// replace typed input validation, parameterized SQL, URL allowlists, or shell-free APIs.
static ATTACK_PATTERNS: &[&str] = &[
    // SQL Injection
    "union select",
    "union%20select",
    "union+select",
    "' or '1'='1",
    "%27%20or%20%271%27=%271",
    "; drop table",
    ";%20drop%20table",
    "sleep(",
    "benchmark(",
    "extractvalue(",
    "information_schema",
    // Path Traversal
    "../",
    "..\\",
    "%2e%2e/",
    "%2e%2e%2f",
    "/etc/passwd",
    "c:\\windows\\system32",
    // SSRF
    "169.254.169.254",
    "metadata.google.internal",
    "127.0.0.1:2375",
    // RCE & Shell Injection
    "; cat ",
    "| sh",
    "; rm -rf",
    user_agent::POWERSHELL,
    "cmd.exe",
    "/bin/bash",
    "/bin/sh",
    // Log4j / JNDI
    "${jndi:",
    "${ldap:",
    "${rmi:",
    "${dns:",
];

#[inline]
fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let h_bytes = haystack.as_bytes();
    let n_bytes = needle.as_bytes();
    if h_bytes.len() < n_bytes.len() {
        return false;
    }

    h_bytes.windows(n_bytes.len()).any(|window| {
        window
            .iter()
            .zip(n_bytes.iter())
            .all(|(&h, &n)| fold_ascii_byte(h) == fold_ascii_byte(n))
    })
}

const fn fold_ascii_byte(byte: u8) -> u8 {
    if byte >= b'A' && byte <= b'Z' {
        byte + (b'a' - b'A')
    } else {
        byte
    }
}

/// Whether `payload` contains a signature. In a `User-Agent` value, a
/// PowerShell product token does not count as the `powershell` keyword.
fn has_attack_pattern(payload: &str, is_user_agent: bool) -> bool {
    ATTACK_PATTERNS.iter().any(|pattern| {
        if is_user_agent && *pattern == user_agent::POWERSHELL {
            user_agent::has_powershell_outside_product_tokens(payload)
        } else {
            contains_ignore_ascii_case(payload, pattern)
        }
    })
}

/// Applies the signatures to the raw text and to one percent-decoded layer.
fn inspect_value(payload: &str, is_user_agent: bool) -> bool {
    if has_attack_pattern(payload, is_user_agent) {
        return true;
    }

    let decoded = decode_percent_once(payload);
    decoded != payload && has_attack_pattern(&decoded, is_user_agent)
}

fn decode_percent_once(payload: &str) -> String {
    let bytes = payload.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = (bytes[index + 1] as char).to_digit(16);
            let low = (bytes[index + 2] as char).to_digit(16);
            if let (Some(high), Some(low)) = (high, low) {
                decoded.push(((high << 4) | low) as u8);
                index += 3;
                continue;
            }
        } else if bytes[index] == b'+' {
            decoded.push(b' ');
            index += 1;
            continue;
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

fn inspect_json_value(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(value) => RaspInspector::inspect_text(value),
        serde_json::Value::Array(values) => values.iter().any(inspect_json_value),
        serde_json::Value::Object(values) => values
            .iter()
            .any(|(key, value)| RaspInspector::inspect_text(key) || inspect_json_value(value)),
        _ => false,
    }
}

fn request_body_media_type(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::CONTENT_TYPE)?
        .to_str()
        .ok()
        .map(media_type::essence)
}

fn should_inspect_body(headers: &HeaderMap) -> bool {
    request_body_media_type(headers).is_some_and(media_type::is_inspected_request_body)
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

fn plain_response(status: StatusCode, message: &'static str) -> Response<Body> {
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

fn record_interception(uri_bad: bool, headers_bad: bool, body_bad: bool) {
    crate::telemetry::SecurityStore::global().record_rasp_interception(
        uri_bad,
        headers_bad,
        body_bad,
    );
}

fn forbidden_response(uri_bad: bool, headers_bad: bool, body_bad: bool) -> Response<Body> {
    record_interception(uri_bad, headers_bad, body_bad);
    (
        StatusCode::FORBIDDEN,
        "RASP Security Violation: malicious payload intercepted.",
    )
        .into_response()
}

/// Stateless signature checks used by [`RaspSecurityLayer`]; see the module
/// documentation for coverage and limits.
pub struct RaspInspector;

impl RaspInspector {
    /// Inspects text for known attack signatures, including one layer of URL encoding.
    pub fn inspect_text(payload: &str) -> bool {
        inspect_value(payload, false)
    }

    /// Inspects an incoming request target URI and query string for attack patterns.
    pub fn inspect_uri(uri: &str) -> bool {
        Self::inspect_text(uri)
    }

    /// Inspects HTTP headers (User-Agent, Referer, Custom Headers) for injected attack payloads.
    ///
    /// Values are decoded lossily, so an obs-text byte (0x80-0xFF), which
    /// hyper accepts but `HeaderValue::to_str` rejects, cannot hide a payload.
    /// In `User-Agent`, the product token of a stock PowerShell client
    /// (`PowerShell/7.4.1`, `WindowsPowerShell/5.1…`) is not treated as the
    /// `powershell` command keyword; every other signature still applies.
    pub fn inspect_headers(headers: &HeaderMap) -> bool {
        for (name, val) in headers {
            if name == "cookie" || name == "authorization" {
                continue;
            }
            let is_user_agent = name == header::USER_AGENT;
            if inspect_value(&String::from_utf8_lossy(val.as_bytes()), is_user_agent) {
                return true;
            }
        }
        false
    }

    /// Inspects a bounded textual request body. JSON strings and keys are decoded before rules
    /// are applied so escaped payloads cannot bypass the raw-text pass.
    pub fn inspect_body(payload: &str, media_type: &str) -> bool {
        if Self::inspect_text(payload) {
            return true;
        }

        let is_json = media_type::is_json(media_type::essence(media_type));
        is_json
            && serde_json::from_str::<serde_json::Value>(payload)
                .ok()
                .is_some_and(|value| inspect_json_value(&value))
    }
}

/// Tower layer that refuses requests matching the RASP signatures.
///
/// See the module documentation for what is inspected and what is not.
#[derive(Clone, Default)]
pub struct RaspSecurityLayer;

impl<S> Layer<S> for RaspSecurityLayer {
    type Service = RaspSecurityService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RaspSecurityService { inner }
    }
}

/// Tower Service for RASP middleware.
#[derive(Clone)]
pub struct RaspSecurityService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for RaspSecurityService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let uri_str = req.uri().to_string();
        let headers_bad = RaspInspector::inspect_headers(req.headers());
        let uri_bad = RaspInspector::inspect_uri(&uri_str);

        if uri_bad || headers_bad {
            let res = forbidden_response(uri_bad, headers_bad, false);
            return Box::pin(async move { Ok(res) });
        }

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        if !should_inspect_body(req.headers()) {
            return Box::pin(async move { inner.call(req).await });
        }

        if !has_identity_encoding(req.headers()) {
            let response = plain_response(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Encoded request bodies cannot be inspected by RASP.",
            );
            return Box::pin(async move { Ok(response) });
        }

        if declared_body_is_too_large(req.headers()) {
            let response = plain_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Request body exceeds the RASP inspection limit.",
            );
            return Box::pin(async move { Ok(response) });
        }

        let media_type = request_body_media_type(req.headers())
            .unwrap_or("text/plain")
            .to_owned();
        Box::pin(async move {
            let (parts, body) = req.into_parts();
            let bytes = match axum::body::to_bytes(body, MAX_INSPECTED_REQUEST_BYTES).await {
                Ok(bytes) => bytes,
                Err(_) => {
                    return Ok(plain_response(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "Request body could not be inspected within the RASP limit.",
                    ));
                }
            };
            let payload = match std::str::from_utf8(&bytes) {
                Ok(payload) => payload,
                Err(_) => {
                    return Ok(plain_response(
                        StatusCode::BAD_REQUEST,
                        "Declared textual request body is not valid UTF-8.",
                    ));
                }
            };

            if RaspInspector::inspect_body(payload, &media_type) {
                return Ok(forbidden_response(false, false, true));
            }

            inner
                .call(Request::from_parts(parts, Body::from(bytes)))
                .await
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_ascii_fold_policy() {
        let byte: u8 = kani::any();
        let folded = fold_ascii_byte(byte);

        assert!(folded < b'A' || folded > b'Z');
        assert_eq!(fold_ascii_byte(folded), folded);
        if byte >= b'A' && byte <= b'Z' {
            assert_eq!(folded, byte + (b'a' - b'A'));
        } else {
            assert_eq!(folded, byte);
        }
    }
}
