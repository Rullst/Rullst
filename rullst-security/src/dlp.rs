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

mod masking;

pub(crate) use masking::{SegmentRewriter, mask_text};

const MAX_BUFFERED_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;

fn textual_media_type(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::CONTENT_TYPE)?.to_str().ok()?;
    let media_type = crate::media_type::essence(value);

    if media_type.eq_ignore_ascii_case("text/event-stream") {
        return None;
    }

    crate::media_type::is_textual_response_body(media_type).then_some(media_type)
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
    withheld_response("response inspection failed")
}

/// A masked range no longer matches the byte range its `Content-Range`
/// names, and a single-part 206 without `Content-Range` is malformed
/// (RFC 9110 15.3.7). Forwarding the unmasked range would leak the secret,
/// so the partial response is withheld instead.
fn partial_content_withheld() -> Response<Body> {
    tracing::warn!(
        target: "rullst_security::dlp",
        "DLP withheld a 206 Partial Content response because masking would change its byte range"
    );
    withheld_response("partial response withheld by data loss prevention")
}

fn withheld_response(message: &'static str) -> Response<Body> {
    let mut response = Response::new(Body::from(message));
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
///
/// Masks complete PEM private-key blocks (`PRIVATE KEY`, `ENCRYPTED PRIVATE
/// KEY`, `RSA`, `EC`, `DSA` and `OPENSSH PRIVATE KEY`, and `PGP PRIVATE KEY
/// BLOCK`), 20-character AWS access-key IDs and
/// the password in `postgres://`, `postgresql://`, `mysql://`, `redis://` and
/// `rediss://` URLs. Each pass is linear in the input length. A URL password is masked
/// only when it appears inside the URL authority: credentials must be RFC 3986
/// percent-encoded, and the authority ends at the first `/`, `?`, `#`,
/// whitespace, quote, `<`, `>`, backtick or control character, or after
/// 2,048 bytes. The last `@` in the authority ends the userinfo.
pub fn mask_response_payload(input: &[u8]) -> (Vec<u8>, bool) {
    if input.is_empty() {
        return (Vec::new(), false);
    }

    let Ok(text) = std::str::from_utf8(input) else {
        return (input.to_vec(), false);
    };

    match masking::mask_text(text) {
        Some(sanitized) => {
            let store = SecurityStore::global();
            store.inc_dlp_masked();

            store.push_local_event(LiveSecurityEvent::local(
                "DLP_SECRET_LEAK_PREVENTED",
                "Neutralized secret credentials/key from outgoing HTTP response",
                "unknown",
            ));
            (sanitized.into_bytes(), true)
        }
        None => (input.to_vec(), false),
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
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
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
                if parts.status == StatusCode::PARTIAL_CONTENT {
                    return Ok(partial_content_withheld());
                }
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
mod tests;

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
