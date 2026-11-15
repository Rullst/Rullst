//! A local server speaking `rullst.monitor.v1` and `rullst.monitor.v2`, with
//! hostile variants, and a private temporary directory for the project list.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The token every test project uses.
pub(super) const TOKEN: &str = "fH6MncgeJPW9jH2ICx_2ylFrxtHa5ztWWiN4xPEgE-I";

/// A valid document whose route label and application name are `label`.
pub(super) fn document(label: &str) -> Value {
    let window = json!({
        "covered_seconds": 120,
        "requests": 40,
        "requests_per_second": 0.333,
        "status": {"1xx": 0, "2xx": 36, "3xx": 0, "4xx": 2, "5xx": 2},
        "latency_ms": {"p50": 2.5, "p90": 10.0, "p95": 25.0, "p99": 50.0, "max": 61.2}
    });
    json!({
        "schema": "rullst.monitor.v1",
        "generated_at": 1_792_648_800_u64,
        "application": {"name": label, "version": "1.4.0"},
        "framework": {"name": "rullst", "version": "13.0.0-alpha.3"},
        "uptime_seconds": 90_061,
        "process": {"memory_rss_mb": 48.2, "cpu_usage_percent": 3.5, "active_tokio_tasks": 42, "tokio_latency_micros": 18},
        "http": {
            "window_seconds": 300,
            "route_limit": 256,
            "routes_overflowed": false,
            "requests_total": 1200,
            "status_total": {"1xx": 0, "2xx": 1150, "3xx": 10, "4xx": 30, "5xx": 10},
            "window": window,
            "routes": [{
                "method": "GET",
                "route": label,
                "requests_total": 1200,
                "status_total": {"1xx": 0, "2xx": 1150, "3xx": 10, "4xx": 30, "5xx": 10},
                "window": window
            }]
        },
        "database": {
            "state": "observed",
            "queries_total": 9876,
            "n_plus_one": {"threshold": 3, "findings_total": 1, "findings": [{
                "method": "GET", "route": label, "fingerprint": "LineItem.find (line_items)",
                "requests": 12, "max_occurrences": 25, "last_seen": 1_792_648_790_u64
            }]}
        },
        "queues": [{
            "name": "mail", "pending": {"state": "observed", "count": 3},
            "in_flight": 1, "completed_total": 500, "failed_total": 4,
            "retried_total": 2, "stalled_recovered_total": 0
        }],
        "future_field": {"ignored": true}
    })
}

/// A valid version 2 document: [`document`] plus traces and the security
/// radar, whose route, span kind, counter and event labels are `label`.
pub(super) fn document_v2(label: &str) -> Value {
    let mut value = document(label);
    value["schema"] = json!("rullst.monitor.v2");
    value["traces"] = json!({
        "sample_limit": 50,
        "sampled_total": 321,
        "requests": [
            {"at": 1_792_648_790_u64, "method": "GET", "route": label, "status": 503,
             "duration_ms": 1_250.5, "db_operations": 4, "n_plus_one": true},
            {"at": 1_792_648_700_u64, "method": "POST", "route": "/orders", "status": 201,
             "duration_ms": 12.25, "db_operations": 1, "n_plus_one": false}
        ],
        "spans": {"kept": 7, "kinds": [
            {"kind": "sql", "count": 5, "total_us": 4_500, "max_us": 2_000, "last_at": 1_792_648_780_u64},
            {"kind": label, "count": 2, "total_us": 30, "max_us": 20, "last_at": 1_792_648_000_u64}
        ]}
    });
    value["security"] = json!({
        "events_total": 17,
        "active_bans": 2,
        "counters": {"rate_limit_blocks": 40, "login_jail_bans": 3, "honeypot_traps": 2},
        "events": [
            {"type": "LOGIN_JAIL_TRIGGERED", "at": 1_792_648_795_u64},
            {"type": label, "at": 1_792_648_600_u64}
        ]
    });
    value["security"]["counters"][label] = json!(9);
    value
}

/// What the server saw.
#[derive(Default)]
pub(super) struct Seen {
    pub(super) authorizations: Mutex<Vec<String>>,
    /// Requests for a version 2 document that this server does not serve.
    pub(super) v2_misses: AtomicUsize,
}

impl Seen {
    pub(super) fn v2_misses(&self) -> usize {
        self.v2_misses.load(Ordering::SeqCst)
    }
}

async fn missing(State(seen): State<Arc<Seen>>, uri: Uri) -> StatusCode {
    if uri.path().ends_with("/_rullst/monitor/v2") {
        seen.v2_misses.fetch_add(1, Ordering::SeqCst);
    }
    StatusCode::NOT_FOUND
}

fn json_response(body: impl Into<Body>) -> Response {
    ([(header::CONTENT_TYPE, "application/json")], body.into()).into_response()
}

async fn ok(State(seen): State<Arc<Seen>>, headers: HeaderMap) -> Response {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    seen.authorizations
        .lock()
        .unwrap()
        .push(authorization.clone());
    if authorization != format!("Bearer {TOKEN}") {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    json_response(document("/orders/{id}").to_string())
}

/// Starts the server; variants are path prefixes before `_rullst/monitor/v1`:
/// `ok`, `xss`, `huge`, `stream`, `malformed`, `schema`, `slow`, `html`,
/// `redirect` (to `redirect_to`) and `status`; and before
/// `_rullst/monitor/v2`: `v2`, `v2xss`, `v2bare` (no version 2 sections) and
/// `v2error` (`500`). Other version 2 paths answer `404`.
pub(super) async fn start(redirect_to: Option<String>) -> (String, Arc<Seen>) {
    let seen = Arc::new(Seen::default());
    let redirect_to = redirect_to.unwrap_or_default();
    let app = Router::new()
        .route("/ok/_rullst/monitor/v1", get(ok))
        .route(
            "/v2/_rullst/monitor/v2",
            get(|| async { json_response(document_v2("/orders/{id}").to_string()) }),
        )
        .route(
            "/v2xss/_rullst/monitor/v2",
            get(|| async {
                json_response(document_v2("<script>alert('xss')</script>").to_string())
            }),
        )
        .route(
            "/v2bare/_rullst/monitor/v2",
            get(|| async {
                let mut value = document("/");
                value["schema"] = json!("rullst.monitor.v2");
                json_response(value.to_string())
            }),
        )
        .route(
            "/v2error/_rullst/monitor/v2",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR.into_response() }),
        )
        .route(
            "/xss/_rullst/monitor/v1",
            get(|| async {
                json_response(
                    document("<script>alert('xss')</script>\"><img src=x onerror=alert(1)>")
                        .to_string(),
                )
            }),
        )
        .route(
            "/huge/_rullst/monitor/v1",
            get(|| async { json_response(" ".repeat(2 * 1024 * 1024)) }),
        )
        .route(
            "/stream/_rullst/monitor/v1",
            get(|| async {
                let chunks = (0..64).map(|_| Ok::<_, std::io::Error>(vec![b' '; 32 * 1024]));
                json_response(Body::from_stream(futures_util::stream::iter(chunks)))
            }),
        )
        .route(
            "/malformed/_rullst/monitor/v1",
            get(|| async { json_response("{\"schema\": ") }),
        )
        .route(
            "/schema/_rullst/monitor/v1",
            get(|| async {
                let mut value = document("/");
                value["schema"] = json!("rullst.monitor.v2");
                json_response(value.to_string())
            }),
        )
        .route(
            "/slow/_rullst/monitor/v1",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                json_response(document("/").to_string())
            }),
        )
        .route(
            "/html/_rullst/monitor/v1",
            get(|| async {
                ([(header::CONTENT_TYPE, "text/html")], "<html></html>").into_response()
            }),
        )
        .route(
            "/redirect/_rullst/monitor/v1",
            get(move || {
                let target = redirect_to.clone();
                async move { (StatusCode::FOUND, [(header::LOCATION, target)]).into_response() }
            }),
        )
        .route(
            "/status/_rullst/monitor/v1",
            get(|| async { StatusCode::UPGRADE_REQUIRED.into_response() }),
        )
        .fallback(missing)
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{address}/"), seen)
}

/// A loopback port nothing listens on, reserved while this value lives.
///
/// It is the local end of a connection to a listener this test owned, so a
/// connection to it is refused and no parallel test can bind it. A port read
/// from a dropped listener can be handed to another test's server before it
/// is used, which then answers in place of a refusal.
pub(super) struct RefusedPort {
    pub(super) port: u16,
    _connection: (std::net::TcpStream, std::net::TcpStream),
}

impl RefusedPort {
    pub(super) fn new() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let client = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        Self {
            port: client.local_addr().unwrap().port(),
            _connection: (client, server),
        }
    }
}

/// A temporary directory removed on drop.
pub(super) struct TempDir(PathBuf);

impl TempDir {
    pub(super) fn new(label: &str) -> Self {
        let mut bytes = [0_u8; 8];
        ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut bytes).unwrap();
        let suffix: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        let path = std::env::temp_dir().join(format!("rullst-studio-{label}-{suffix}"));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(super) fn path(&self) -> &Path {
        &self.0
    }

    pub(super) fn paths(&self) -> super::StudioPaths {
        super::StudioPaths::in_directories(&self.0.join("config"), &self.0.join("data"))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
