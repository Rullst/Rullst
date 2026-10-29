//! The AI assistant follows the `ai` feature (since 13.0.0-alpha.4): with it
//! `/chat` serves the assistant and the panel links to it; without it Nexus
//! does not link `rullst-ai`, shows no assistant link, and `/chat` and
//! `/chat/query` answer 404 with a notice that names the feature.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
use support::{authenticated_test_router, local_request};

use axum::body::Body;
use axum::http::StatusCode;
use rullst_nexus::Nexus;
use tower::ServiceExt;

async fn get(path: &str) -> (StatusCode, String) {
    let response = authenticated_test_router(Nexus::new())
        .oneshot(local_request().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

async fn ask(message: &str) -> (StatusCode, String) {
    let csrf = "nexus_assistant_feature_csrf";
    let response = authenticated_test_router(Nexus::new())
        .oneshot(
            local_request()
                .method("POST")
                .uri("/chat/query")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Cookie", format!("rullst_csrf={csrf}"))
                .header("X-CSRF-Token", csrf)
                .body(Body::from(format!("message={message}")))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[cfg(feature = "ai")]
#[tokio::test]
async fn with_the_ai_feature_the_panel_serves_and_links_the_assistant() {
    let (status, page) = get("/chat").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("AI Query Assistant"), "{page}");
    assert!(page.contains("nexus-chat-form"));

    let (status, dashboard) = get("/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(dashboard.contains("href=\"/nexus/chat\""));
    assert!(dashboard.contains("Open AI Query Assistant"));

    let (status, answer) = ask("show+tables").await;
    assert_eq!(status, StatusCode::OK);
    assert!(answer.contains("nexus-chat-user"), "{answer}");
}

#[cfg(not(feature = "ai"))]
#[tokio::test]
async fn without_the_ai_feature_the_assistant_answers_404_naming_the_feature() {
    let (status, page) = get("/chat").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(page.contains("AI assistant not enabled"), "{page}");
    assert!(page.contains("nexus-ai"), "{page}");
    assert!(!page.contains("nexus-chat-form"));

    let (status, answer) = ask("show+tables").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(answer.contains("not enabled in this build"), "{answer}");
    assert!(
        !answer.contains("nexus-chat-user"),
        "the message is not echoed"
    );

    let (status, dashboard) = get("/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!dashboard.contains("href=\"/nexus/chat\""), "{dashboard}");
    assert!(!dashboard.contains("Open AI Query Assistant"));

    let (_, security) = get("/security").await;
    assert!(
        security.contains("AI assistant not enabled in this build"),
        "{security}"
    );

    let (detected, reason) = rullst_nexus::detect_ai_provider();
    assert!(!detected);
    assert_eq!(reason, "AI assistant not enabled in this build");
}
