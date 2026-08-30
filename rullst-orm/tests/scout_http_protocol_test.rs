#![cfg(feature = "scout-http")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use rullst_orm::{AlgoliaEngine, ElasticsearchEngine, SearchEngine};
use serde_json::{Value, json};

#[derive(Default)]
struct ProtocolState {
    elastic_updates: AtomicUsize,
    elastic_deletes: AtomicUsize,
    algolia_updates: AtomicUsize,
    algolia_deletes: AtomicUsize,
}

#[tokio::test]
async fn elasticsearch_and_algolia_protocols_are_bounded_and_parse_ids() {
    let state = Arc::new(ProtocolState::default());
    let router = Router::new()
        .route(
            "/{index}/_doc/{id}",
            put(elastic_update).delete(elastic_delete),
        )
        .route("/{index}/_search", post(elastic_search))
        .route(
            "/1/indexes/{index}/{id}",
            put(algolia_update).delete(algolia_delete),
        )
        .route("/1/indexes/{index}/query", post(algolia_search))
        .route("/1/indexes/{index}/task/{task_id}", get(algolia_task))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind protocol fixture");
    let address = listener
        .local_addr()
        .expect("read protocol fixture address");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("serve protocol fixture");
    });
    let endpoint = format!("http://{address}");

    let elastic = ElasticsearchEngine::local(&endpoint).expect("local Elasticsearch fixture");
    elastic
        .update("articles", 7, json!({"title": "Elastic document"}))
        .await
        .expect("send Elasticsearch update");
    assert_eq!(
        elastic
            .search("articles", "elastic")
            .await
            .expect("parse Elasticsearch hits"),
        vec![7]
    );
    assert!(elastic.search("badids", "elastic").await.is_err());
    assert!(elastic.search("oversized", "elastic").await.is_err());
    elastic
        .delete("articles", 7)
        .await
        .expect("send Elasticsearch delete");

    let algolia = AlgoliaEngine::with_endpoint(&endpoint, "APP123", "algolia-test-key")
        .expect("local Algolia-compatible fixture");
    algolia
        .update("articles", 8, json!({"title": "Algolia document"}))
        .await
        .expect("send Algolia update and await task");
    assert_eq!(
        algolia
            .search("articles", "algolia")
            .await
            .expect("parse Algolia hits"),
        vec![8]
    );
    algolia
        .delete("articles", 8)
        .await
        .expect("send Algolia delete and await task");

    assert_eq!(state.elastic_updates.load(Ordering::SeqCst), 1);
    assert_eq!(state.elastic_deletes.load(Ordering::SeqCst), 1);
    assert_eq!(state.algolia_updates.load(Ordering::SeqCst), 1);
    assert_eq!(state.algolia_deletes.load(Ordering::SeqCst), 1);
    shutdown_tx.send(()).expect("request fixture shutdown");
    server.await.expect("join protocol fixture");
}

async fn elastic_update(
    State(state): State<Arc<ProtocolState>>,
    Path((index, id)): Path<(String, i32)>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    assert_eq!(index, "articles");
    assert_eq!(id, 7);
    assert_eq!(payload.get("id").and_then(Value::as_i64), Some(7));
    state.elastic_updates.fetch_add(1, Ordering::SeqCst);
    Json(json!({"result": "created"}))
}

async fn elastic_delete(
    State(state): State<Arc<ProtocolState>>,
    Path((index, id)): Path<(String, i32)>,
) -> Json<Value> {
    assert_eq!((index.as_str(), id), ("articles", 7));
    state.elastic_deletes.fetch_add(1, Ordering::SeqCst);
    Json(json!({"result": "deleted"}))
}

async fn elastic_search(Path(index): Path<String>, Json(payload): Json<Value>) -> Json<Value> {
    assert_eq!(payload.get("size").and_then(Value::as_u64), Some(1_000));
    match index.as_str() {
        "articles" => Json(json!({"hits": {"hits": [{"_id": "7"}]}})),
        "badids" => Json(json!({"hits": {"hits": [{"_id": "not-an-id"}]}})),
        "oversized" => Json(json!({"padding": "x".repeat(4 * 1_048_576 + 1)})),
        _ => Json(json!({"hits": {"hits": []}})),
    }
}

async fn algolia_update(
    State(state): State<Arc<ProtocolState>>,
    headers: HeaderMap,
    Path((index, id)): Path<(String, i32)>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    assert_algolia_headers(&headers);
    assert_eq!((index.as_str(), id), ("articles", 8));
    assert_eq!(payload.get("objectID").and_then(Value::as_str), Some("8"));
    state.algolia_updates.fetch_add(1, Ordering::SeqCst);
    Json(json!({"taskID": 9}))
}

async fn algolia_delete(
    State(state): State<Arc<ProtocolState>>,
    headers: HeaderMap,
    Path((index, id)): Path<(String, i32)>,
) -> Json<Value> {
    assert_algolia_headers(&headers);
    assert_eq!((index.as_str(), id), ("articles", 8));
    state.algolia_deletes.fetch_add(1, Ordering::SeqCst);
    Json(json!({"taskID": 9}))
}

async fn algolia_search(
    headers: HeaderMap,
    Path(index): Path<String>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    assert_algolia_headers(&headers);
    assert_eq!(index, "articles");
    assert_eq!(
        payload.get("hitsPerPage").and_then(Value::as_u64),
        Some(1_000)
    );
    Json(json!({"hits": [{"objectID": "8"}]}))
}

async fn algolia_task(
    headers: HeaderMap,
    Path((index, task_id)): Path<(String, u64)>,
) -> Json<Value> {
    assert_algolia_headers(&headers);
    assert_eq!((index.as_str(), task_id), ("articles", 9));
    Json(json!({"status": "published"}))
}

fn assert_algolia_headers(headers: &HeaderMap) {
    assert_eq!(
        headers
            .get("x-algolia-application-id")
            .and_then(|value| value.to_str().ok()),
        Some("APP123")
    );
    assert_eq!(
        headers
            .get("x-algolia-api-key")
            .and_then(|value| value.to_str().ok()),
        Some("algolia-test-key")
    );
}
