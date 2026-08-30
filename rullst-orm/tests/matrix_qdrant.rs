#![cfg(feature = "qdrant")]

mod support;

use rullst_orm::{
    QdrantConfig, QdrantStore, VectorCollectionName, VectorDimensions, VectorPoint,
    VectorQueryLimit, VectorRepository,
};
use serde_json::{Map, Value, json};
use testcontainers::GenericImage;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;

fn payload(label: &str) -> Map<String, Value> {
    let Value::Object(payload) = json!({ "label": label }) else {
        unreachable!("object literal must remain an object");
    };
    payload
}

#[tokio::test]
async fn qdrant_adapter_passes_a_live_cosine_lifecycle() {
    let container = match GenericImage::new(
        "qdrant/qdrant",
        "v1.19.0@sha256:057ee3a8da769fe7310dd3537b4dc7583bf87a95ce8ac43c0af5a46bc580d1fc",
    )
    .with_wait_for(WaitFor::message_on_stdout("Qdrant HTTP listening on 6333"))
    .with_exposed_port(6333.tcp())
    .start()
    .await
    {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("Qdrant", error);
            return;
        }
    };
    let host = container
        .get_host()
        .await
        .expect("Qdrant host should be available");
    let port = container
        .get_host_port_ipv4(6333)
        .await
        .expect("Qdrant port should be available");
    let store = QdrantStore::connect_or_mock(QdrantConfig::unauthenticated_local(format!(
        "http://{host}:{port}"
    )))
    .expect("live Qdrant adapter should initialize");
    assert!(!store.is_mock());

    let collection = VectorCollectionName::new("rullst-matrix").expect("valid collection");
    store
        .create_collection(
            &collection,
            VectorDimensions::new(3).expect("valid dimensions"),
        )
        .await
        .expect("Qdrant collection should be created");
    store
        .upsert(
            &collection,
            VectorPoint::new(1, vec![1.0, 0.0, 0.0], payload("closest")).expect("valid point"),
        )
        .await
        .expect("closest point should be inserted");
    store
        .upsert(
            &collection,
            VectorPoint::new(2, vec![0.0, 1.0, 0.0], payload("farther")).expect("valid point"),
        )
        .await
        .expect("farther point should be inserted");

    let matches = store
        .search(
            &collection,
            &[0.9, 0.1, 0.0],
            VectorQueryLimit::new(2).expect("valid query limit"),
        )
        .await
        .expect("live vector query should succeed");
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].id(), 1);
    assert_eq!(matches[0].payload()["label"], "closest");
    assert!(matches[0].score() > matches[1].score());

    store
        .delete(&collection, 1)
        .await
        .expect("point deletion should be applied");
    let remaining = store
        .search(
            &collection,
            &[1.0, 0.0, 0.0],
            VectorQueryLimit::new(10).expect("valid query limit"),
        )
        .await
        .expect("query after deletion should succeed");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id(), 2);
}
