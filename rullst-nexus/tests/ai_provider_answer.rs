//! With a configured provider, the assistant answers with the provider's
//! reply rather than the offline schema helper. The process environment is
//! global, so this binary holds a single test that sets it first.
#![cfg(feature = "ai")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
use support::{authenticated_test_router, local_request};

use axum::body::Body;
use axum::http::StatusCode;
use rullst_nexus::Nexus;
use tower::ServiceExt;

#[tokio::test]
async fn a_configured_provider_answers_the_chat() {
    // SAFETY: this binary runs no other test, so nothing reads the
    // environment concurrently. A `mock_` key never reaches the network: the
    // provider answers with its deterministic offline echo.
    unsafe { std::env::set_var("GEMINI_API_KEY", "mock_nexus_answer_test") };
    let csrf = "nexus_provider_answer_csrf";
    let response = authenticated_test_router(Nexus::new())
        .oneshot(
            local_request()
                .method("POST")
                .uri("/chat/query")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Cookie", format!("rullst_csrf={csrf}"))
                .header("X-CSRF-Token", csrf)
                .body(Body::from("message=show+tables"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let answer = String::from_utf8(body.to_vec()).unwrap();
    assert!(answer.contains("Mock response from"), "{answer}");
    assert!(answer.contains("show tables"), "{answer}");
    assert!(
        !answer.contains("Registered Database Models"),
        "the offline helper must not answer: {answer}"
    );
}
