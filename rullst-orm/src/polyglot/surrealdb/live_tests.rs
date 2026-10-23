//! Live SurrealDB reads, deletes, pages and graph rows against a loopback fake.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashMap;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::*;
use crate::polyglot::{
    CollectionName, DocumentId, DocumentInventory, DocumentPage, DocumentRepository,
};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Event {
    label: String,
}

fn ok(result: Value) -> Json<Value> {
    Json(json!([{ "status": "OK", "result": result }]))
}

fn table_rows(table: &str, count: usize) -> Value {
    Value::Array(
        (1..=count)
            .map(|index| json!({ "id": format!("{table}:evt-{index}"), "label": format!("row-{index}") }))
            .collect(),
    )
}

fn rows(count: usize) -> Value {
    table_rows("events", count)
}

/// `evt-1` exists, `evt-2` does not, `evt-3` matches twice, `evt-4` is a
/// null result, `evt-5` a failed statement that mentions "already exists".
async fn record(Path(key): Path<String>) -> Json<Value> {
    match key.as_str() {
        "\"evt-1\"" => ok(json!([{ "id": "events:evt-1", "label": "one" }])),
        "\"evt-3\"" => ok(rows(2)),
        "\"evt-4\"" => ok(Value::Null),
        "\"evt-5\"" => Json(json!([{ "status": "ERR", "result": "record already exists" }])),
        _ => ok(json!([])),
    }
}

/// Returns `limit` rows of the requested table, or one more valid row for
/// the `overflow` table.
async fn page(Query(query): Query<HashMap<String, String>>) -> Json<Value> {
    let limit: usize = query["limit"].parse().unwrap();
    let extra = usize::from(query["table"] == "overflow");
    ok(table_rows(&query["table"], limit + extra))
}

async fn graph(body: String) -> Json<Value> {
    let count = if body.ends_with("LIMIT 2") { 2 } else { 4 };
    ok(rows(count))
}

async fn exact() -> Response {
    let body = json!([{ "status": "OK", "result": [] }]).to_string();
    (
        StatusCode::OK,
        format!("{body}{}", " ".repeat(1024 - body.len())),
    )
        .into_response()
}

async fn store(limit: Option<usize>) -> (SurrealDbStore<Event>, tokio::task::JoinHandle<()>) {
    let router = Router::new()
        .route("/key/events/{key}", get(record).delete(record))
        .route("/key/exact/{key}", get(exact))
        .route("/sql", post(page))
        .route("/gql", post(graph));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = SurrealConfig::new(endpoint, "main", "app", SurrealAuth::None);
    if let Some(limit) = limit {
        config = config.with_response_limit(limit).unwrap();
    }
    (SurrealDbStore::connect_or_mock(config).unwrap(), server)
}

fn id(value: &str) -> DocumentId {
    DocumentId::new(value).unwrap()
}

#[tokio::test]
async fn finds_and_deletes_report_exactly_what_the_server_returned() {
    let (store, server) = store(None).await;
    let events = CollectionName::new("events").unwrap();
    assert_eq!(
        store.find(&events, &id("evt-1")).await.unwrap(),
        Some(Event {
            label: "one".to_owned()
        })
    );
    assert_eq!(store.find(&events, &id("evt-2")).await.unwrap(), None);
    assert_eq!(store.find(&events, &id("evt-4")).await.unwrap(), None);
    assert!(store.find(&events, &id("evt-3")).await.is_err());
    assert!(matches!(
        store.find(&events, &id("evt-5")).await,
        Err(PolyglotError::Driver { .. })
    ));
    assert!(store.delete(&events, &id("evt-1")).await.unwrap());
    assert!(!store.delete(&events, &id("evt-2")).await.unwrap());
    server.abort();
}

#[tokio::test]
async fn pages_and_graph_rows_stay_within_their_limits() {
    let (store, server) = store(None).await;
    let page = DocumentPage::new(0, 2).unwrap();
    let events = CollectionName::new("events").unwrap();
    let listed = store.list(&events, page).await.unwrap();
    assert_eq!(listed.len(), 2);
    let entries = store.list_entries(&events, page).await.unwrap();
    let ids: Vec<&str> = entries.iter().map(|entry| entry.id().as_str()).collect();
    assert_eq!(ids, ["evt-1", "evt-2"]);
    let overflow = CollectionName::new("overflow").unwrap();
    assert!(store.list(&overflow, page).await.is_err());
    assert!(store.list_entries(&overflow, page).await.is_err());

    let rows = store
        .query_graph(&GraphQuery::read_only("MATCH (n:event) RETURN n", 2).unwrap())
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["label"], "row-1");
    assert!(
        store
            .query_graph(&GraphQuery::read_only("MATCH (n:event) RETURN n", 3).unwrap())
            .await
            .is_err()
    );
    server.abort();
}

#[tokio::test]
async fn a_response_of_exactly_the_limit_is_read() {
    let (store, server) = store(Some(1024)).await;
    let exact = CollectionName::new("exact").unwrap();
    assert_eq!(store.find(&exact, &id("evt-1")).await.unwrap(), None);
    server.abort();
}
