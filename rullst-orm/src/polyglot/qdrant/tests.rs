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

#[test]
fn bracketed_ipv6_loopback_is_a_local_endpoint() {
    assert!(
        QdrantStore::connect_or_mock(QdrantConfig::unauthenticated_local("http://[::1]:6333"))
            .is_ok()
    );
    assert!(
        QdrantStore::connect_or_mock(QdrantConfig::unauthenticated_local(
            "http://[2001:db8::1]:6333"
        ))
        .is_err()
    );
}

fn configuration_error(config: QdrantConfig) -> &'static str {
    match QdrantStore::connect_or_mock(config) {
        Err(PolyglotError::InvalidConfiguration { reason, .. }) => reason,
        Err(other) => panic!("unexpected error: {other:?}"),
        Ok(_) => panic!("configuration must be rejected"),
    }
}

#[test]
fn mock_selection_endpoint_shape_and_response_limits_are_exact() {
    let mock = |config| {
        QdrantStore::connect_or_mock(config)
            .expect("valid config")
            .is_mock()
    };
    assert!(mock(QdrantConfig::new(
        "https://qdrant.example.com",
        "mock_key"
    )));
    assert!(mock(QdrantConfig::new("mock://qdrant", "key")));
    assert!(!mock(QdrantConfig::new(
        "https://qdrant.example.com",
        "key"
    )));
    assert!(!mock(QdrantConfig::unauthenticated_local(
        "http://127.0.0.1:6333"
    )));

    let shape = "endpoint must not contain credentials, a path, query parameters, or a fragment";
    for endpoint in [
        "https://user@qdrant.example.com",
        "https://:pw@qdrant.example.com",
        "https://qdrant.example.com?x=1",
        "https://qdrant.example.com#x",
    ] {
        assert_eq!(
            configuration_error(QdrantConfig::new(endpoint, "key")),
            shape,
            "{endpoint}"
        );
    }

    assert_eq!(
        format!(
            "{:?}",
            QdrantConfig::new("https://qdrant.example.com", "key")
        ),
        "QdrantConfig { endpoint: \"[CONFIGURED]\", api_key: \"[REDACTED]\", response_limit: 2097152 }"
    );
    let config = || QdrantConfig::new("https://qdrant.example.com", "key");
    assert!(config().with_response_limit(16 * 1024 * 1024).is_ok());
    assert!(config().with_response_limit(16 * 1024 * 1024 + 1).is_err());
}

#[test]
fn vectors_and_payloads_reach_exactly_their_bounds() {
    use super::types::{validate_payload, validate_vector};
    assert!(validate_vector(&vec![1.0; 65_536], None).is_ok());
    assert!(validate_vector(&vec![1.0; 65_537], None).is_err());
    assert!(validate_vector(&[], None).is_err());

    // `{"p":"…"}` is 8 bytes plus the string.
    let exact = payload_of(1024 * 1024 - 8);
    assert_eq!(serde_json::to_vec(&exact).unwrap().len(), 1024 * 1024);
    assert!(validate_payload(&exact).is_ok());
    assert!(validate_payload(&payload_of(1024 * 1024 - 7)).is_err());
}

fn payload_of(len: usize) -> Map<String, Value> {
    let Value::Object(payload) = json!({ "p": "x".repeat(len) }) else {
        unreachable!("object literal must remain an object");
    };
    payload
}

#[tokio::test]
async fn mock_search_scores_are_cosine_similarities() {
    let store = QdrantStore::connect_or_mock(QdrantConfig::new("", "")).expect("mock");
    let collection = VectorCollectionName::new("cosine").expect("valid collection");
    store
        .create_collection(&collection, VectorDimensions::new(2).expect("dimensions"))
        .await
        .expect("collection");
    for (id, vector) in [(1, vec![1.0, 0.0]), (2, vec![3.0, 4.0])] {
        store
            .upsert(
                &collection,
                VectorPoint::new(id, vector, payload("p")).expect("point"),
            )
            .await
            .expect("upsert");
    }
    let matches = store
        .search(
            &collection,
            &[2.0, 0.0],
            VectorQueryLimit::new(2).expect("limit"),
        )
        .await
        .expect("search");
    let scores: Vec<(u64, f32)> = matches.iter().map(|hit| (hit.id(), hit.score())).collect();
    assert_eq!(scores, [(1, 1.0), (2, 0.6)]);
}
