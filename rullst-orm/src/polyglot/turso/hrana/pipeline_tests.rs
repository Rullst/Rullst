//! The Hrana pipeline client against a loopback fake server.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::post};
use serde_json::{Value, json};

use super::HranaClient;
use crate::polyglot::{PolyglotError, TursoQueryLimit, TursoStatement, TursoValue};

fn ok(response: Value) -> Value {
    json!({ "type": "ok", "response": response })
}

fn close() -> Value {
    ok(json!({ "type": "close" }))
}

fn execute_result(result: Value) -> Value {
    ok(json!({ "type": "execute", "result": result }))
}

fn pipeline(results: Vec<Value>) -> Response {
    Json(json!({ "baton": null, "results": results })).into_response()
}

/// Answers by the first statement's SQL, so one server covers every case.
async fn hrana(headers: HeaderMap, Json(request): Json<Value>) -> Response {
    let authorized = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        == Some("Bearer token");
    let sql = request
        .pointer("/requests/0/stmt/sql")
        .and_then(Value::as_str)
        .unwrap_or("BATCH");
    match sql {
        "WHO" => pipeline(vec![
            execute_result(json!({ "affected_row_count": u64::from(authorized) })),
            close(),
        ]),
        "UPDATE" => pipeline(vec![
            execute_result(json!({ "affected_row_count": 3 })),
            close(),
        ]),
        "SELECT" => pipeline(vec![
            execute_result(json!({
                "cols": [{ "name": "n" }],
                "rows": [[{ "type": "integer", "value": "7" }]]
            })),
            close(),
        ]),
        "STATUS" => StatusCode::BAD_GATEWAY.into_response(),
        "HUGE" => {
            let body = format!("\"{}\"", "x".repeat(16 * 1024 * 1024 + 1));
            ([(header::CONTENT_TYPE, "application/json")], body).into_response()
        }
        "EXACT" => {
            let results = json!({
                "baton": null,
                "results": [execute_result(json!({ "affected_row_count": 4 })), close()]
            })
            .to_string();
            let padded = format!("{results}{}", " ".repeat(16 * 1024 * 1024 - results.len()));
            ([(header::CONTENT_TYPE, "application/json")], padded).into_response()
        }
        "BATON" => Json(json!({ "baton": "open", "results": [] })).into_response(),
        "EXTRA" => pipeline(vec![
            execute_result(json!({ "affected_row_count": 1 })),
            close(),
            close(),
        ]),
        "NOCLOSE" => pipeline(vec![
            execute_result(json!({ "affected_row_count": 1 })),
            execute_result(json!({ "affected_row_count": 1 })),
        ]),
        "WRONG" => pipeline(vec![close(), close()]),
        "CODED" => pipeline(vec![
            json!({ "type": "error", "error": { "message": "boom", "code": "SQLITE_ERROR" } }),
        ]),
        "UNCODED" => pipeline(vec![
            json!({ "type": "error", "error": { "message": "boom", "code": "" } }),
        ]),
        _ => pipeline(vec![
            ok(json!({
                "type": "batch",
                "result": {
                    "step_results": [
                        { "affected_row_count": 0 },
                        { "affected_row_count": 2 },
                        { "affected_row_count": 5 },
                        { "affected_row_count": 0 },
                        null
                    ],
                    "step_errors": [null, null, null, null, null]
                }
            })),
            close(),
        ]),
    }
}

async fn client(token: &str) -> (HranaClient, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = Router::new().route("/v3/pipeline", post(hrana));
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (
        HranaClient::new(&endpoint, token.to_owned()).unwrap(),
        server,
    )
}

fn statement(sql: &str) -> TursoStatement {
    TursoStatement::new(sql, vec![]).unwrap()
}

fn driver_message(error: PolyglotError) -> String {
    match error {
        PolyglotError::Driver { message, .. } => message,
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn pipelines_return_counts_rows_and_batch_results() {
    let (authenticated, server) = client("token").await;
    assert_eq!(authenticated.execute(statement("WHO")).await.unwrap(), 1);
    assert_eq!(authenticated.execute(statement("UPDATE")).await.unwrap(), 3);
    // A response of exactly 16 MiB is read.
    assert_eq!(authenticated.execute(statement("EXACT")).await.unwrap(), 4);
    let rows = authenticated
        .query(statement("SELECT"), TursoQueryLimit::new(10).unwrap())
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("n"), Some(&TursoValue::Integer(7)));
    assert_eq!(
        authenticated
            .transaction(vec![statement("A"), statement("B")])
            .await
            .unwrap(),
        [2, 5]
    );
    // More statements than the batch returned results for.
    let six = ["A", "B", "C", "D", "E", "F"].map(statement).to_vec();
    assert!(authenticated.transaction(six).await.is_err());
    server.abort();

    let (anonymous, server) = client("").await;
    assert_eq!(anonymous.execute(statement("WHO")).await.unwrap(), 0);
    server.abort();
}

#[tokio::test]
async fn malformed_pipelines_fail_closed() {
    let (client, server) = client("token").await;
    for sql in ["STATUS", "BATON", "EXTRA", "NOCLOSE", "WRONG"] {
        assert!(client.execute(statement(sql)).await.is_err(), "{sql}");
    }
    assert!(matches!(
        client.execute(statement("HUGE")).await,
        Err(PolyglotError::ResponseTooLarge { .. })
    ));
    assert_eq!(
        driver_message(client.execute(statement("CODED")).await.unwrap_err()),
        "Hrana server error SQLITE_ERROR: boom"
    );
    assert_eq!(
        driver_message(client.execute(statement("UNCODED")).await.unwrap_err()),
        "Hrana server error: boom"
    );
    server.abort();
}
