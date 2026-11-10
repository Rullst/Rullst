//! The recording layer: method, matched route, status, duration and body
//! bytes of every response, plus the ORM operations the request started.
//!
//! The layer is the outermost but one, so the bytes it counts are the body
//! as sent: after any compression the application applies, without HTTP
//! headers or TLS. A body of known length counts when the response is
//! recorded; a streamed one when it ends or the client goes away. `HEAD`
//! responses send no body and count none.

use super::MONITOR_PATH;
use super::collector::{Collector, RouteHandle};
use crate::server::dev_telemetry::request_scope::observe_shared;
use axum::body::{Body, Bytes};
use axum::extract::{MatchedPath, Request};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;
use http_body::{Body as HttpBody, Frame};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

/// Wraps every route of `router` (and its fallback) with the recorder. Add
/// it after the framework layers, so their refusals (`403`, `429`, `503`)
/// are counted too. Requests to [`MONITOR_PATH`] itself are not counted.
pub(crate) fn record_requests(router: axum::Router, collector: Arc<Collector>) -> axum::Router {
    router.layer(axum::middleware::from_fn(
        move |request: Request, next: Next| {
            let collector = Arc::clone(&collector);
            async move { record(collector, request, next).await }
        },
    ))
}

async fn record(collector: Arc<Collector>, request: Request, next: Next) -> Response {
    if request.uri().path() == MONITOR_PATH {
        return next.run(request).await;
    }
    let method = request.method().clone();
    // A cheap clone: the matched route is reference-counted.
    let route = request.extensions().get::<MatchedPath>().cloned();
    let started = Instant::now();
    let (response, queries) = observe_shared(next.run(request)).await;
    let known = if method == Method::HEAD {
        Some(0)
    } else {
        response.body().size_hint().exact()
    };
    let handle = collector.observe(
        &method,
        route.as_ref().map(MatchedPath::as_str),
        response.status().as_u16(),
        started.elapsed(),
        &queries,
        known,
    );
    if known.is_some() {
        return response;
    }
    response.map(|body| {
        Body::new(Counted {
            inner: body,
            bytes: 0,
            report: Some((collector, handle)),
        })
    })
}

/// A streamed body that adds the bytes it sent to its route once it ends.
/// Its length is unknown, so the default size hint and end-of-stream answer
/// are the inner body's too.
struct Counted {
    inner: Body,
    bytes: u64,
    /// Taken when the bytes are reported, so they count once.
    report: Option<(Arc<Collector>, RouteHandle)>,
}

impl Counted {
    fn finish(&mut self) {
        if let Some((collector, handle)) = self.report.take() {
            collector.add_bytes(&handle, self.bytes);
        }
    }
}

impl HttpBody for Counted {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let polled = Pin::new(&mut self.inner).poll_frame(context);
        match &polled {
            Poll::Ready(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    let sent = u64::try_from(data.len()).unwrap_or(u64::MAX);
                    self.bytes = self.bytes.saturating_add(sent);
                }
            }
            Poll::Ready(_) => self.finish(),
            Poll::Pending => {}
        }
        polled
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.finish();
    }
}
