//! A configured AI provider is reported as active. The process environment is
//! global, so this binary holds a single test that sets it first.

mod support;
use support::{authenticated_test_router, local_request};

use axum::body::Body;
use rullst_nexus::Nexus;
use tower::ServiceExt;

#[tokio::test]
async fn the_security_page_names_the_configured_provider() {
    // SAFETY: this binary runs no other test, so nothing reads the
    // environment concurrently. A `mock_` key never reaches the network.
    unsafe { std::env::set_var("GEMINI_API_KEY", "mock_nexus_provider_test") };
    let names = rullst_ai::AutoAiConfig::from_env()
        .expect("valid AI configuration")
        .provider_names()
        .join(" → ");
    assert!(!names.is_empty());

    let response = authenticated_test_router(Nexus::new())
        .oneshot(
            local_request()
                .uri("/security")
                .header("hx-request", "true")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("security page");
    let bytes = axum::body::to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("bounded body");
    let html = String::from_utf8_lossy(&bytes);
    assert!(
        html.contains(&format!("Active: {names}</span>")),
        "{names}: {html}"
    );
}
