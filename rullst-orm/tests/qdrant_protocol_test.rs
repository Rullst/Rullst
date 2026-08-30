#![cfg(feature = "qdrant")]

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::put,
};
use rullst_orm::{
    PolyglotError, QdrantConfig, QdrantStore, VectorCollectionName, VectorDimensions,
    VectorRepository,
};
use serde_json::{Value, json};
use tokio::sync::mpsc;

#[derive(Debug)]
struct CapturedRequest {
    api_key: Option<String>,
    body: Value,
}

async fn create_fixture(
    Path(collection): Path<String>,
    State(sender): State<mpsc::UnboundedSender<CapturedRequest>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let api_key = headers
        .get("api-key")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    sender
        .send(CapturedRequest { api_key, body })
        .expect("capture receiver should remain open");
    match collection.as_str() {
        "oversized" => Json(json!({
            "status": "ok",
            "result": "x".repeat(2_048)
        }))
        .into_response(),
        "failure" => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "provider-body-secret-must-not-escape",
        )
            .into_response(),
        _ => Json(json!({ "status": "ok", "result": true })).into_response(),
    }
}

#[tokio::test]
async fn authenticated_protocol_is_bounded_and_redacts_provider_bodies() {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let app = Router::new()
        .route("/collections/{collection}", put(create_fixture))
        .with_state(sender);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("fixture listener should bind");
    let address = listener
        .local_addr()
        .expect("fixture address should be available");
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("fixture server should run");
    });
    let endpoint = format!("http://{address}");

    let store = QdrantStore::connect_or_mock(QdrantConfig::new(&endpoint, "fixture-api-key"))
        .expect("authenticated loopback adapter should initialize");
    store
        .create_collection(
            &VectorCollectionName::new("captured").expect("valid collection"),
            VectorDimensions::new(3).expect("valid dimensions"),
        )
        .await
        .expect("fixture collection request should succeed");
    let captured = receiver
        .recv()
        .await
        .expect("fixture request should be captured");
    assert_eq!(captured.api_key.as_deref(), Some("fixture-api-key"));
    assert_eq!(
        captured.body,
        json!({ "vectors": { "size": 3, "distance": "Cosine" } })
    );

    let limited = QdrantStore::connect_or_mock(
        QdrantConfig::new(&endpoint, "fixture-api-key")
            .with_response_limit(1_024)
            .expect("valid response ceiling"),
    )
    .expect("limited adapter should initialize");
    assert!(matches!(
        limited
            .create_collection(
                &VectorCollectionName::new("oversized").expect("valid collection"),
                VectorDimensions::new(3).expect("valid dimensions"),
            )
            .await,
        Err(PolyglotError::ResponseTooLarge {
            backend: "Qdrant",
            limit_bytes: 1_024
        })
    ));
    receiver
        .recv()
        .await
        .expect("oversized fixture request should be captured");

    let error = store
        .create_collection(
            &VectorCollectionName::new("failure").expect("valid collection"),
            VectorDimensions::new(3).expect("valid dimensions"),
        )
        .await
        .expect_err("provider failure should remain visible");
    assert!(!error.to_string().contains("provider-body-secret"));
    receiver
        .recv()
        .await
        .expect("failure fixture request should be captured");

    server.abort();
}
