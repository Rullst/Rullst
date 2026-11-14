#![cfg(feature = "surrealdb")]

mod support;

use rullst_orm::polyglot::{
    CollectionName, DocumentEntry, DocumentId, DocumentInventory, DocumentPage,
    DocumentRecoveryBinding, DocumentRecoveryKey, DocumentRecoveryPolicy, DocumentRepository,
    MockDocumentStore, SurrealAuth, SurrealConfig, SurrealDbStore, export_document_snapshot,
    restore_document_snapshot,
};
use serde::{Deserialize, Serialize};
use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::surrealdb::{SURREALDB_PORT, SurrealDb};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Event {
    sequence: u64,
    label: String,
}

fn event(sequence: u64) -> Event {
    Event {
        sequence,
        label: format!("event-{sequence}"),
    }
}

fn surreal_store(endpoint: &str, database: &str) -> SurrealDbStore<Event> {
    let store = SurrealDbStore::<Event>::connect_or_mock(SurrealConfig::new(
        endpoint,
        "recovery",
        database,
        SurrealAuth::basic("root", "root"),
    ))
    .expect("SurrealDB recovery store should initialize");
    assert!(!store.is_mock());
    store
}

/// A snapshot exported from a live SurrealDB database restores into the
/// deterministic store, and that store's snapshot restores into another live
/// database: export and restore cross a backend boundary in both directions.
#[tokio::test]
async fn test_matrix_document_recovery_surrealdb_deterministic_round_trip() {
    let surreal = match SurrealDb::default().with_tag("v3.2.4").start().await {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("SurrealDB recovery", error);
            return;
        }
    };
    let host = surreal
        .get_host()
        .await
        .expect("SurrealDB recovery host should be available");
    let port = surreal
        .get_host_port_ipv4(SURREALDB_PORT)
        .await
        .expect("SurrealDB recovery port should be available");
    let endpoint = format!("http://{host}:{port}");
    let bootstrap = reqwest::Client::new()
        .post(format!("{endpoint}/sql"))
        .basic_auth("root", Some("root"))
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::CONTENT_TYPE, "text/plain")
        .body(
            "DEFINE NAMESPACE recovery; USE NS recovery; \
             DEFINE DATABASE source; DEFINE DATABASE destination; \
             USE NS recovery DB source; DEFINE TABLE portable_events SCHEMALESS; \
             USE NS recovery DB destination; DEFINE TABLE portable_events SCHEMALESS;",
        )
        .send()
        .await
        .expect("SurrealDB recovery bootstrap should execute")
        .error_for_status()
        .expect("SurrealDB recovery bootstrap should succeed")
        .json::<Vec<serde_json::Value>>()
        .await
        .expect("SurrealDB recovery bootstrap should return JSON");
    assert!(bootstrap.iter().all(|result| result["status"] == "OK"));
    let source = surreal_store(&endpoint, "source");
    let destination = surreal_store(&endpoint, "destination");
    let deterministic = MockDocumentStore::<Event>::new();

    let collection = CollectionName::new("portable_events").expect("valid collection");
    for sequence in [3, 1, 2] {
        source
            .create(
                &collection,
                &DocumentId::new(format!("event-{sequence:02}")).expect("valid document ID"),
                &event(sequence),
            )
            .await
            .expect("seed SurrealDB source");
    }
    let binding = DocumentRecoveryBinding::try_new("matrix.production", collection.clone())
        .expect("valid recovery binding");
    let key = DocumentRecoveryKey::try_new("matrix-2026-09", [29; 32]).expect("valid recovery key");
    let policy =
        DocumentRecoveryPolicy::try_new(2, 10, 64 * 1024).expect("bounded recovery policy");

    let surreal_snapshot = export_document_snapshot(&source, &binding, &key, policy)
        .await
        .expect("SurrealDB export should stabilize");
    let to_deterministic =
        restore_document_snapshot(&deterministic, &surreal_snapshot, &binding, &key, policy)
            .await
            .expect("SurrealDB snapshot should restore into the deterministic store");
    assert_eq!(to_deterministic.inserted(), 3);
    assert_eq!(to_deterministic.verified(), 3);

    let deterministic_snapshot = export_document_snapshot(&deterministic, &binding, &key, policy)
        .await
        .expect("deterministic export should stabilize");
    let to_surreal = restore_document_snapshot(
        &destination,
        &deterministic_snapshot,
        &binding,
        &key,
        policy,
    )
    .await
    .expect("deterministic snapshot should restore into SurrealDB");
    assert_eq!(to_surreal.inserted(), 3);
    assert_eq!(to_surreal.verified(), 3);

    let entries: Vec<DocumentEntry<Event>> = destination
        .list_entries(&collection, DocumentPage::new(0, 10).expect("bounded page"))
        .await
        .expect("SurrealDB destination inventory");
    assert_eq!(
        entries
            .iter()
            .map(|entry| (entry.id().as_str(), entry.entity().sequence))
            .collect::<Vec<_>>(),
        vec![("event-01", 1), ("event-02", 2), ("event-03", 3)]
    );
}
