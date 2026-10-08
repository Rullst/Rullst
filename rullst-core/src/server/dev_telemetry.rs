//! Development-only request, ORM and queue telemetry for `cargo rullst dash`.
//!
//! `GET /_rullst/dev-telemetry` is mounted together with the development
//! reload routes: a debug build, the Development environment and a valid
//! supervisor-provided `RULLST_DEV_GENERATION`. Staging, production, release
//! builds and applications started without the CLI supervisor never mount it.
//! It answers only a loopback peer that names a loopback `Host` (and, when
//! present, a loopback `Origin`) over HTTP/1.1 or newer without proxy
//! forwarding headers; every other request receives `404`.
//!
//! The payload holds counters and bounded recent lists: request method, path
//! without query string, status and duration; ORM operation labels and
//! durations; ORM operation fingerprints (static labels) that one request
//! repeated at least [`crate::query_patterns::N_PLUS_ONE_THRESHOLD`] times,
//! with the request's matched route; a configured queue's pending count.
//! Request and response bodies,
//! headers, cookies, query strings, SQL text, bindings and error messages are
//! never recorded. Requests are recorded by [`record_responses`], the
//! outermost layer, so a panic answered by the development console and the
//! responses of the security, lifecycle and traffic layers are counted too.

#[cfg(test)]
mod n_plus_one_tests;
mod orm_layer;
mod recorder;
mod request_scope;
#[cfg(test)]
mod tests;

pub(crate) use orm_layer::{debug_layer, mark_installed};
use recorder::{record_repeated, record_request};

use crate::queue::Queue;
use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, Request, State, connect_info::MockConnectInfo},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
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
/// Headers a reverse proxy or tunnel adds; the dashboard never sends them.
const FORWARDING_HEADERS: [&str; 9] = [
    "forwarded",
    "x-forwarded-for",
    "x-forwarded-host",
    "x-forwarded-proto",
    "x-forwarded-server",
    "x-real-ip",
    "via",
    "cf-connecting-ip",
    "true-client-ip",
];

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

/// Wraps the complete application, as its outermost layer, with the request
/// recorder; added only where [`mount`] mounted the endpoint. Development
/// polls and, when `static_mounted`, the framework's `/static` files are not
/// counted, matching the access log.
pub(super) fn record_responses(router: Router, static_mounted: bool) -> Router {
    router.layer(axum::middleware::from_fn(
        move |request: Request, next: Next| async move {
            record(request, next, static_mounted).await
        },
    ))
}

async fn record(request: Request, next: Next, static_mounted: bool) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let route = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|route| route.as_str().to_string());
    let started = std::time::Instant::now();
    let (response, operations) = request_scope::observe(next.run(request)).await;
    if is_recorded(&path, static_mounted) {
        record_request(
            method.as_str(),
            &path,
            response.status().as_u16(),
            started.elapsed(),
        );
        record_repeated(
            method.as_str(),
            route.as_deref().unwrap_or(&path),
            &operations,
        );
    }
    response
}

fn is_recorded(path: &str, static_mounted: bool) -> bool {
    let static_file = path == "/static" || path.starts_with("/static/");
    super::console::is_logged(path) && !(static_mounted && static_file)
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
/// machines and clients resolved through trusted proxies. A same-host reverse
/// proxy or tunnel connects from loopback and may rewrite `Host`, so requests
/// with a forwarding header, or over HTTP/1.0 (nginx's default upstream
/// protocol), are refused as well.
fn is_local(request: &Request) -> bool {
    if matches!(
        request.version(),
        axum::http::Version::HTTP_09 | axum::http::Version::HTTP_10
    ) || FORWARDING_HEADERS
        .iter()
        .any(|name| request.headers().contains_key(*name))
    {
        return false;
    }
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
