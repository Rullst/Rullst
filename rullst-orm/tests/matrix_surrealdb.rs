#![cfg(feature = "surrealdb")]

mod support;

use rullst_orm::polyglot::{
    CollectionName, DocumentId, DocumentPage, DocumentRepository, GraphQuery, GraphRepository,
    PolyglotError, SurrealAuth, SurrealConfig, SurrealDbStore,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::surrealdb::{SURREALDB_PORT, SurrealDb};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Event {
    sequence: u64,
    label: String,
}

#[tokio::test]
async fn test_matrix_surrealdb_document_contract() {
    let container = match SurrealDb::default()
        .with_tag("v3.2.4")
        .with_env_var("SURREAL_CAPS_ALLOW_EXPERIMENTAL", "gql")
        .start()
        .await
    {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("SurrealDB", error);
            return;
        }
    };
    let host = container
        .get_host()
        .await
        .expect("SurrealDB container host should be available");
    let port = container
        .get_host_port_ipv4(SURREALDB_PORT)
        .await
        .expect("SurrealDB container port should be available");
    let endpoint = format!("http://{host}:{port}");
    let bootstrap = reqwest::Client::new()
        .post(format!("{endpoint}/sql"))
        .basic_auth("root", Some("root"))
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::CONTENT_TYPE, "text/plain")
        .body("DEFINE NAMESPACE rullst_matrix; USE NS rullst_matrix; DEFINE DATABASE persistence;")
        .send()
        .await
        .expect("SurrealDB namespace/database bootstrap should execute")
        .error_for_status()
        .expect("SurrealDB namespace/database bootstrap should return success")
        .json::<Vec<serde_json::Value>>()
        .await
        .expect("SurrealDB bootstrap response should be JSON");
    assert!(bootstrap.iter().all(|result| result["status"] == "OK"));

    let config = SurrealConfig::new(
        endpoint,
        "rullst_matrix",
        "persistence",
        SurrealAuth::basic("root", "root"),
    );
    let store = SurrealDbStore::<Event>::connect_or_mock(config)
        .expect("SurrealDB HTTP adapter should initialize");
    assert!(!store.is_mock());

    let collection = CollectionName::new("events").expect("valid collection");
    let first = DocumentId::new("event-01").expect("valid id");
    let second = DocumentId::new("event-02").expect("valid id");
    let event = |sequence| Event {
        sequence,
        label: format!("event-{sequence}"),
    };

    store
        .create(&collection, &second, &event(2))
        .await
        .expect("create second event");
    store
        .create(&collection, &first, &event(1))
        .await
        .expect("create first event");
    assert!(matches!(
        store.create(&collection, &first, &event(10)).await,
        Err(PolyglotError::Conflict)
    ));
    assert_eq!(
        store.find(&collection, &first).await.expect("find event"),
        Some(event(1))
    );

    store
        .replace(&collection, &first, &event(11))
        .await
        .expect("replace event");
    assert_eq!(
        store
            .list(&collection, DocumentPage::new(0, 1).expect("bounded page"),)
            .await
            .expect("list events"),
        vec![event(11)]
    );

    let graph_rows = store
        .query_graph(
            &GraphQuery::read_only(
                "MATCH (event:events) RETURN event.sequence AS sequence ORDER BY sequence",
                10,
            )
            .expect("bounded read-only GQL query"),
        )
        .await
        .expect("ISO GQL endpoint should return matching document nodes");
    assert_eq!(
        graph_rows,
        vec![json!({ "sequence": 2 }), json!({ "sequence": 11 })]
    );

    assert!(
        store
            .delete(&collection, &first)
            .await
            .expect("delete event")
    );
    assert!(
        !store
            .delete(&collection, &first)
            .await
            .expect("repeat delete")
    );
}
