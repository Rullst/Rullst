//! A streamed insert whose server answers with `base_url`, against loopback
//! fakes: the rest of the stream follows a same-origin URL with its baton,
//! and another origin is refused before the token is sent there.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::post};
use reqwest::Url;
use serde_json::{Value, json};

use super::HranaClient;
use super::endpoint::{continued_pipeline_url, pipeline_url};
use crate::polyglot::{PolyglotError, TursoStatement, TursoValue};

/// One request a fake received: path, baton, `Authorization` and the first
/// statement (`batch` for the opening request).
type Seen = (String, Option<String>, Option<String>, String);
type Log = Arc<Mutex<Vec<Seen>>>;

fn record(log: &Log, uri: &Uri, headers: &HeaderMap, request: &Value) {
    let sql = request
        .pointer("/requests/0/stmt/sql")
        .and_then(Value::as_str)
        .unwrap_or("batch");
    log.lock().unwrap().push((
        uri.path().to_owned(),
        request["baton"].as_str().map(str::to_owned),
        headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        sql.to_owned(),
    ));
}

fn ok(response: Value) -> Value {
    json!({ "type": "ok", "response": response })
}

/// Opens a stream whose `base_url` is the insert's bound text (`none` sends
/// none) and answers its key as NULL when that text names `rollback`;
/// finishes it on any path, keeping it open under `/moved-open`.
async fn home_pipeline(
    State(log): State<Log>,
    uri: Uri,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Response {
    record(&log, &uri, &headers, &request);
    if request["baton"].is_string() {
        let baton = uri
            .path()
            .starts_with("/moved-open/")
            .then_some("still-open");
        return Json(json!({
            "baton": baton,
            "base_url": null,
            "results": [
                ok(json!({ "type": "execute", "result": { "affected_row_count": 0 } })),
                ok(json!({ "type": "close" })),
            ]
        }))
        .into_response();
    }
    let base_url = request
        .pointer("/requests/0/batch/steps/1/stmt/args/0/value")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let key = if base_url.contains("rollback") {
        json!({ "type": "null" })
    } else {
        json!({ "type": "text", "value": "generated-key" })
    };
    Json(json!({
        "baton": "stream-1",
        "base_url": (base_url != "none").then_some(base_url),
        "results": [ok(json!({
            "type": "batch",
            "result": {
                "step_results": [
                    { "affected_row_count": 0 },
                    { "cols": [{ "name": "id" }], "rows": [[key]] }
                ],
                "step_errors": [null, null]
            }
        }))]
    }))
    .into_response()
}

async fn any_request(State(log): State<Log>, uri: Uri, headers: HeaderMap) -> Response {
    record(&log, &uri, &headers, &json!({}));
    axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

struct Fakes {
    client: HranaClient,
    home: String,
    home_log: Log,
    elsewhere: String,
    elsewhere_log: Log,
    servers: [tokio::task::JoinHandle<()>; 2],
}

async fn fakes() -> Fakes {
    let home_log = Log::default();
    let elsewhere_log = Log::default();
    let home_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let other_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let home = format!("http://{}", home_listener.local_addr().unwrap());
    let elsewhere_url = format!("http://{}", other_listener.local_addr().unwrap());
    let mut home_router = Router::new();
    for path in ["", "/moved", "/rollback", "/moved-open"] {
        home_router = home_router.route(&format!("{path}/v3/pipeline"), post(home_pipeline));
    }
    let home_router = home_router.with_state(home_log.clone());
    let other_router = Router::new()
        .fallback(any_request)
        .with_state(elsewhere_log.clone());
    let servers = [
        tokio::spawn(async move { axum::serve(home_listener, home_router).await.unwrap() }),
        tokio::spawn(async move { axum::serve(other_listener, other_router).await.unwrap() }),
    ];
    Fakes {
        client: HranaClient::new(&home, "token".to_owned()).unwrap(),
        home,
        home_log,
        elsewhere: elsewhere_url,
        elsewhere_log,
        servers,
    }
}

impl Fakes {
    async fn insert(&self, base_url: &str) -> Result<Option<TursoValue>, PolyglotError> {
        let insert = TursoStatement::new(
            r#"INSERT INTO "rows" ("name") VALUES (?1) RETURNING "id""#,
            vec![TursoValue::Text(base_url.to_owned())],
        )
        .unwrap();
        self.client.insert_returning_key(insert, "id").await
    }

    fn taken(&self) -> Vec<Seen> {
        std::mem::take(&mut *self.home_log.lock().unwrap())
    }

    fn stop(self) {
        assert!(self.elsewhere_log.lock().unwrap().is_empty());
        for server in self.servers {
            server.abort();
        }
    }
}

fn seen(path: &str, baton: Option<&str>, sql: &str) -> Seen {
    (
        path.to_owned(),
        baton.map(str::to_owned),
        Some("Bearer token".to_owned()),
        sql.to_owned(),
    )
}

#[tokio::test]
async fn a_same_origin_base_url_carries_the_rest_of_the_stream() {
    let fakes = fakes().await;
    let opened = seen("/v3/pipeline", None, "batch");
    for (base_url, path) in [
        (format!("{}/moved", fakes.home), "/moved/v3/pipeline"),
        (format!("{}/moved/", fakes.home), "/moved/v3/pipeline"),
        (fakes.home.clone(), "/v3/pipeline"),
        ("none".to_owned(), "/v3/pipeline"),
    ] {
        assert_eq!(
            fakes.insert(&base_url).await.unwrap(),
            Some(TursoValue::Text("generated-key".to_owned())),
            "{base_url}"
        );
        assert_eq!(
            fakes.taken(),
            [opened.clone(), seen(path, Some("stream-1"), "COMMIT")],
            "{base_url}"
        );
    }

    let rollback = format!("{}/rollback", fakes.home);
    assert_eq!(fakes.insert(&rollback).await.unwrap(), None);
    assert_eq!(
        fakes.taken(),
        [
            opened.clone(),
            seen("/rollback/v3/pipeline", Some("stream-1"), "ROLLBACK")
        ]
    );

    // The moved stream must still be closed by its last request.
    let kept_open = format!("{}/moved-open", fakes.home);
    match fakes.insert(&kept_open).await {
        Err(PolyglotError::Driver { message, .. }) => assert_eq!(
            message,
            "invalid Hrana response: server kept a stream open after close"
        ),
        other => panic!("unexpected result: {other:?}"),
    }
    assert_eq!(
        fakes.taken(),
        [
            opened,
            seen("/moved-open/v3/pipeline", Some("stream-1"), "COMMIT")
        ]
    );
    fakes.stop();
}

#[tokio::test]
async fn another_origin_is_refused_before_the_token_is_sent() {
    let fakes = fakes().await;
    let port = fakes.home.rsplit(':').next().unwrap().to_owned();
    for base_url in [
        format!("{}/moved", fakes.elsewhere),
        format!("http://localhost:{port}/moved"),
        format!("https://127.0.0.1:{port}/moved"),
        format!("libsql://127.0.0.1:{port}"),
    ] {
        assert!(
            matches!(
                fakes.insert(&base_url).await,
                Err(PolyglotError::CrossOriginEndpoint { backend: "Turso" })
            ),
            "{base_url}"
        );
        assert_eq!(
            fakes.taken(),
            [seen("/v3/pipeline", None, "batch")],
            "{base_url}"
        );
    }
    // `stop` checks that the other origin received nothing.
    fakes.stop();
}

#[tokio::test]
async fn malformed_base_urls_fail_closed() {
    let fakes = fakes().await;
    let authority = fakes.home.trim_start_matches("http://").to_owned();
    let parts = "base_url must not contain credentials, query parameters, or a fragment";
    for (base_url, reason) in [
        ("moved".to_owned(), "base_url is not an absolute URL"),
        (format!("http://user:secret@{authority}/moved"), parts),
        (format!("http://user@{authority}/moved"), parts),
        (format!("http://:secret@{authority}/moved"), parts),
        (format!("{}/moved?instance=2", fakes.home), parts),
        (format!("{}/moved#instance", fakes.home), parts),
    ] {
        match fakes.insert(&base_url).await {
            Err(PolyglotError::Driver { message, .. }) => {
                assert_eq!(message, format!("invalid Hrana response: {reason}"));
            }
            other => panic!("{base_url}: {other:?}"),
        }
        assert_eq!(
            fakes.taken(),
            [seen("/v3/pipeline", None, "batch")],
            "{base_url}"
        );
    }
    fakes.stop();
}

#[test]
fn an_origin_matches_with_its_default_port_and_case_folded_host() {
    let configured = pipeline_url("libsql://db-org.turso.io").unwrap();
    assert_eq!(
        continued_pipeline_url(&configured, "https://DB-org.turso.io:443/instance-2")
            .unwrap()
            .as_str(),
        "https://db-org.turso.io/instance-2/v3/pipeline"
    );
    for other in [
        "https://db-org.turso.io:8443/",
        "http://db-org.turso.io/",
        "https://other-org.turso.io/",
    ] {
        assert!(
            matches!(
                continued_pipeline_url(&configured, other),
                Err(PolyglotError::CrossOriginEndpoint { .. })
            ),
            "{other}"
        );
    }
    let loopback = Url::parse("http://127.0.0.1:8080/v3/pipeline").unwrap();
    assert!(continued_pipeline_url(&loopback, "http://127.0.0.1:8080").is_ok());
}
