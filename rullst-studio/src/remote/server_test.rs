//! A local server speaking `rullst.monitor.v1`, with hostile variants, and a
//! private temporary directory for the project list.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
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

/// What the server saw.
#[derive(Default)]
pub(super) struct Seen {
    pub(super) authorizations: Mutex<Vec<String>>,
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
/// `redirect` (to `redirect_to`) and `status`.
pub(super) async fn start(redirect_to: Option<String>) -> (String, Arc<Seen>) {
    let seen = Arc::new(Seen::default());
    let redirect_to = redirect_to.unwrap_or_default();
    let app = Router::new()
        .route("/ok/_rullst/monitor/v1", get(ok))
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
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{address}/"), seen)
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
