//! The radar's threat level reports local activity from counters and events
//! alone. Its own test binary, because it changes the process-wide store.
#![cfg(feature = "local")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use rullst_core::security::{LiveSecurityEvent, SecurityStore};
use std::sync::atomic::Ordering;
use tower::ServiceExt;

async fn threat_level() -> String {
    let response = rullst_studio::security_radar::router()
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let stats: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    stats["threat_level"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_counter_or_an_event_alone_marks_local_activity() {
    let store = SecurityStore::global();
    assert_eq!(threat_level().await, "NO_LOCAL_ACTIVITY_RECORDED");

    store.inc_log_redactions();
    assert_eq!(threat_level().await, "LOCAL_ACTIVITY_RECORDED");
    store.log_redactions_count.store(0, Ordering::Relaxed);
    assert_eq!(threat_level().await, "NO_LOCAL_ACTIVITY_RECORDED");

    store.push_local_event(LiveSecurityEvent::local(
        "STUDIO_TEST_EVENT",
        "an event without a counter or a ban",
        "127.0.0.1",
    ));
    assert_eq!(store.active_banned_count(), 0);
    assert_eq!(threat_level().await, "LOCAL_ACTIVITY_RECORDED");
}
