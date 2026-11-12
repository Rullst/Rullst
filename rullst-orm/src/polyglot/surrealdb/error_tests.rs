//! SurrealDB failures keep the server's bounded, redacted reason.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::*;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Event {
    label: String,
}

// A third-party service's route, not an application route (see live_tests).
const ANY_KEY: &str = "/key/{table}/{key}";

const ECHO: &str = "rejected s3cret-pass and s3cret-token from https://user:pw@db.example/sql";

/// Answers a record read by its table name.
async fn record(Path((table, _key)): Path<(String, String)>) -> Response {
    match table.as_str() {
        "undefined" => Json(json!([{
            "status": "ERR",
            "result": "The database 'app'\n does not exist"
        }]))
        .into_response(),
        "opaque" => {
            Json(json!([{ "status": "ERR", "result": { "kind": "Internal" } }])).into_response()
        }
        "plain" => (StatusCode::UNAUTHORIZED, "The password did not verify").into_response(),
        "echo" => (StatusCode::UNAUTHORIZED, ECHO).into_response(),
        "parse" => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "code": 400,
                "details": "Request problems detected",
                "information": "Parse error: Unexpected token"
            })),
        )
            .into_response(),
        "uninformative" => (StatusCode::FORBIDDEN, Json(json!({ "code": 403 }))).into_response(),
        "huge" => (StatusCode::INTERNAL_SERVER_ERROR, "x".repeat(2048)).into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Answers a replace by the record id it targets.
async fn rpc(Json(request): Json<Value>) -> Json<Value> {
    match request.pointer("/params/1/id").and_then(Value::as_str) {
        Some("parse") => Json(json!({
            "id": 1,
            "error": { "code": -32000, "message": "Parse error: Unexpected token `x`" }
        })),
        Some("bare") => Json(json!({ "id": 1, "error": { "code": -32000 } })),
        _ => Json(json!({ "id": 1 })),
    }
}

async fn server() -> (String, tokio::task::JoinHandle<()>) {
    let router = Router::new()
        .route(ANY_KEY, get(record))
        .route("/rpc", post(rpc));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (endpoint, server)
}

fn store(endpoint: &str, auth: SurrealAuth) -> SurrealDbStore<Event> {
    let config = SurrealConfig::new(endpoint, "main", "app", auth)
        .with_response_limit(1024)
        .unwrap();
    SurrealDbStore::connect_or_mock(config).unwrap()
}

async fn find_failure(store: &SurrealDbStore<Event>, table: &str) -> String {
    let collection = CollectionName::new(table).unwrap();
    match store
        .find(&collection, &DocumentId::new("evt-1").unwrap())
        .await
    {
        Err(PolyglotError::Driver { backend, message }) => {
            assert_eq!(backend, "SurrealDB");
            message
        }
        other => panic!("{table}: {other:?}"),
    }
}

async fn replace_failure(store: &SurrealDbStore<Event>, id: &str) -> String {
    let event = Event {
        label: "x".to_owned(),
    };
    match store
        .replace(
            &CollectionName::new("events").unwrap(),
            &DocumentId::new(id).unwrap(),
            &event,
        )
        .await
    {
        Err(PolyglotError::Driver { message, .. }) => message,
        other => panic!("{id}: {other:?}"),
    }
}

#[tokio::test]
async fn failed_statements_and_requests_keep_the_server_reason() {
    let (endpoint, server) = server().await;
    let store = store(&endpoint, SurrealAuth::None);
    for (table, expected) in [
        (
            "undefined",
            "statement returned status ERR: The database 'app' does not exist",
        ),
        ("opaque", "statement returned status ERR"),
        (
            "plain",
            "HTTP status 401 Unauthorized: The password did not verify",
        ),
        (
            "parse",
            "HTTP status 400 Bad Request: Parse error: Unexpected token",
        ),
        ("uninformative", "HTTP status 403 Forbidden: {\"code\":403}"),
        ("missing", "HTTP status 404 Not Found"),
        // A body over the response bound is not read for a reason.
        ("huge", "HTTP status 500 Internal Server Error"),
    ] {
        assert_eq!(find_failure(&store, table).await, expected, "{table}");
    }
    assert_eq!(
        replace_failure(&store, "parse").await,
        "RPC query returned an error: Parse error: Unexpected token `x`"
    );
    assert_eq!(
        replace_failure(&store, "bare").await,
        "RPC query returned an error"
    );
    assert_eq!(
        replace_failure(&store, "none").await,
        "RPC query returned an error"
    );
    server.abort();
}

#[tokio::test]
async fn an_echoed_password_token_or_url_never_reaches_the_error() {
    let (endpoint, server) = server().await;
    for (auth, expected) in [
        (
            SurrealAuth::None,
            "rejected s3cret-pass and s3cret-token from [redacted URL]",
        ),
        (
            SurrealAuth::basic("root", "s3cret-pass"),
            "rejected [redacted] and s3cret-token from [redacted URL]",
        ),
        (
            SurrealAuth::bearer("s3cret-token"),
            "rejected s3cret-pass and [redacted] from [redacted URL]",
        ),
    ] {
        assert_eq!(
            find_failure(&store(&endpoint, auth), "echo").await,
            format!("HTTP status 401 Unauthorized: {expected}")
        );
    }
    server.abort();
}
