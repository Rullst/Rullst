//! Development-only request, ORM and queue telemetry for `cargo rullst dash`.
//!
//! `GET /_rullst/dev-telemetry` is mounted together with the development
//! reload routes: a debug build, the Development environment and a valid
//! supervisor-provided `RULLST_DEV_GENERATION`. Staging, production, release
//! builds and applications started without the CLI supervisor never mount it.
//! It answers only a loopback peer that names a loopback `Host` (and, when
//! present, a loopback `Origin`); every other request receives `404`.
//!
//! The payload holds counters and bounded recent lists: request method, path
//! without query string, status and duration; ORM operation labels and
//! durations; a configured queue's pending count. Request and response bodies,
//! headers, cookies, query strings, SQL text, bindings and error messages are
//! never recorded.

mod orm_layer;
mod recorder;
#[cfg(test)]
mod tests;

pub(crate) use orm_layer::{debug_layer, mark_installed};
pub(crate) use recorder::record_request;

use crate::queue::Queue;
use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, Request, State, connect_info::MockConnectInfo},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use recorder::{HttpSnapshot, QuerySnapshot, Recorder};
use serde::Serialize;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

/// Path polled by `cargo rullst dash`.
pub(super) const PATH: &str = "/_rullst/dev-telemetry";
/// Version of the JSON document served at [`PATH`].
const SCHEMA: &str = "rullst.dev-telemetry.v1";
/// Longest wait for a configured queue's pending count.
const QUEUE_PROBE_TIMEOUT: Duration = Duration::from_millis(250);

/// A `GET`/`HEAD` of the telemetry endpoint.
pub(super) fn is_telemetry_request(request: &Request) -> bool {
    matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD
    ) && request.uri().path() == PATH
}

/// Mounts the endpoint only where the development reload routes are mounted.
pub(super) fn mount(
    router: Router,
    development: bool,
    generation: Option<String>,
    queue: Option<Arc<Queue>>,
) -> Router {
    if !super::dev_reload::is_enabled(development, generation.as_deref()) {
        return router;
    }
    let Some(generation) = generation else {
        return router;
    };
    router.route(PATH, routes(recorder::enable_global(), generation, queue))
}

fn routes(
    recorder: Arc<Recorder>,
    generation: String,
    queue: Option<Arc<Queue>>,
) -> axum::routing::MethodRouter {
    get(serve).with_state(Endpoint {
        recorder,
        generation: Arc::from(generation),
        queue,
    })
}

#[derive(Clone)]
struct Endpoint {
    recorder: Arc<Recorder>,
    generation: Arc<str>,
    queue: Option<Arc<Queue>>,
}

#[derive(Serialize)]
struct Payload<'a> {
    schema: &'static str,
    generation: &'a str,
    uptime_ms: u64,
    http: HttpSnapshot,
    database: Database,
    queue: QueueDepth,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum Database {
    Observed(QuerySnapshot),
    Unavailable { reason: &'static str },
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum QueueDepth {
    Observed { pending: u64 },
    NotConfigured,
    Unavailable { reason: &'static str },
}

async fn serve(State(endpoint): State<Endpoint>, request: Request) -> Response {
    if !is_local(&request) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let payload = Payload {
        schema: SCHEMA,
        generation: &endpoint.generation,
        uptime_ms: u64::try_from(endpoint.recorder.uptime().as_millis()).unwrap_or(u64::MAX),
        http: endpoint.recorder.http_snapshot(),
        database: database(
            &endpoint.recorder,
            orm_layer::installed(),
            orm_spans_enabled(),
        ),
        queue: queue_depth(endpoint.queue.as_deref()).await,
    };
    let Ok(body) = serde_json::to_vec(&payload) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = Response::new(Body::from(body));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn database(recorder: &Recorder, layer_installed: bool, spans_enabled: bool) -> Database {
    if !layer_installed {
        Database::Unavailable {
            reason: "subscriber_not_installed",
        }
    } else if !spans_enabled {
        Database::Unavailable {
            reason: "orm_spans_filtered",
        }
    } else {
        Database::Observed(recorder.query_snapshot())
    }
}

/// Whether the installed subscriber keeps `rullst_orm` INFO spans (a
/// `RUST_LOG` such as `warn` disables them, and with them the counts).
fn orm_spans_enabled() -> bool {
    tracing::enabled!(
        kind: tracing::metadata::Kind::SPAN,
        target: "rullst_orm",
        tracing::Level::INFO
    )
}

async fn queue_depth(queue: Option<&Queue>) -> QueueDepth {
    let Some(queue) = queue else {
        return QueueDepth::NotConfigured;
    };
    match tokio::time::timeout(QUEUE_PROBE_TIMEOUT, queue.pending_count()).await {
        Ok(Ok(pending)) => QueueDepth::Observed { pending },
        // Driver errors can name hosts or paths; only a fixed reason leaves.
        Ok(Err(_)) => QueueDepth::Unavailable {
            reason: "driver_error",
        },
        Err(_) => QueueDepth::Unavailable { reason: "timeout" },
    }
}

/// A direct loopback peer that addresses the server by a loopback authority.
/// The `Host` check rejects DNS-rebinding pages; the peer check rejects other
/// machines and clients resolved through trusted proxies.
fn is_local(request: &Request) -> bool {
    let extensions = request.extensions();
    let peer = extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| *peer)
        .or_else(|| {
            extensions
                .get::<MockConnectInfo<SocketAddr>>()
                .map(|MockConnectInfo(peer)| *peer)
        });
    if !peer.is_some_and(|peer| peer.ip().to_canonical().is_loopback()) {
        return false;
    }
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            request
                .uri()
                .authority()
                .map(|authority| authority.as_str())
        });
    if !host.is_some_and(is_loopback_authority) {
        return false;
    }
    match request.headers().get(header::ORIGIN) {
        None => true,
        Some(origin) => origin
            .to_str()
            .ok()
            .and_then(|origin| {
                origin
                    .strip_prefix("http://")
                    .or_else(|| origin.strip_prefix("https://"))
            })
            .is_some_and(is_loopback_authority),
    }
}

fn is_loopback_authority(value: &str) -> bool {
    let Ok(authority) = value.parse::<axum::http::uri::Authority>() else {
        return false;
    };
    if authority.as_str().contains('@') {
        return false;
    }
    let host = authority.host();
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok_and(|address| address.to_canonical().is_loopback())
}
