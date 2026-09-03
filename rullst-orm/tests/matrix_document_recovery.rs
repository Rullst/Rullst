#![cfg(all(feature = "mongodb", feature = "surrealdb"))]

mod support;

use rullst_orm::polyglot::{
    CollectionName, DocumentEntry, DocumentId, DocumentInventory, DocumentPage,
    DocumentRecoveryBinding, DocumentRecoveryKey, DocumentRecoveryPolicy, DocumentRepository,
    MongoDbStore, SurrealAuth, SurrealConfig, SurrealDbStore, export_document_snapshot,
    restore_document_snapshot,
};
use serde::{Deserialize, Serialize};
use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::{
    mongo::Mongo,
    surrealdb::{SURREALDB_PORT, SurrealDb},
};

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

#[tokio::test]
async fn test_matrix_document_recovery_mongodb_surrealdb_round_trip() {
    let mongo = match Mongo::default().start().await {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("MongoDB recovery", error);
            return;
        }
    };
    let surreal = match SurrealDb::default().with_tag("v3.2.4").start().await {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("SurrealDB recovery", error);
            return;
        }
    };

    let mongo_host = mongo
        .get_host()
        .await
        .expect("MongoDB recovery host should be available");
    let mongo_port = mongo
        .get_host_port_ipv4(27017)
        .await
        .expect("MongoDB recovery port should be available");
    let mongo_uri = format!("mongodb://{mongo_host}:{mongo_port}");
    let mongo_source = MongoDbStore::<Event>::connect_or_mock(&mongo_uri, "rullst_recovery_source")
        .await
        .expect("MongoDB source should initialize");
    let mongo_destination =
        MongoDbStore::<Event>::connect_or_mock(&mongo_uri, "rullst_recovery_destination")
            .await
            .expect("MongoDB destination should initialize");

    let surreal_host = surreal
        .get_host()
        .await
        .expect("SurrealDB recovery host should be available");
    let surreal_port = surreal
        .get_host_port_ipv4(SURREALDB_PORT)
        .await
        .expect("SurrealDB recovery port should be available");
    let surreal_endpoint = format!("http://{surreal_host}:{surreal_port}");
    let bootstrap = reqwest::Client::new()
        .post(format!("{surreal_endpoint}/sql"))
        .basic_auth("root", Some("root"))
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::CONTENT_TYPE, "text/plain")
        .body(
            "DEFINE NAMESPACE recovery; USE NS recovery; DEFINE DATABASE matrix; \
             USE NS recovery DB matrix; DEFINE TABLE portable_events SCHEMALESS;",
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
    let surreal_destination = SurrealDbStore::<Event>::connect_or_mock(SurrealConfig::new(
        surreal_endpoint,
        "recovery",
        "matrix",
        SurrealAuth::basic("root", "root"),
    ))
    .expect("SurrealDB destination should initialize");

    let collection = CollectionName::new("portable_events").expect("valid collection");
    for sequence in [3, 1, 2] {
        mongo_source
            .create(
                &collection,
                &DocumentId::new(format!("event-{sequence:02}")).expect("valid document ID"),
                &event(sequence),
            )
            .await
            .expect("seed MongoDB source");
    }
    let binding = DocumentRecoveryBinding::try_new("matrix.production", collection.clone())
        .expect("valid recovery binding");
    let key = DocumentRecoveryKey::try_new("matrix-2026-09", [29; 32]).expect("valid recovery key");
    let policy =
        DocumentRecoveryPolicy::try_new(2, 10, 64 * 1024).expect("bounded recovery policy");

    let mongo_snapshot = export_document_snapshot(&mongo_source, &binding, &key, policy)
        .await
        .expect("MongoDB export should stabilize");
    let to_surreal = restore_document_snapshot(
        &surreal_destination,
        &mongo_snapshot,
        &binding,
        &key,
        policy,
    )
    .await
    .expect("MongoDB snapshot should restore into SurrealDB");
    assert_eq!(to_surreal.inserted(), 3);
    assert_eq!(to_surreal.verified(), 3);

    let surreal_snapshot = export_document_snapshot(&surreal_destination, &binding, &key, policy)
        .await
        .expect("SurrealDB export should stabilize");
    let to_mongo = restore_document_snapshot(
        &mongo_destination,
        &surreal_snapshot,
        &binding,
        &key,
        policy,
    )
    .await
    .expect("SurrealDB snapshot should restore into MongoDB");
    assert_eq!(to_mongo.inserted(), 3);
    assert_eq!(to_mongo.verified(), 3);

    let entries: Vec<DocumentEntry<Event>> = mongo_destination
        .list_entries(&collection, DocumentPage::new(0, 10).expect("bounded page"))
        .await
        .expect("MongoDB destination inventory");
    assert_eq!(
        entries
            .iter()
            .map(|entry| (entry.id().as_str(), entry.entity().sequence))
            .collect::<Vec<_>>(),
        vec![("event-01", 1), ("event-02", 2), ("event-03", 3)]
    );
}
