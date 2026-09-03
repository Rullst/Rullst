#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::body::{Body, to_bytes};
use axum::http::Request as HttpRequest;
use axum::routing::get;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use tower::ServiceExt;

#[test]
fn registry_is_immutable_bounded_and_validated() {
    assert!(matches!(
        ApplicationLifecycle::with_required_components([""]),
        Err(ApplicationLifecycleError::InvalidComponentLabel(_))
    ));
    assert!(ApplicationLifecycle::with_required_components(["a".repeat(64)]).is_ok());
    assert!(matches!(
        ApplicationLifecycle::with_required_components(["a".repeat(65)]),
        Err(ApplicationLifecycleError::InvalidComponentLabel(_))
    ));
    assert!(matches!(
        ApplicationLifecycle::with_required_components(["database/primary"]),
        Err(ApplicationLifecycleError::InvalidComponentLabel(_))
    ));
    assert!(matches!(
        ApplicationLifecycle::with_required_components(["db", "db"]),
        Err(ApplicationLifecycleError::DuplicateComponent(_))
    ));
    let too_many = (0..=MAX_REQUIRED_COMPONENTS).map(|index| format!("component-{index}"));
    assert!(matches!(
        ApplicationLifecycle::with_required_components(too_many),
        Err(ApplicationLifecycleError::TooManyComponents)
    ));

    let lifecycle = ApplicationLifecycle::with_required_components(["database"]).unwrap();
    assert!(matches!(
        lifecycle.set_component_ready("cache", true),
        Err(ApplicationLifecycleError::UnknownComponent(_))
    ));
    assert!(!format!("{lifecycle:?}").contains("database"));
}

#[test]
fn readiness_requires_phase_and_every_component() {
    let lifecycle = ApplicationLifecycle::with_required_components(["database", "queue"]).unwrap();
    assert_eq!(lifecycle.phase(), ApplicationPhase::Starting);
    assert!(!lifecycle.snapshot().ready);

    lifecycle.mark_ready().unwrap();
    assert!(!lifecycle.snapshot().ready);
    lifecycle.set_component_ready("database", true).unwrap();
    lifecycle.set_component_ready("queue", true).unwrap();
    assert!(lifecycle.snapshot().ready);

    lifecycle.set_component_ready("queue", false).unwrap();
    assert!(!lifecycle.snapshot().ready);
    lifecycle.begin_draining().unwrap();
    assert_eq!(lifecycle.phase(), ApplicationPhase::Draining);
    assert!(matches!(
        lifecycle.mark_ready(),
        Err(ApplicationLifecycleError::InvalidTransition { .. })
    ));
    lifecycle.mark_stopped();
    assert_eq!(lifecycle.phase(), ApplicationPhase::Stopped);
}

#[tokio::test]
async fn drain_rejects_new_requests_and_waits_for_an_admitted_one() {
    // TM-CORE-02: draining must close admission without cancelling work that
    // was already accepted, and the wait must remain bounded.
    let lifecycle = ApplicationLifecycle::new();
    lifecycle.mark_ready().unwrap();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let handler_entered = Arc::clone(&entered);
    let handler_release = Arc::clone(&release);
    let app = apply_lifecycle(
        Router::new().route(
            "/work",
            get(move || {
                let entered = Arc::clone(&handler_entered);
                let release = Arc::clone(&handler_release);
                async move {
                    entered.notify_one();
                    release.notified().await;
                    "done"
                }
            }),
        ),
        lifecycle.clone(),
    );

    let first_app = app.clone();
    let first = tokio::spawn(async move {
        first_app
            .oneshot(HttpRequest::get("/work").body(Body::empty()).unwrap())
            .await
            .unwrap()
    });
    entered.notified().await;
    assert_eq!(lifecycle.in_flight_requests(), 1);
    lifecycle.begin_draining().unwrap();

    let rejected = app
        .oneshot(HttpRequest::get("/work").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(rejected.headers()[header::RETRY_AFTER], "1");
    assert!(matches!(
        lifecycle.wait_for_drain(Duration::from_millis(1)).await,
        Err(ApplicationLifecycleError::DrainTimedOut { in_flight: 1 })
    ));

    release.notify_one();
    assert_eq!(first.await.unwrap().status(), StatusCode::OK);
    lifecycle
        .wait_for_drain(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(lifecycle.in_flight_requests(), 0);
}

#[tokio::test]
async fn probes_bypass_application_admission_without_exposing_components() {
    let lifecycle = ApplicationLifecycle::with_required_components(["private-db"]).unwrap();
    let app = apply_lifecycle(
        Router::new()
            .route("/health", get(|| async { "health" }))
            .route("/ready", get(|| async { "ready" }))
            .route("/work", get(|| async { "work" })),
        lifecycle,
    );

    for request in [
        HttpRequest::get("/health").body(Body::empty()).unwrap(),
        HttpRequest::head("/ready").body(Body::empty()).unwrap(),
    ] {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let response = app
        .oneshot(HttpRequest::get("/work").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert!(
        !String::from_utf8(body.to_vec())
            .unwrap()
            .contains("private-db")
    );
}

#[test]
fn poisoned_component_state_fails_closed() {
    let lifecycle = ApplicationLifecycle::with_required_components(["database"]).unwrap();
    lifecycle.mark_ready().unwrap();
    let inner = Arc::clone(&lifecycle.inner);
    let panicked = Arc::new(AtomicBool::new(false));
    let panicked_in_thread = Arc::clone(&panicked);
    let _ = std::thread::spawn(move || {
        let _guard = inner.components.write().unwrap();
        panicked_in_thread.store(true, AtomicOrdering::Release);
        panic!("poison test-only component lock");
    })
    .join();
    assert!(panicked.load(AtomicOrdering::Acquire));
    let snapshot = lifecycle.snapshot();
    assert!(!snapshot.ready);
    assert!(!snapshot.state_available);
    assert_eq!(
        lifecycle.set_component_ready("database", true),
        Err(ApplicationLifecycleError::StateUnavailable)
    );
}
