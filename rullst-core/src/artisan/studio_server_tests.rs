//! Routes of the legacy Studio router.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt;

async fn call(method: &str, uri: &str) -> axum::response::Response {
    studio_router()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn the_root_redirects_and_registry_actions_are_not_implemented() {
    let root = call("GET", "/").await;
    assert_eq!(root.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(root.headers()[header::LOCATION], "/studio");

    for (uri, operation) in [
        ("/studio/api/migrations/run", "run migrations"),
        ("/studio/api/migrations/rollback", "roll back migrations"),
        ("/studio/api/seeders/run", "run seeders"),
    ] {
        let response = call("POST", uri).await;
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED, "{uri}");
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(
            json["message"],
            format!(
                "Studio cannot {operation} without an explicitly supplied application registry; use the project CLI"
            )
        );
    }
    assert_eq!(
        call("GET", "/studio/unknown").await.status(),
        StatusCode::NOT_FOUND
    );
}
