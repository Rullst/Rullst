//! Request-correlated ORM repetitions (possible N+1) in development telemetry.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::orm_layer::OrmQueryLayer;
use super::recorder::{RECENT_REPEATED, Recorder};
use super::request_scope::observe;
use super::*;
use crate::query_patterns::{N_PLUS_ONE_THRESHOLD, RepeatedOperation, repeated_operations};
use axum::http::Request as HttpRequest;
use tower::ServiceExt;
use tracing_subscriber::layer::SubscriberExt;

fn orm_operation(model: Option<&str>, table: Option<&str>, operation: &str) {
    match (model, table) {
        (Some(model), Some(table)) => tracing::info_span!(
            target: "rullst_orm",
            "rullst.orm.query",
            orm.model = model,
            orm.table = table,
            orm.operation = operation
        )
        .in_scope(|| {}),
        _ => tracing::info_span!(
            target: "rullst_orm",
            "rullst.orm.query",
            orm.operation = operation
        )
        .in_scope(|| {}),
    }
}

#[tokio::test]
async fn a_request_collects_the_fingerprints_of_its_outermost_operations() {
    let recorder = Arc::new(Recorder::new());
    let subscriber = tracing_subscriber::registry().with(OrmQueryLayer::local(recorder.clone()));
    let _guard = tracing::subscriber::set_default(subscriber);

    let ((), operations) = observe(async {
        for _ in 0..3 {
            orm_operation(Some("Post"), Some("posts"), "find");
        }
        let outer = tracing::info_span!(
            target: "rullst_orm",
            "rullst.orm.query",
            orm.model = "User",
            orm.table = "users",
            orm.operation = "select_many"
        );
        // Eager loading inside an operation is part of that operation.
        outer.in_scope(|| orm_operation(Some("Post"), Some("posts"), "find"));
        drop(outer);
        // Raw statements have no identity beyond "raw.select".
        orm_operation(None, None, "raw.select");
        // Work spawned on another task is not attributed to the request.
        tokio::spawn(async { orm_operation(Some("Post"), Some("posts"), "find") })
            .await
            .unwrap();
    })
    .await;

    assert_eq!(
        operations,
        vec![
            "Post.find (posts)",
            "Post.find (posts)",
            "Post.find (posts)",
            "User.select_many (users)",
        ]
    );
    assert_eq!(
        repeated_operations(&operations, N_PLUS_ONE_THRESHOLD),
        vec![RepeatedOperation {
            fingerprint: "Post.find (posts)".to_string(),
            occurrences: 3,
        }]
    );
    // Query counting itself is unchanged.
    assert_eq!(recorder.query_snapshot().queries_total, 6);
}

#[test]
fn operations_outside_a_request_are_not_attributed() {
    let recorder = Arc::new(Recorder::new());
    let subscriber = tracing_subscriber::registry().with(OrmQueryLayer::local(recorder.clone()));
    tracing::subscriber::with_default(subscriber, || {
        for _ in 0..3 {
            orm_operation(Some("Post"), Some("posts"), "find");
        }
    });
    let snapshot = recorder.query_snapshot();
    assert_eq!(snapshot.queries_total, 3);
    assert_eq!(snapshot.repeated_queries_total, 0);
    assert!(snapshot.recent_repeated.is_empty());
}

#[test]
fn findings_are_bounded_and_serialized_with_the_database_state() {
    let recorder = Recorder::new();
    for index in 0..RECENT_REPEATED + 2 {
        recorder.record_repeated(
            "GET",
            "/posts/{id}\u{1b}",
            vec![RepeatedOperation {
                fingerprint: format!("Post.find{index} (posts)"),
                occurrences: 3 + index,
            }],
        );
    }
    let snapshot = recorder.query_snapshot();
    assert_eq!(snapshot.repeated_threshold, 3);
    assert_eq!(
        snapshot.repeated_queries_total,
        (RECENT_REPEATED + 2) as u64
    );
    assert_eq!(snapshot.recent_repeated.len(), RECENT_REPEATED);
    let newest = snapshot.recent_repeated.last().unwrap();
    assert_eq!(newest.seq, snapshot.repeated_queries_total);
    assert_eq!(newest.route, "/posts/{id}?");

    let value = serde_json::to_value(super::database(&recorder, true, true)).unwrap();
    assert_eq!(value["state"], "observed");
    assert_eq!(value["repeated_threshold"], 3);
    assert_eq!(
        value["recent_repeated"][0]["fingerprint"],
        "Post.find2 (posts)"
    );
    assert_eq!(value["recent_repeated"][0]["method"], "GET");
    let unavailable = serde_json::to_value(super::database(&recorder, false, true)).unwrap();
    assert!(unavailable.get("recent_repeated").is_none());
}

#[cfg(debug_assertions)]
async fn repeating_handler() -> &'static str {
    for _ in 0..4 {
        orm_operation(Some("Comment"), Some("dash_n1_comments"), "find");
    }
    "ok"
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn the_recording_layer_reports_repetitions_by_matched_route() {
    let telemetry = mount(Router::new(), true, Some("0".repeat(32)), None);
    let subscriber = tracing_subscriber::registry().with(orm_layer::debug_layer().unwrap());
    let _guard = tracing::subscriber::set_default(subscriber);
    let app = record_responses(
        Router::new()
            // rullst-access: public — loopback test fixture without data.
            .route("/dash-n1-probe/{id}", get(repeating_handler))
            .merge(telemetry),
        false,
    );
    let mut probe = HttpRequest::builder()
        .uri("/dash-n1-probe/42?secret=1")
        .body(Body::empty())
        .unwrap();
    probe
        .extensions_mut()
        .insert(ConnectInfo("127.0.0.1:1".parse::<SocketAddr>().unwrap()));
    assert_eq!(app.oneshot(probe).await.unwrap().status(), StatusCode::OK);

    let snapshot = recorder::global().unwrap().query_snapshot();
    let finding = snapshot
        .recent_repeated
        .iter()
        .find(|finding| finding.fingerprint == "Comment.find (dash_n1_comments)")
        .unwrap();
    assert_eq!(finding.route, "/dash-n1-probe/{id}");
    assert_eq!(finding.method, "GET");
    assert_eq!(finding.occurrences, 4);
}
