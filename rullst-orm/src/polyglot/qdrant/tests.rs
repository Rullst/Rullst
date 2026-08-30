use serde_json::{Map, Value, json};

use super::{
    QdrantConfig, QdrantStore, VectorCollectionName, VectorDimensions, VectorPoint,
    VectorQueryLimit, VectorRepository,
};
use crate::polyglot::{Backend, Capability, PolyglotError};

fn payload(label: &str) -> Map<String, Value> {
    let Value::Object(payload) = json!({ "label": label }) else {
        unreachable!("object literal must remain an object");
    };
    payload
}

#[tokio::test]
async fn mock_runs_a_deterministic_cosine_lifecycle() {
    let store = QdrantStore::connect_or_mock(QdrantConfig::new("", ""))
        .expect("empty configuration should select the mock");
    assert!(store.is_mock());
    assert_eq!(QdrantStore::capabilities().backend(), Backend::Qdrant);
    assert!(
        QdrantStore::capabilities().supports(Capability::Vectors),
        "Qdrant should declare only its bounded vector capability"
    );
    let collection = VectorCollectionName::new("documents-v1").expect("valid collection");
    let dimensions = VectorDimensions::new(3).expect("valid dimensions");
    store
        .create_collection(&collection, dimensions)
        .await
        .expect("collection should be created");
    assert!(matches!(
        store.create_collection(&collection, dimensions).await,
        Err(PolyglotError::Conflict)
    ));

    store
        .upsert(
            &collection,
            VectorPoint::new(2, vec![0.0, 1.0, 0.0], payload("second")).expect("valid point"),
        )
        .await
        .expect("second point should be inserted");
    store
        .upsert(
            &collection,
            VectorPoint::new(1, vec![1.0, 0.0, 0.0], payload("closest")).expect("valid point"),
        )
        .await
        .expect("closest point should be inserted");

    let matches = store
        .search(
            &collection,
            &[0.9, 0.1, 0.0],
            VectorQueryLimit::new(2).expect("valid limit"),
        )
        .await
        .expect("search should succeed");
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].id(), 1);
    assert_eq!(matches[0].payload()["label"], "closest");
    assert!(matches[0].score() > matches[1].score());

    store
        .delete(&collection, 1)
        .await
        .expect("delete should succeed");
    let matches = store
        .search(
            &collection,
            &[1.0, 0.0, 0.0],
            VectorQueryLimit::new(2).expect("valid limit"),
        )
        .await
        .expect("search after delete should succeed");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id(), 2);
}

#[tokio::test]
async fn mock_rejects_dimension_mismatch() {
    let store = QdrantStore::connect_or_mock(QdrantConfig::new("mock://qdrant", "mock_key"))
        .expect("mock configuration should initialize");
    let collection = VectorCollectionName::new("documents").expect("valid collection");
    store
        .create_collection(
            &collection,
            VectorDimensions::new(3).expect("valid dimensions"),
        )
        .await
        .expect("collection should be created");
    let point = VectorPoint::new(1, vec![1.0, 0.0], Map::new()).expect("valid point");
    assert!(matches!(
        store.upsert(&collection, point).await,
        Err(PolyglotError::InvalidConfiguration {
            backend: "Qdrant",
            ..
        })
    ));
}

#[test]
fn validates_structural_and_resource_bounds() {
    assert!(VectorCollectionName::new("../escape").is_err());
    assert!(VectorCollectionName::new("-invalid").is_err());
    assert!(VectorDimensions::new(0).is_err());
    assert!(VectorQueryLimit::new(0).is_err());
    assert!(VectorQueryLimit::new(1_001).is_err());
    assert!(VectorPoint::new(1, Vec::new(), Map::new()).is_err());
    assert!(VectorPoint::new(1, vec![0.0, 0.0], Map::new()).is_err());
    assert!(VectorPoint::new(1, vec![f32::NAN], Map::new()).is_err());
    let mut oversized = Map::new();
    oversized.insert("content".to_owned(), Value::String("x".repeat(1024 * 1024)));
    assert!(VectorPoint::new(1, vec![1.0], oversized).is_err());
    assert!(
        QdrantConfig::new("mock_local", "mock_key")
            .with_response_limit(512)
            .is_err()
    );
}

#[test]
fn endpoint_policy_and_debug_output_are_fail_closed() {
    assert!(
        QdrantStore::connect_or_mock(QdrantConfig::new("http://qdrant.example.com", "secret"))
            .is_err()
    );
    assert!(
        QdrantStore::connect_or_mock(QdrantConfig::unauthenticated_local(
            "https://qdrant.example.com"
        ))
        .is_err()
    );
    assert!(
        QdrantStore::connect_or_mock(QdrantConfig::new(
            "https://qdrant.example.com/path",
            "secret"
        ))
        .is_err()
    );
    assert!(
        QdrantStore::connect_or_mock(QdrantConfig::new(
            "https://qdrant.example.com",
            "line\nbreak"
        ))
        .is_err()
    );
    let rendered = format!(
        "{:?}",
        QdrantConfig::new("https://private.example.com", "top-secret")
    );
    assert!(!rendered.contains("private.example.com"));
    assert!(!rendered.contains("top-secret"));
}
