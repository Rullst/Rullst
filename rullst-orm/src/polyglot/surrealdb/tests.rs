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

#[test]
fn canonical_surreal_record_keys_preserve_portable_ids() {
    assert_eq!(
        decode_surreal_record_key("event_01").unwrap().as_str(),
        "event_01"
    );
    assert_eq!(
        decode_surreal_record_key("⟨event-01⟩").unwrap().as_str(),
        "event-01"
    );
    assert_eq!(
        decode_surreal_record_key("`event-02`").unwrap().as_str(),
        "event-02"
    );
    assert!(decode_surreal_record_key("⟨../secret⟩").is_err());
    assert!(decode_surreal_record_key("⟨event-01").is_err());
}

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
        .route("/key/events/%22evt-1%22", post(create_record))
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
    let ipv6_loopback = SurrealConfig::new("http://[::1]:8000", "main", "app", SurrealAuth::None);
    assert!(SurrealDbStore::<Event>::connect_or_mock(ipv6_loopback).is_ok());
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
        spawn_test_server(Router::new().route("/key/events/%22evt-1%22", post(oversized))).await;
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

async fn update_missing_record(Json(body): Json<Value>) -> Json<Value> {
    assert_eq!(body["method"], "query");
    assert!(
        body["params"][0]
            .as_str()
            .is_some_and(|statement| statement.starts_with("UPDATE type::record($table, $id)"))
    );
    assert_eq!(
        body["params"][1],
        json!({ "table": "events", "id": "evt-9", "data": { "label": "edited" } })
    );
    Json(json!({ "id": 1, "result": [{ "status": "OK", "result": [] }] }))
}

#[tokio::test]
async fn replace_updates_without_creating_a_missing_document() {
    let (endpoint, server) =
        spawn_test_server(Router::new().route("/rpc", post(update_missing_record))).await;
    let store = SurrealDbStore::<Event>::connect_or_mock(SurrealConfig::new(
        endpoint,
        "main",
        "app",
        SurrealAuth::None,
    ))
    .unwrap();
    let replaced = store
        .replace(
            &CollectionName::new("events").unwrap(),
            &DocumentId::new("evt-9").unwrap(),
            &Event {
                label: "edited".to_owned(),
            },
        )
        .await;
    assert!(matches!(replaced, Err(PolyglotError::NotFound)));
    server.abort();
}

#[test]
fn any_mock_endpoint_or_credential_selects_the_offline_store() {
    let store = |endpoint: &str, auth: SurrealAuth| {
        SurrealDbStore::<Event>::connect_or_mock(SurrealConfig::new(endpoint, "main", "app", auth))
            .expect("valid configuration")
            .is_mock()
    };
    let live = "https://db.example.com";
    assert!(store("", SurrealAuth::None));
    assert!(store("mock://surreal", SurrealAuth::None));
    assert!(store("mock_local", SurrealAuth::None));
    assert!(store(live, SurrealAuth::basic("mock_user", "secret")));
    assert!(store(live, SurrealAuth::basic("user", "mock_secret")));
    assert!(store(live, SurrealAuth::basic("", "secret")));
    assert!(store(live, SurrealAuth::bearer("mock_token")));
    assert!(!store(live, SurrealAuth::basic("user", "secret")));
    assert!(!store(live, SurrealAuth::bearer("token")));
    assert!(!store(live, SurrealAuth::None));
}

#[test]
fn endpoints_are_bare_urls_and_configuration_debug_is_redacted() {
    let shape = "endpoint must not contain credentials, query parameters, or a fragment";
    for endpoint in [
        "https://user@db.example.com",
        "https://:pw@db.example.com",
        "https://db.example.com?ns=x",
        "https://db.example.com#x",
    ] {
        assert!(
            matches!(
                super::config::validate_endpoint(endpoint, false),
                Err(PolyglotError::InvalidConfiguration { reason, .. }) if reason == shape
            ),
            "{endpoint}"
        );
    }
    let base = super::config::validate_endpoint("https://db.example.com/surreal", false)
        .expect("valid endpoint");
    assert_eq!(base.as_str(), "https://db.example.com/surreal/");
    let slashed = super::config::validate_endpoint("https://db.example.com/surreal/", false)
        .expect("valid endpoint");
    assert_eq!(slashed.as_str(), "https://db.example.com/surreal/");

    assert_eq!(
        format!(
            "{:?}",
            SurrealConfig::new(
                "https://db.example.com",
                "main",
                "app",
                SurrealAuth::basic("user", "secret")
            )
        ),
        "SurrealConfig { endpoint: \"[CONFIGURED]\", namespace: \"main\", database: \"app\", auth: SurrealAuth::Basic([REDACTED]), response_limit: 1048576, allow_insecure_http: false }"
    );
    assert_eq!(format!("{:?}", SurrealAuth::None), "SurrealAuth::None");
    assert_eq!(
        format!("{:?}", SurrealAuth::bearer("token")),
        "SurrealAuth::Bearer([REDACTED])"
    );

    let config = || SurrealConfig::new("https://db.example.com", "main", "app", SurrealAuth::None);
    assert!(config().with_response_limit(8 * 1024 * 1024).is_ok());
    assert!(config().with_response_limit(8 * 1024 * 1024 + 1).is_err());
    assert!(config().with_response_limit(1024).is_ok());
    assert!(config().with_response_limit(1023).is_err());
}

#[test]
fn graph_queries_reach_exactly_their_size_and_row_limits() {
    let padded = "MATCH (n:person) RETURN n";
    let longest = format!("{padded}{}", "a".repeat(64 * 1024 - padded.len()));
    assert!(GraphQuery::read_only(longest.clone(), 1).is_ok());
    assert!(GraphQuery::read_only(format!("{longest}a"), 1).is_err());
    assert!(GraphQuery::read_only("   ", 1).is_err());
    assert!(GraphQuery::read_only("MATCH (n:person) RETURN n", 1_000).is_ok());
    assert!(GraphQuery::read_only("MATCH (n:person) RETURN n", 1_001).is_err());
}
