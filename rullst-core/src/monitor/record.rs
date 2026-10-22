//! The recording layer: method, matched route, status and duration of every
//! response, plus the ORM operations the request started.

use super::MONITOR_PATH;
use super::collector::Collector;
use crate::server::dev_telemetry::request_scope::observe_shared;
use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;
use std::time::Instant;

/// Wraps every route of `router` (and its fallback) with the recorder. Add
/// it after the framework layers, so their refusals (`403`, `429`, `503`)
/// are counted too. Requests to [`MONITOR_PATH`] itself are not counted.
pub(crate) fn record_requests(router: axum::Router, collector: Arc<Collector>) -> axum::Router {
    router.layer(axum::middleware::from_fn(
        move |request: Request, next: Next| {
            let collector = Arc::clone(&collector);
            async move { record(&collector, request, next).await }
        },
    ))
}

async fn record(collector: &Collector, request: Request, next: Next) -> Response {
    if request.uri().path() == MONITOR_PATH {
        return next.run(request).await;
    }
    let method = request.method().clone();
    // A cheap clone: the matched route is reference-counted.
    let route = request.extensions().get::<MatchedPath>().cloned();
    let started = Instant::now();
    let (response, queries) = observe_shared(next.run(request)).await;
    collector.record(
        &method,
        route.as_ref().map(MatchedPath::as_str),
        response.status().as_u16(),
        started.elapsed(),
        &queries,
    );
    response
}
