use axum::{
    Json, Router,
    body::Bytes,
    extract::Query,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};

use super::*;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Event {
    label: String,
}

async fn spawn_test_server(router: Router) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (format!("http://{address}"), handle)
}

async fn create_record(headers: HeaderMap, Json(body): Json<Value>) -> Json<Value> {
    assert_eq!(headers.get("surreal-ns").unwrap(), "main");
    assert_eq!(headers.get("surreal-db").unwrap(), "app");
    assert_eq!(body, json!({ "label": "live" }));
    Json(json!([{ "status": "OK", "result": [{
        "id": "events:evt-1", "label": "live"
    }] }]))
}

async fn list_records(
    Query(parameters): Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> Json<Value> {
    assert_eq!(parameters.get("table").map(String::as_str), Some("events"));
    assert_eq!(parameters.get("start").map(String::as_str), Some("2"));
    assert_eq!(parameters.get("limit").map(String::as_str), Some("1"));
    let query = String::from_utf8_lossy(&body);
    assert!(query.contains("type::table($table)"));
    assert!(query.contains("type::int($start)"));
    assert!(query.contains("type::int($limit)"));
    Json(json!([{ "status": "OK", "result": [{
        "id": "events:evt-3", "label": "third"
    }] }]))
}

#[tokio::test]
async fn live_http_contract_sets_scope_and_uses_bounded_sql_page() {
    let router = Router::new()
        .route("/key/events/evt-1", post(create_record))
        .route("/sql", post(list_records));
    let (endpoint, server) = spawn_test_server(router).await;
    let store = SurrealDbStore::<Event>::connect_or_mock(SurrealConfig::new(
        endpoint,
        "main",
        "app",
        SurrealAuth::None,
    ))
    .unwrap();
    let collection = CollectionName::new("events").unwrap();
    let id = DocumentId::new("evt-1").unwrap();
    store
        .create(
            &collection,
            &id,
            &Event {
                label: "live".to_owned(),
            },
        )
        .await
        .unwrap();
    let events = store
        .list(&collection, DocumentPage::new(2, 1).unwrap())
        .await
        .unwrap();
    assert_eq!(
        events,
        vec![Event {
            label: "third".to_owned()
        }]
    );
    server.abort();
}

#[test]
fn configuration_and_graph_queries_fail_closed() {
    let debug = format!("{:?}", SurrealAuth::bearer("very-secret"));
    assert!(!debug.contains("very-secret"));
    assert!(GraphQuery::read_only("DELETE person", 10).is_err());
    assert!(GraphQuery::read_only("MATCH (n:person) RETURN n", 0).is_err());
    let graph = GraphQuery::read_only("MATCH (n:person) RETURN n", 10).unwrap();
    assert_eq!(graph.bounded_query(), "MATCH (n:person) RETURN n LIMIT 10");

    let insecure = SurrealConfig::new("http://database.example", "main", "app", SurrealAuth::None);
    assert!(SurrealDbStore::<Event>::connect_or_mock(insecure).is_err());
    let invalid_mock =
        SurrealConfig::new("mock_local", "invalid namespace", "app", SurrealAuth::None);
    assert!(SurrealDbStore::<Event>::connect_or_mock(invalid_mock).is_err());
}

#[tokio::test]
async fn oversized_responses_are_rejected_before_deserialization() {
    async fn oversized() -> (StatusCode, String) {
        (StatusCode::OK, "x".repeat(2048))
    }
    let (endpoint, server) =
        spawn_test_server(Router::new().route("/key/events/evt-1", post(oversized))).await;
    let config = SurrealConfig::new(endpoint, "main", "app", SurrealAuth::None)
        .with_response_limit(1024)
        .unwrap();
    let store = SurrealDbStore::<Event>::connect_or_mock(config).unwrap();
    let result = store
        .create(
            &CollectionName::new("events").unwrap(),
            &DocumentId::new("evt-1").unwrap(),
            &Event {
                label: "live".to_owned(),
            },
        )
        .await;
    assert!(matches!(
        result,
        Err(PolyglotError::ResponseTooLarge { .. })
    ));
    server.abort();
}
