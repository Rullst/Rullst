//! Development counterpart of the WAF: it applies the same rules but only
//! logs the requests that staging and production would refuse.

use super::{
    MAX_INSPECTED_REQUEST_BYTES, MultipartFieldInspection, Rule, body_is_inspected,
    mark_multipart_fields, screen_body_bytes, screen_body_headers, screen_head,
};
use axum::{
    body::{Body, Bytes},
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body::{Body as HttpBody, Frame};
use std::collections::VecDeque;
use std::pin::Pin;

/// Logs, at `warn` level, the rule and path of each request that
/// [`super::waf_middleware`] would refuse, and passes every request on with
/// its body unchanged. `apply_security_baseline` mounts it in development, so
/// a false positive shows up before a deployment refuses it with `403`.
///
/// The log names only the rule and the path, never the query, headers or
/// body. A body over the 1 MiB inspection limit is replayed to the handler
/// in full. A body that fails to arrive gets `400`, as the handler could not
/// read it either. [`crate::uploads::Multipart`] logs multipart text fields
/// the same way.
pub(crate) async fn waf_report_middleware(mut req: Request, next: Next) -> Response {
    mark_multipart_fields(&mut req, MultipartFieldInspection::Report);
    if let Some(rule) = screen_head(&req) {
        would_refuse(rule, req.uri().path());
        return next.run(req).await;
    }
    if !body_is_inspected(&req) {
        return next.run(req).await;
    }
    if let Some(rule) = screen_body_headers(req.headers()) {
        would_refuse(rule, req.uri().path());
        return next.run(req).await;
    }

    let (parts, body) = req.into_parts();
    let body = match read_up_to(body, MAX_INSPECTED_REQUEST_BYTES).await {
        Read::Complete(bytes) => {
            if let Some(rule) = screen_body_bytes(&parts.headers, &bytes) {
                would_refuse(rule, parts.uri.path());
            }
            Body::from(bytes)
        }
        Read::TooLarge(body) => {
            would_refuse(Rule::BodyUnreadable, parts.uri.path());
            body
        }
        Read::Failed => {
            return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
        }
    };
    next.run(Request::from_parts(parts, body)).await
}

pub(super) fn would_refuse(rule: Rule, path: &str) {
    tracing::warn!(
        target: "rullst::security::waf",
        rule = rule.name(),
        path,
        "the WAF would refuse this request in staging and production"
    );
}

/// A request body read for inspection.
enum Read {
    /// The whole body, within the limit.
    Complete(Bytes),
    /// A body over the limit: the frames read so far, then the rest.
    TooLarge(Body),
    /// The body failed before its end.
    Failed,
}

/// Reads `body` until it ends or exceeds `limit` bytes.
async fn read_up_to(mut body: Body, limit: usize) -> Read {
    let mut frames = VecDeque::new();
    let mut length = 0_usize;
    loop {
        match std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await {
            None => break,
            Some(Err(_)) => return Read::Failed,
            Some(Ok(frame)) => {
                length = length.saturating_add(frame.data_ref().map_or(0, Bytes::len));
                frames.push_back(frame);
                if length > limit {
                    return Read::TooLarge(crate::security::csrf_multipart::replay(frames, body));
                }
            }
        }
    }
    let mut bytes = Vec::with_capacity(length);
    for data in frames.iter().filter_map(Frame::data_ref) {
        bytes.extend_from_slice(data);
    }
    Read::Complete(Bytes::from(bytes))
}
