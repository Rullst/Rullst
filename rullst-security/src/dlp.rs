//! Data Loss Prevention (DLP) Response Interceptor.
//! Intercepts HTTP response streams to prevent accidental leaks of private keys, AWS credentials, and database secrets.

use crate::telemetry::{LiveSecurityEvent, SecurityStore};
use axum::{
    body::{Body, HttpBody},
    http::{
        HeaderMap, HeaderValue, Method, Request, Response, StatusCode,
        header::{self, HeaderName},
    },
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

const MAX_BUFFERED_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;

fn textual_media_type(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::CONTENT_TYPE)?.to_str().ok()?;
    let media_type = value.split(';').next()?.trim();

    if media_type.eq_ignore_ascii_case("text/event-stream") {
        return None;
    }

    let is_text = media_type
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("text/"));
    let is_json = media_type.eq_ignore_ascii_case("application/json")
        || media_type
            .strip_suffix("+json")
            .is_some_and(|prefix| prefix.starts_with("application/"));

    (is_text || is_json).then_some(media_type)
}

fn has_identity_encoding(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_ENCODING)
        .is_none_or(|value| value.as_bytes().eq_ignore_ascii_case(b"identity"))
}

/// Only fixed, in-memory bodies are inspected. Unknown-size bodies are likely streams and must
/// pass through untouched because consuming a partial stream cannot be rolled back safely.
fn is_safely_bufferable(headers: &HeaderMap, body: &Body) -> bool {
    let hint = body.size_hint();
    let Some(upper) = hint.upper() else {
        return false;
    };

    if !is_fixed_body_size_within_limit(hint.lower(), upper) {
        return false;
    }

    match headers.get(header::CONTENT_LENGTH) {
        Some(value) => value
            .to_str()
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|declared| declared == upper),
        None => true,
    }
}

const fn is_fixed_body_size_within_limit(lower: u64, upper: u64) -> bool {
    lower == upper && upper <= MAX_BUFFERED_RESPONSE_BYTES
}

fn remove_stale_representation_headers(headers: &mut HeaderMap, body_len: usize) {
    headers.remove(header::ETAG);
    headers.remove(header::CONTENT_RANGE);
    headers.remove(header::ACCEPT_RANGES);
    headers.remove(HeaderName::from_static("content-md5"));
    headers.remove(HeaderName::from_static("digest"));
    headers.remove(HeaderName::from_static("content-digest"));

    headers.remove(header::CONTENT_LENGTH);
    if let Ok(value) = HeaderValue::from_str(&body_len.to_string()) {
        headers.insert(header::CONTENT_LENGTH, value);
    }
}

fn body_collection_failure() -> Response<Body> {
    let mut response = Response::new(Body::from("response inspection failed"));
    *response.status_mut() = StatusCode::BAD_GATEWAY;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// Masks sensitive patterns from response payloads. Returns (sanitized_bytes, was_masked).
pub fn mask_response_payload(input: &[u8]) -> (Vec<u8>, bool) {
    if input.is_empty() {
        return (Vec::new(), false);
    }

    let Ok(text) = std::str::from_utf8(input) else {
        return (input.to_vec(), false);
    };
    let mut sanitized = text.to_owned();
    let mut modified = false;

    // 1. Mask Private Keys (RSA / OpenSSH / Generic)
    for (begin, end_marker) in [
        ("-----BEGIN PRIVATE KEY-----", "-----END PRIVATE KEY-----"),
        (
            "-----BEGIN RSA PRIVATE KEY-----",
            "-----END RSA PRIVATE KEY-----",
        ),
        (
            "-----BEGIN OPENSSH PRIVATE KEY-----",
            "-----END OPENSSH PRIVATE KEY-----",
        ),
    ] {
        let mut cursor = 0;
        while let Some(offset) = sanitized[cursor..].find(begin) {
            let start = cursor + offset;
            let search_from = start + begin.len();
            let Some(end_offset) = sanitized[search_from..].find(end_marker) else {
                break;
            };
            let end = search_from + end_offset + end_marker.len();
            sanitized.replace_range(start..end, "[DLP_BLOCKED_PRIVATE_KEY]");
            modified = true;
            cursor = start + "[DLP_BLOCKED_PRIVATE_KEY]".len();
        }
    }

    // 2. Mask all AWS Access Keys (AKIA...)
    let mut cursor = 0;
    while let Some(offset) = sanitized[cursor..].find("AKIA") {
        let start = cursor + offset;
        let remainder = &sanitized.as_bytes()[start..];
        let is_access_key = remainder.len() >= 20
            && remainder[..20]
                .iter()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit());

        if is_access_key {
            sanitized.replace_range(start..start + 20, "AKIA****************");
            modified = true;
            cursor = start + 20;
        } else {
            cursor = start + 4;
        }
        if cursor >= sanitized.len() {
            break;
        }
    }

    // 3. Mask all database connection string passwords (postgres://user:pass@host:5432/db)
    for scheme in &["postgres://", "postgresql://", "mysql://", "redis://"] {
        let mut cursor = 0;
        while let Some(offset) = sanitized[cursor..].find(scheme) {
            let start = cursor + offset;
            let rest = &sanitized[start + scheme.len()..];
            if let Some(at_idx) = rest.find('@') {
                let auth_part = &rest[..at_idx];
                if let Some(colon_idx) = auth_part.find(':') {
                    let pass_start = start + scheme.len() + colon_idx + 1;
                    let pass_end = start + scheme.len() + at_idx;
                    if &sanitized[pass_start..pass_end] != "*****" {
                        sanitized.replace_range(pass_start..pass_end, "*****");
                        modified = true;
                    }
                    cursor = start + scheme.len() + at_idx + 1;
                    if cursor >= sanitized.len() {
                        break;
                    }
                    continue;
                }
            }
            cursor = start + scheme.len();
            if cursor >= sanitized.len() {
                break;
            }
        }
    }

    if modified {
        let store = SecurityStore::global();
        store.inc_dlp_masked();

        store.push_local_event(LiveSecurityEvent::local(
            "DLP_SECRET_LEAK_PREVENTED",
            "Neutralized secret credentials/key from outgoing HTTP response",
            "unknown",
        ));
        (sanitized.into_bytes(), true)
    } else {
        (input.to_vec(), false)
    }
}

/// Tower Layer for Data Loss Prevention (DLP) response interception.
#[derive(Clone, Default)]
pub struct DlpResponseLayer;

impl<S> Layer<S> for DlpResponseLayer {
    type Service = DlpResponseService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DlpResponseService { inner }
    }
}

/// Tower Service for DLP response interception.
#[derive(Clone)]
pub struct DlpResponseService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for DlpResponseService<S>
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
        let mut inner = self.inner.clone();
        let request_method = req.method().clone();

        Box::pin(async move {
            let res = inner.call(req).await?;

            if request_method == Method::HEAD
                || res.status().is_informational()
                || matches!(
                    res.status(),
                    StatusCode::NO_CONTENT | StatusCode::NOT_MODIFIED
                )
                || textual_media_type(res.headers()).is_none()
                || !has_identity_encoding(res.headers())
                || !is_safely_bufferable(res.headers(), res.body())
            {
                return Ok(res);
            }

            let (mut parts, body) = res.into_parts();
            let bytes = match axum::body::to_bytes(body, MAX_BUFFERED_RESPONSE_BYTES as usize).await
            {
                Ok(bytes) => bytes,
                Err(_) => return Ok(body_collection_failure()),
            };

            let (sanitized_bytes, was_modified) = mask_response_payload(&bytes);
            if was_modified {
                remove_stale_representation_headers(&mut parts.headers, sanitized_bytes.len());
            }

            Ok(Response::from_parts(parts, Body::from(sanitized_bytes)))
        })
    }
}

/// Alias for DlpResponseLayer
pub type DlpLayer = DlpResponseLayer;

/// Alias for DlpResponseService
pub type DlpService<S> = DlpResponseService<S>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_private_key() {
        let payload = b"Error: key was -----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA...\n-----END RSA PRIVATE KEY----- in config";
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(was_modified);
        let masked_str = String::from_utf8(masked).unwrap();
        assert!(masked_str.contains("[DLP_BLOCKED_PRIVATE_KEY]"));
        assert!(!masked_str.contains("BEGIN RSA PRIVATE KEY"));
    }

    #[test]
    fn test_mask_database_url() {
        let payload =
            b"{\"db\": \"postgres://admin:super_secret_password_123@localhost:5432/app\"}";
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(was_modified);
        let masked_str = String::from_utf8(masked).unwrap();
        assert!(masked_str.contains("postgres://admin:*****@localhost:5432/app"));
        assert!(!masked_str.contains("super_secret_password_123"));
    }

    #[test]
    fn test_clean_payload_untouched() {
        let payload = b"{\"message\": \"Hello Rullst!\", \"status\": 200}";
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(!was_modified);
        assert_eq!(masked, payload);
    }

    #[test]
    fn invalid_utf8_and_incomplete_pem_are_not_corrupted() {
        let binary = b"\xff\xfeAKIAIOSFODNN7EXAMPLE";
        let (masked, was_modified) = mask_response_payload(binary);
        assert!(!was_modified);
        assert_eq!(masked, binary);

        let incomplete = b"prefix -----BEGIN PRIVATE KEY----- unfinished payload";
        let (masked, was_modified) = mask_response_payload(incomplete);
        assert!(!was_modified);
        assert_eq!(masked, incomplete);
    }

    #[test]
    fn test_mask_aws_access_key() {
        let payload = b"{\"aws_key\": \"AKIAIOSFODNN7EXAMPLE\"}";
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(was_modified);
        let masked_str = String::from_utf8(masked).unwrap();
        assert!(masked_str.contains("AKIA****************"));
        assert!(!masked_str.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_mask_mysql_and_redis_urls() {
        let payload = b"mysql://root:my_secret_sql_pass@127.0.0.1:3306/db and redis://default:redis_auth_token@cache:6379";
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(was_modified);
        let masked_str = String::from_utf8(masked).unwrap();
        assert!(masked_str.contains("mysql://root:*****@127.0.0.1:3306/db"));
        assert!(masked_str.contains("redis://default:*****@cache:6379"));
    }

    #[test]
    fn test_mask_multiple_keys_and_dsns() {
        let payload = br#"{"keys": ["AKIA1111111111111111", "AKIA2222222222222222"], "dbs": ["postgres://u1:p1@h1:5432/d1", "postgres://u2:p2@h2:5432/d2"]}"#;
        let (masked, was_modified) = mask_response_payload(payload);
        assert!(was_modified);
        let masked_str = String::from_utf8(masked).unwrap();
        assert!(!masked_str.contains("AKIA1111111111111111"));
        assert!(!masked_str.contains("AKIA2222222222222222"));
        assert!(!masked_str.contains(":p1@"));
        assert!(!masked_str.contains(":p2@"));
        assert!(masked_str.contains("postgres://u1:*****@h1:5432/d1"));
        assert!(masked_str.contains("postgres://u2:*****@h2:5432/d2"));
    }

    #[tokio::test]
    async fn test_dlp_layer_middleware() {
        use axum::http::{Request, StatusCode, header};
        use axum::response::IntoResponse;
        use axum::routing::get;
        use tower::ServiceExt;

        async fn secret_handler() -> impl IntoResponse {
            let mut response = (
                StatusCode::OK,
                "Config leaked: postgres://user:secret123@db:5432/main",
            )
                .into_response();
            response
                .headers_mut()
                .insert(header::ETAG, HeaderValue::from_static("\"old-validator\""));
            response
        }

        async fn binary_handler() -> Response<Body> {
            let payload = b"\xffpostgres://user:secret123@db:5432/main".to_vec();
            let mut response = Response::new(Body::from(payload));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            response
        }

        async fn event_stream_handler() -> Response<Body> {
            let mut response = Response::new(Body::from(
                "data: postgres://user:secret123@db:5432/main\n\n",
            ));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/event-stream"),
            );
            response
        }

        async fn oversized_handler() -> Response<Body> {
            let mut payload = vec![b'a'; MAX_BUFFERED_RESPONSE_BYTES as usize + 1];
            payload.extend_from_slice(b"postgres://user:secret123@db:5432/main");
            let mut response = Response::new(Body::from(payload));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );
            response
        }

        let app = axum::Router::new()
            .route("/secret", get(secret_handler))
            .route("/binary", get(binary_handler))
            .route("/events", get(event_stream_handler))
            .route("/oversized", get(oversized_handler))
            .layer(DlpResponseLayer);

        let req = Request::builder()
            .uri("/secret")
            .body(Body::empty())
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().get(header::ETAG).is_none());
        let declared_length = resp
            .headers()
            .get(header::CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 10000).await.unwrap();
        assert_eq!(declared_length, body.len());
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("postgres://user:*****@db:5432/main"));
        assert!(!body_str.contains("secret123"));

        for (path, limit) in [
            ("/binary", 10_000usize),
            ("/events", 10_000usize),
            ("/oversized", MAX_BUFFERED_RESPONSE_BYTES as usize + 10_000),
        ] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(response.into_body(), limit)
                .await
                .unwrap();
            assert!(
                bytes
                    .windows(b"secret123".len())
                    .any(|window| window == b"secret123"),
                "{path} must bypass DLP without truncation"
            );
        }
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_buffer_size_gate_boundaries() {
        let lower: u64 = kani::any();
        let upper: u64 = kani::any();
        let accepted = is_fixed_body_size_within_limit(lower, upper);

        assert_eq!(
            accepted,
            lower == upper && upper <= MAX_BUFFERED_RESPONSE_BYTES
        );
        if accepted {
            assert_eq!(lower, upper);
            assert!(upper <= MAX_BUFFERED_RESPONSE_BYTES);
        }
    }
}
