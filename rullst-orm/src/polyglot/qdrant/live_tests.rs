//! Live Qdrant requests and responses against a loopback fake server.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{post, put};
use axum::{Json, Router};
use serde_json::{Map, Value, json};

use super::{
    QdrantConfig, QdrantStore, VectorCollectionName, VectorPoint, VectorQueryLimit,
    VectorRepository,
};

fn points(count: u64) -> Value {
    Value::Array(
        (1..=count)
            .map(|id| json!({ "id": id, "score": 0.5, "payload": { "n": id } }))
            .collect(),
    )
}

async fn query(Path(collection): Path<String>) -> Response {
    match collection.as_str() {
        "within" => {
            Json(json!({ "status": "ok", "result": { "points": points(2) } })).into_response()
        }
        "over" => {
            Json(json!({ "status": "ok", "result": { "points": points(3) } })).into_response()
        }
        _ => {
            let body = json!({ "status": "ok", "result": { "points": [] } }).to_string();
            let padded = format!("{body}{}", " ".repeat(1024 - body.len()));
            (StatusCode::OK, padded).into_response()
        }
    }
}

async fn upsert() -> Json<Value> {
    Json(json!({ "status": "ok", "result": { "status": "completed" } }))
}

// Paths of the Qdrant HTTP API this test server imitates. They are a
// third-party service's routes, not application routes, so the IDOR audit's
// access classification does not apply to them.
const QUERY_POINTS: &str = "/collections/{collection}/points/query";
const UPSERT_POINTS: &str = "/collections/{collection}/points";

async fn store(limit: usize) -> (QdrantStore, tokio::task::JoinHandle<()>) {
    let router = Router::new()
        .route(QUERY_POINTS, post(query))
        .route(UPSERT_POINTS, put(upsert));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let config = QdrantConfig::unauthenticated_local(endpoint)
        .with_response_limit(limit)
        .unwrap();
    (QdrantStore::connect_or_mock(config).unwrap(), server)
}

fn collection(name: &str) -> VectorCollectionName {
    VectorCollectionName::new(name).unwrap()
}

#[tokio::test]
async fn searches_stay_within_the_requested_limit() {
    let (store, server) = store(1024 * 1024).await;
    let limit = VectorQueryLimit::new(2).unwrap();
    let matches = store
        .search(&collection("within"), &[1.0], limit)
        .await
        .unwrap();
    let ids: Vec<u64> = matches.iter().map(|hit| hit.id()).collect();
    assert_eq!(ids, [1, 2]);
    assert!(
        store
            .search(&collection("over"), &[1.0], limit)
            .await
            .is_err()
    );
    server.abort();
}

#[tokio::test]
async fn a_response_of_exactly_the_limit_is_read() {
    let (store, server) = store(1024).await;
    let limit = VectorQueryLimit::new(2).unwrap();
    assert!(
        store
            .search(&collection("exact"), &[1.0], limit)
            .await
            .unwrap()
            .is_empty()
    );
    server.abort();
}

#[tokio::test]
async fn a_point_with_a_mebibyte_payload_is_sent() {
    let (store, server) = store(1024 * 1024).await;
    let Value::Object(payload) = json!({ "p": "x".repeat(1024 * 1024 - 8) }) else {
        unreachable!("object literal");
    };
    let payload: Map<String, Value> = payload;
    let point = VectorPoint::new(1, vec![1.0], payload).unwrap();
    store
        .upsert(&collection("large"), point)
        .await
        .expect("a 1 MiB payload fits the request bound");
    server.abort();
}
