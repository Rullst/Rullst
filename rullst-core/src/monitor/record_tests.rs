//! What the recorder keeps: route patterns and aggregates, never request
//! data (`TM-STUDIO-2`, `MONITOR-04`, `MONITOR-10`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::test_support::{BASE64_TOKEN, bearer, get, read};
use super::{Installed, MONITOR_PATH, Monitor};
use crate::config::Environment;
use axum::Router;
use axum::http::{HeaderValue, StatusCode};
use axum::routing::get as get_route;
use tower::ServiceExt;
use tracing_subscriber::layer::SubscriberExt;

const CLIENT: &str = "192.0.2.30:40000";

fn installed() -> Installed {
    Installed::prepare(
        Monitor::new().with_token(BASE64_TOKEN),
        Environment::Development,
    )
    .unwrap()
    .unwrap()
}

fn application() -> Router {
    Router::new()
        // rullst-access: public — test fixture without data.
        .route("/users/{id}", get_route(|| async { "user" }))
        .route(
            "/",
            get_route(|| async {
                let mut response = axum::response::Response::new(axum::body::Body::from("home"));
                response
                    .headers_mut()
                    .insert("x-application", HeaderValue::from_static("kept"));
                response
            }),
        )
}

fn monitored(installed: &Installed) -> Router {
    installed.record(installed.mount(application()))
}

async fn document(app: &Router) -> String {
    let authorization = bearer(BASE64_TOKEN);
    let response = app
        .clone()
        .oneshot(get(
            MONITOR_PATH,
            CLIENT,
            &[(authorization.0, authorization.1.as_str())],
        ))
        .await
        .unwrap();
    let (status, body) = read(response).await;
    assert_eq!(status, StatusCode::OK);
    String::from_utf8(body).unwrap()
}

#[tokio::test]
async fn raw_paths_queries_and_client_data_never_reach_the_document() {
    let installed = installed();
    let app = monitored(&installed);
    let request = get(
        "/users/8675309?email=alice.secret%40example.org&token=query-marker",
        "198.51.100.77:41000",
        &[
            ("cookie", "session=cookie-marker-7f3a"),
            ("x-forwarded-for", "203.0.113.99"),
            ("user-agent", "agent-marker/1.0"),
            ("authorization", "Bearer header-marker-value"),
            ("x-user-id", "user-id-marker"),
        ],
    );
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::OK
    );
    let missing = get("/admin/secret-path-marker?x=1", "198.51.100.77:41000", &[]);
    assert_eq!(
        app.clone().oneshot(missing).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );

    let body = document(&app).await;
    assert!(body.contains("\"route\":\"/users/{id}\""), "{body}");
    assert!(body.contains("\"route\":\"(unmatched)\""), "{body}");
    for marker in [
        "8675309",
        "alice",
        "example.org",
        "query-marker",
        "cookie-marker",
        "198.51.100",
        "203.0.113",
        "agent-marker",
        "header-marker",
        "user-id-marker",
        "secret-path-marker",
        "?",
        BASE64_TOKEN,
    ] {
        assert!(!body.contains(marker), "{marker} leaked");
    }
}

#[tokio::test]
async fn the_monitor_does_not_count_its_own_requests() {
    let installed = installed();
    let app = monitored(&installed);
    for _ in 0..3 {
        document(&app).await;
    }
    let home = get("/", CLIENT, &[]);
    assert_eq!(
        app.clone().oneshot(home).await.unwrap().status(),
        StatusCode::OK
    );

    let routes = installed.collector().routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(&*routes[0].route, "/");
    assert_eq!(routes[0].stats.requests_total, 1);
    assert!(!document(&app).await.contains(MONITOR_PATH));
}

#[tokio::test]
async fn the_recorder_leaves_responses_unchanged() {
    let installed = installed();
    let plain = application();
    let recorded = installed.record(application());
    for uri in ["/", "/users/1", "/missing"] {
        let without = plain.clone().oneshot(get(uri, CLIENT, &[])).await.unwrap();
        let with = recorded
            .clone()
            .oneshot(get(uri, CLIENT, &[]))
            .await
            .unwrap();
        assert_eq!(without.headers(), with.headers(), "{uri}");
        assert_eq!(read(without).await, read(with).await, "{uri}");
    }
    assert_eq!(installed.collector().routes().len(), 3);
}

/// Emits one ORM operation span, as `rullst-orm` does.
fn orm_find(model: &str, table: &str) {
    tracing::info_span!(
        target: "rullst_orm",
        "rullst.orm.query",
        orm.model = model,
        orm.table = table,
        orm.operation = "find"
    )
    .in_scope(|| {});
}

#[tokio::test]
async fn orm_operations_of_a_request_are_counted_and_repetitions_found() {
    let subscriber =
        tracing_subscriber::registry().with(crate::server::dev_telemetry::query_layer());
    let _guard = tracing::subscriber::set_default(subscriber);
    let installed = installed();
    let app = installed.record(
        Router::new()
            // rullst-access: public — test fixture without data.
            .route(
                "/posts/{id}",
                get_route(|| async {
                    orm_find("Post", "posts");
                    for _ in 0..3 {
                        orm_find("Comment", "comments");
                    }
                    "post"
                }),
            ),
    );
    let request = get("/posts/42", CLIENT, &[]);
    assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::OK);

    let collector = installed.collector();
    assert_eq!(collector.queries(), 4);
    let (total, findings) = collector.findings();
    assert_eq!(total, 1);
    assert_eq!(findings[0].method, "GET");
    assert_eq!(&*findings[0].route, "/posts/{id}");
    assert_eq!(findings[0].fingerprint, "Comment.find (comments)");
    assert_eq!(findings[0].max_occurrences, 3);
}

#[tokio::test]
async fn orm_operations_outside_a_recorded_request_are_not_counted() {
    let subscriber =
        tracing_subscriber::registry().with(crate::server::dev_telemetry::query_layer());
    let _guard = tracing::subscriber::set_default(subscriber);
    let installed = installed();
    for _ in 0..3 {
        orm_find("Job", "jobs");
    }
    assert_eq!(installed.collector().queries(), 0);
    assert_eq!(installed.collector().findings().0, 0);
}
