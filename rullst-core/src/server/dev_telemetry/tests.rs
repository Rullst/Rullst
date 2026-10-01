#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::orm_layer::OrmQueryLayer;
use super::recorder::{QueryLabels, RECENT_REQUESTS, RECENT_SLOW_QUERIES, Recorder};
use super::*;
use axum::http::Request as HttpRequest;
use std::time::Duration;
use tower::ServiceExt;

const MARKER: &str = "0123456789abcdef0123456789abcdef";

fn endpoint(recorder: Arc<Recorder>, queue: Option<Arc<Queue>>) -> Router {
    Router::new().route(PATH, routes(recorder, MARKER.to_string(), queue))
}

fn request(peer: Option<&str>, host: Option<&str>, origin: Option<&str>) -> Request {
    let mut builder = HttpRequest::builder().uri(PATH);
    if let Some(host) = host {
        builder = builder.header(header::HOST, host);
    }
    if let Some(origin) = origin {
        builder = builder.header(header::ORIGIN, origin);
    }
    let mut request = builder.body(Body::empty()).unwrap();
    if let Some(peer) = peer {
        request
            .extensions_mut()
            .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    }
    request
}

fn local() -> Request {
    request(Some("127.0.0.1:50000"), Some("127.0.0.1:3000"), None)
}

async fn json(router: &Router, request: Request) -> (StatusCode, serde_json::Value) {
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, value)
}

#[test]
fn request_counters_classify_statuses_and_keep_only_the_newest_samples() {
    let recorder = Recorder::new();
    for index in 0..(RECENT_REQUESTS + 6) {
        let status = match index % 3 {
            0 => 200,
            1 => 404,
            _ => 503,
        };
        recorder.record_request(
            "GET",
            &format!("/items/{index}"),
            status,
            Duration::from_micros(1_500),
        );
    }
    let snapshot = recorder.http_snapshot();
    assert_eq!(snapshot.requests_total, 70);
    assert_eq!(snapshot.client_errors_total, 23);
    assert_eq!(snapshot.server_errors_total, 23);
    assert_eq!(snapshot.recent.len(), RECENT_REQUESTS);
    assert_eq!(snapshot.recent.first().unwrap().seq, 7);
    let newest = snapshot.recent.last().unwrap();
    assert_eq!(newest.seq, snapshot.requests_total);
    assert_eq!(newest.path, "/items/69");
    assert_eq!(newest.duration_us, 1_500);
}

#[test]
fn recorded_text_is_bounded_and_free_of_control_characters() {
    let recorder = Recorder::new();
    let long = format!("/{}", "é".repeat(400));
    recorder.record_request("GET\u{1b}[31m", &long, 200, Duration::ZERO);
    recorder.record_request("GET", "/a\u{7}\nb", 200, Duration::ZERO);
    let recent = recorder.http_snapshot().recent;
    assert!(recent[0].path.len() <= 256);
    assert!(recent[0].path.starts_with("/éé"));
    assert_eq!(recent[0].method, "GET?[31m");
    assert_eq!(recent[1].path, "/a??b");
    assert!(
        recent
            .iter()
            .all(|sample| !sample.path.chars().any(char::is_control))
    );
}

#[test]
fn only_operations_at_the_threshold_are_slow_and_the_list_is_bounded() {
    let recorder = Recorder::new();
    recorder.record_query(QueryLabels::default(), Duration::from_millis(99));
    for index in 0..(RECENT_SLOW_QUERIES + 2) {
        recorder.record_query(
            QueryLabels {
                operation: Some("select_many".into()),
                model: Some(format!("Model{index}")),
                table: Some("x".repeat(200)),
            },
            Duration::from_millis(100),
        );
    }
    let snapshot = recorder.query_snapshot();
    assert_eq!(snapshot.queries_total, 19);
    assert_eq!(snapshot.slow_queries_total, 18);
    assert_eq!(snapshot.slow_threshold_ms, 100);
    assert_eq!(snapshot.recent_slow.len(), RECENT_SLOW_QUERIES);
    let newest = snapshot.recent_slow.last().unwrap();
    assert_eq!(newest.seq, 18);
    assert_eq!(newest.model.as_deref(), Some("Model17"));
    assert_eq!(newest.table.as_ref().map(String::len), Some(64));
    assert_eq!(newest.duration_us, 100_000);

    let unlabeled = Recorder::new();
    unlabeled.record_query(QueryLabels::default(), Duration::from_millis(150));
    let slow = &unlabeled.query_snapshot().recent_slow[0];
    assert_eq!(slow.operation, "unknown");
    let json = serde_json::to_value(slow).unwrap();
    assert!(json.get("model").is_none() && json.get("table").is_none());
}

#[test]
fn the_orm_layer_counts_outermost_operations_with_their_static_labels() {
    use tracing_subscriber::layer::SubscriberExt;

    let recorder = Arc::new(Recorder::new());
    let subscriber = tracing_subscriber::registry().with(OrmQueryLayer::local(recorder.clone()));
    tracing::subscriber::with_default(subscriber, || {
        let outer = tracing::info_span!(
            target: "rullst_orm",
            "rullst.orm.query",
            orm.model = "Post",
            orm.table = "posts",
            orm.operation = "select_many"
        );
        outer.in_scope(|| {
            // Eager loading inside the outer operation is part of it.
            let nested = tracing::info_span!(
                target: "rullst_orm",
                "rullst.orm.query",
                orm.operation = "select_many"
            );
            nested.in_scope(|| {});
            std::thread::sleep(Duration::from_millis(110));
        });
        drop(outer);
        // Other spans, including ORM transaction spans, are not queries.
        tracing::info_span!(target: "rullst_orm", "rullst.orm.transaction").in_scope(|| {});
        tracing::info_span!(target: "app", "rullst.orm.query").in_scope(|| {});
        tracing::info_span!(
            target: "rullst_orm",
            "rullst.orm.query",
            orm.operation = "raw.select",
            orm.binding_count = 2_u64
        )
        .in_scope(|| {});
    });

    let snapshot = recorder.query_snapshot();
    assert_eq!(snapshot.queries_total, 2);
    assert_eq!(snapshot.slow_queries_total, 1);
    let slow = &snapshot.recent_slow[0];
    assert_eq!(slow.operation, "select_many");
    assert_eq!(slow.model.as_deref(), Some("Post"));
    assert_eq!(slow.table.as_deref(), Some("posts"));
    assert!(slow.duration_us >= 100_000);
}

#[test]
fn database_state_names_why_queries_are_not_reported() {
    let recorder = Recorder::new();
    assert_eq!(
        database(&recorder, false, true),
        Database::Unavailable {
            reason: "subscriber_not_installed"
        }
    );
    assert_eq!(
        database(&recorder, true, false),
        Database::Unavailable {
            reason: "orm_spans_filtered"
        }
    );
    assert!(matches!(
        database(&recorder, true, true),
        Database::Observed(_)
    ));
    let json = serde_json::to_value(database(&recorder, true, true)).unwrap();
    assert_eq!(json["state"], "observed");
    assert_eq!(json["queries_total"], 0);
}

#[tokio::test]
async fn a_local_poll_returns_the_versioned_secret_free_document() {
    let recorder = Arc::new(Recorder::new());
    recorder.record_request("POST", "/orders", 201, Duration::from_millis(3));
    recorder.record_request("GET", "/boom", 500, Duration::from_millis(1));
    let router = endpoint(recorder, None);

    let response = router.clone().oneshot(local()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    assert_eq!(
        response.headers()[header::X_CONTENT_TYPE_OPTIONS],
        "nosniff"
    );

    let (_, body) = json(&router, local()).await;
    assert_eq!(body["schema"], "rullst.dev-telemetry.v1");
    assert_eq!(body["generation"], MARKER);
    assert_eq!(body["http"]["requests_total"], 2);
    assert_eq!(body["http"]["server_errors_total"], 1);
    assert_eq!(body["http"]["recent"][0]["path"], "/orders");
    assert_eq!(body["http"]["recent"][0]["status"], 201);
    assert_eq!(body["http"]["recent"][0]["duration_us"], 3_000);
    assert_eq!(body["queue"]["state"], "not_configured");
    assert!(matches!(
        body["database"]["state"].as_str(),
        Some("observed" | "unavailable")
    ));
}

#[tokio::test]
async fn only_loopback_peers_with_a_loopback_authority_are_answered() {
    let router = endpoint(Arc::new(Recorder::new()), None);
    for (peer, host, origin) in [
        (Some("127.0.0.1:1"), Some("localhost:3000"), None),
        (Some("127.0.0.1:1"), Some("LOCALHOST"), None),
        (Some("[::1]:1"), Some("[::1]:3000"), None),
        (Some("[::ffff:127.0.0.1]:1"), Some("127.0.0.1:3000"), None),
        (
            Some("127.0.0.1:1"),
            Some("127.0.0.1:3000"),
            Some("http://localhost:3000"),
        ),
        (
            Some("127.0.0.1:1"),
            Some("127.0.0.1:3000"),
            Some("https://127.0.0.1"),
        ),
    ] {
        let status = router
            .clone()
            .oneshot(request(peer, host, origin))
            .await
            .unwrap()
            .status();
        assert_eq!(status, StatusCode::OK, "{peer:?} {host:?} {origin:?}");
    }
    for (peer, host, origin) in [
        (None, Some("127.0.0.1:3000"), None),
        (Some("192.0.2.10:1"), Some("127.0.0.1:3000"), None),
        (Some("[::ffff:192.0.2.10]:1"), Some("127.0.0.1:3000"), None),
        (Some("127.0.0.1:1"), None, None),
        (Some("127.0.0.1:1"), Some("rebind.example:3000"), None),
        (Some("127.0.0.1:1"), Some("127.0.0.1.rebind.example"), None),
        (Some("127.0.0.1:1"), Some("user@127.0.0.1:3000"), None),
        (
            Some("127.0.0.1:1"),
            Some("127.0.0.1:3000"),
            Some("http://rebind.example"),
        ),
        (Some("127.0.0.1:1"), Some("127.0.0.1:3000"), Some("null")),
        (Some("127.0.0.1:1"), Some("127.0.0.1:3000"), Some("file://")),
    ] {
        let response = router
            .clone()
            .oneshot(request(peer, host, origin))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{peer:?} {host:?} {origin:?}"
        );
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert!(body.is_empty());
    }
    let post = HttpRequest::builder()
        .method("POST")
        .uri(PATH)
        .body(Body::empty())
        .unwrap();
    assert!(!router.oneshot(post).await.unwrap().status().is_success());
}

struct CountingDriver(Result<u64, ()>, Duration);

#[async_trait::async_trait]
impl crate::queue::QueueDriver for CountingDriver {
    async fn push(&self, _: &str, _: &str, _: &str) -> Result<(), crate::queue::QueueError> {
        Ok(())
    }
    async fn pop(&self) -> Result<Option<crate::queue::QueuedJob>, crate::queue::QueueError> {
        Ok(None)
    }
    async fn mark_complete(&self, _: &str) -> Result<(), crate::queue::QueueError> {
        Ok(())
    }
    async fn mark_failed(&self, _: &str, _: &str) -> Result<(), crate::queue::QueueError> {
        Ok(())
    }
    async fn pending_count(&self) -> Result<u64, crate::queue::QueueError> {
        tokio::time::sleep(self.1).await;
        self.0.map_err(|()| {
            crate::queue::QueueError::Driver("redis://user:secret@db.internal".into())
        })
    }
}

#[tokio::test]
async fn queue_depth_reports_counts_and_fixed_failure_reasons_only() {
    let queue = |result, delay| {
        Some(Arc::new(Queue::custom(Box::new(CountingDriver(
            result, delay,
        )))))
    };
    let observed = endpoint(Arc::new(Recorder::new()), queue(Ok(7), Duration::ZERO));
    let (_, body) = json(&observed, local()).await;
    assert_eq!(
        body["queue"],
        serde_json::json!({"state": "observed", "pending": 7})
    );

    let failing = endpoint(Arc::new(Recorder::new()), queue(Err(()), Duration::ZERO));
    let (_, body) = json(&failing, local()).await;
    assert_eq!(
        body["queue"],
        serde_json::json!({"state": "unavailable", "reason": "driver_error"})
    );
    assert!(!body.to_string().contains("secret"));

    let slow = endpoint(
        Arc::new(Recorder::new()),
        queue(Ok(1), Duration::from_secs(5)),
    );
    let (_, body) = json(&slow, local()).await;
    assert_eq!(
        body["queue"],
        serde_json::json!({"state": "unavailable", "reason": "timeout"})
    );
}

#[tokio::test]
async fn the_endpoint_exists_only_in_a_supervised_debug_development_process() {
    for (development, generation) in [
        (false, Some(MARKER.to_string())),
        (true, None),
        (true, Some("not-a-generation".to_string())),
    ] {
        let router = mount(Router::new(), development, generation, None);
        assert_eq!(
            router.oneshot(local()).await.unwrap().status(),
            StatusCode::NOT_FOUND
        );
    }
    let router = mount(Router::new(), true, Some(MARKER.into()), None);
    let expected = if cfg!(debug_assertions) {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    };
    assert_eq!(router.oneshot(local()).await.unwrap().status(), expected);
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn the_access_log_feeds_the_recorder_without_query_strings_or_polls() {
    let telemetry = mount(Router::new(), true, Some(MARKER.into()), None);
    let app = Router::new()
        .route("/dash-probe-orders", get(|| async { "ok" }))
        .merge(telemetry)
        .layer(axum::middleware::from_fn(
            crate::server::console::access_log_middleware,
        ));
    let mut probe = HttpRequest::builder()
        .uri("/dash-probe-orders?token=query-secret")
        .body(Body::empty())
        .unwrap();
    probe
        .extensions_mut()
        .insert(ConnectInfo("127.0.0.1:1".parse::<SocketAddr>().unwrap()));
    assert_eq!(
        app.clone().oneshot(probe).await.unwrap().status(),
        StatusCode::OK
    );
    let (status, body) = json(&app, local()).await;
    assert_eq!(status, StatusCode::OK);

    let recent = body["http"]["recent"].as_array().unwrap();
    assert!(
        recent
            .iter()
            .any(|sample| sample["path"] == "/dash-probe-orders")
    );
    assert!(!body.to_string().contains("query-secret"));
    assert!(recent.iter().all(|sample| sample["path"] != PATH));
}
