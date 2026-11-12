//! Typed inserts against a loopback Hrana fake: an omitted integer key is one
//! plain statement; an omitted text key runs on a stream and is committed
//! only when the database returned a key.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::post};
use serde_json::{Value, json};

use crate::polyglot::{
    PolyglotError, TursoCodec, TursoConfig, TursoModel, TursoPrimaryKey, TursoRow, TursoStore,
    TursoValue,
};

/// A two-column model whose key type is chosen by the test.
#[derive(Debug)]
struct Row<Key> {
    id: Key,
    name: String,
}

impl<Key> TursoModel for Row<Key>
where
    Key: TursoPrimaryKey + Send + Sync + 'static,
{
    fn table_name() -> &'static str {
        "rows"
    }

    fn columns() -> &'static [&'static str] {
        &["id", "name"]
    }

    fn primary_key_column() -> &'static str {
        "id"
    }

    fn encode_turso(&self) -> Result<Vec<TursoValue>, PolyglotError> {
        Ok(vec![self.id.encode_turso()?, self.name.encode_turso()?])
    }

    fn decode_turso(row: &TursoRow) -> Result<Self, PolyglotError> {
        Ok(Self {
            id: Key::decode_turso(row.get("id").unwrap_or(&TursoValue::Null))?,
            name: String::decode_turso(row.get("name").unwrap_or(&TursoValue::Null))?,
        })
    }

    fn primary_key_value(&self) -> Result<TursoValue, PolyglotError> {
        self.id.encode_turso()
    }

    fn primary_key_is_unset(&self) -> bool {
        self.id.is_unset()
    }

    fn assign_primary_key(&mut self, value: &TursoValue) -> Result<(), PolyglotError> {
        self.id.assign_turso(value)
    }
}

type Events = Arc<Mutex<Vec<String>>>;

fn pipeline(baton: Option<&str>, results: Vec<Value>) -> Response {
    Json(json!({ "baton": baton, "base_url": null, "results": results })).into_response()
}

fn ok(response: Value) -> Value {
    json!({ "type": "ok", "response": response })
}

fn close() -> Value {
    ok(json!({ "type": "close" }))
}

fn keyed(key: Value) -> Value {
    json!({ "cols": [{ "name": "id" }], "rows": [[key]] })
}

/// A plain insert returns key 7 (or the text it was given); a streamed one
/// is answered by the scenario named in its bound `name`.
async fn hrana(State(events): State<Events>, Json(request): Json<Value>) -> Response {
    let first = &request["requests"][0];
    if let Some(baton) = request["baton"].as_str() {
        let sql = first["stmt"]["sql"].as_str().unwrap_or_default().to_owned();
        events.lock().unwrap().push(sql.clone());
        return match (baton, sql.as_str()) {
            ("uncommitted", "COMMIT") => pipeline(
                None,
                vec![
                    json!({ "type": "error", "error": { "message": "disk I/O error", "code": "SQLITE_IOERR" } }),
                    close(),
                ],
            ),
            ("unclosed", _) => pipeline(Some("still-open"), vec![]),
            _ => pipeline(
                None,
                vec![
                    ok(json!({ "type": "execute", "result": { "affected_row_count": 0 } })),
                    close(),
                ],
            ),
        };
    }
    if first["type"] == "execute" {
        events.lock().unwrap().push("execute".to_owned());
        let sql = first["stmt"]["sql"].as_str().unwrap_or_default();
        let key = if sql.contains(r#"("id", "name")"#) {
            first["stmt"]["args"][0].clone()
        } else {
            json!({ "type": "integer", "value": "7" })
        };
        return pipeline(
            None,
            vec![
                ok(json!({ "type": "execute", "result": keyed(key) })),
                close(),
            ],
        );
    }
    events.lock().unwrap().push("batch".to_owned());
    let steps = &first["batch"]["steps"];
    assert_eq!(steps[0]["stmt"]["sql"], "BEGIN");
    assert_eq!(steps[1]["condition"], json!({ "type": "ok", "step": 0 }));
    assert_eq!(steps[1]["stmt"]["want_rows"], true);
    let scenario = steps[1]["stmt"]["args"][0]["value"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let inserted = match scenario.as_str() {
        "null" => keyed(json!({ "type": "null" })),
        "empty" => json!({ "cols": [{ "name": "id" }], "rows": [] }),
        _ => keyed(json!({ "type": "text", "value": "generated-key" })),
    };
    let (results, errors) = if scenario == "duplicate" {
        (
            json!([{ "affected_row_count": 0 }, null]),
            json!([null, {
                "message": "SQLite error: UNIQUE constraint failed: rows.name",
                "code": "SQLITE_CONSTRAINT"
            }]),
        )
    } else {
        (
            json!([{ "affected_row_count": 0 }, inserted]),
            json!([null, null]),
        )
    };
    let baton = (scenario != "closed").then_some(scenario.as_str());
    pipeline(
        baton,
        vec![ok(json!({
            "type": "batch",
            "result": { "step_results": results, "step_errors": errors }
        }))],
    )
}

async fn store() -> (TursoStore, Events, tokio::task::JoinHandle<()>) {
    let events = Events::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = Router::new()
        .route("/v3/pipeline", post(hrana))
        .with_state(events.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let store = TursoStore::connect(TursoConfig::new(endpoint, "").allow_insecure_loopback())
        .await
        .unwrap();
    (store, events, server)
}

fn taken(events: &Events) -> Vec<String> {
    std::mem::take(&mut *events.lock().unwrap())
}

fn unset(name: &str) -> Row<String> {
    Row {
        id: String::new(),
        name: name.to_owned(),
    }
}

#[tokio::test]
async fn integer_and_assigned_keys_keep_the_single_statement_insert() {
    let (store, events, server) = store().await;
    let mut counter = Row {
        id: 0_i64,
        name: "counter".to_owned(),
    };
    store.models::<Row<i64>>().save(&mut counter).await.unwrap();
    assert_eq!(counter.id, 7);
    assert_eq!(taken(&events), ["execute"]);

    let mut assigned = Row {
        id: "given".to_owned(),
        name: "assigned".to_owned(),
    };
    store
        .models::<Row<String>>()
        .create(&mut assigned)
        .await
        .unwrap();
    assert_eq!(assigned.id, "given");
    assert_eq!(taken(&events), ["execute"]);
    server.abort();
}

#[tokio::test]
async fn a_defaulted_text_key_is_committed_only_when_returned() {
    let (store, events, server) = store().await;
    let rows = store.models::<Row<String>>();

    let mut generated = unset("generated");
    rows.save(&mut generated).await.unwrap();
    assert_eq!(generated.id, "generated-key");
    assert_eq!(taken(&events), ["batch", "COMMIT"]);

    for scenario in ["null", "empty"] {
        let mut row = unset(scenario);
        match rows.save(&mut row).await {
            Err(PolyglotError::InvalidIdentifier { kind, reason }) => {
                assert_eq!(kind, "Turso model primary key");
                assert_eq!(
                    reason,
                    "the database generated no value for this key; assign it before saving or give its column a DEFAULT"
                );
            }
            other => panic!("{scenario}: {other:?}"),
        }
        assert_eq!(row.id, "", "{scenario}");
        assert_eq!(taken(&events), ["batch", "ROLLBACK"], "{scenario}");
    }

    let mut duplicate = unset("duplicate");
    assert!(matches!(
        rows.save(&mut duplicate).await,
        Err(PolyglotError::Conflict)
    ));
    assert_eq!(taken(&events), ["batch", "ROLLBACK"]);
    server.abort();
}

#[tokio::test]
async fn a_streamed_insert_fails_closed() {
    let (store, events, server) = store().await;
    let rows = store.models::<Row<String>>();
    let message = |error: PolyglotError| match error {
        PolyglotError::Driver { message, .. } => message,
        other => panic!("unexpected error: {other:?}"),
    };

    let mut closed = unset("closed");
    assert_eq!(
        message(rows.save(&mut closed).await.unwrap_err()),
        "invalid Hrana response: server closed the insert's stream"
    );
    assert_eq!(taken(&events), ["batch"]);

    let mut uncommitted = unset("uncommitted");
    assert_eq!(
        message(rows.save(&mut uncommitted).await.unwrap_err()),
        "Hrana server error SQLITE_IOERR: disk I/O error"
    );
    assert_eq!(uncommitted.id, "");
    assert_eq!(taken(&events), ["batch", "COMMIT"]);

    let mut unclosed = unset("unclosed");
    assert_eq!(
        message(rows.save(&mut unclosed).await.unwrap_err()),
        "invalid Hrana response: server kept a stream open after close"
    );
    assert_eq!(taken(&events), ["batch", "COMMIT"]);
    server.abort();
}
