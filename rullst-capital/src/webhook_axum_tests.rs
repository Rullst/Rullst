#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::capital::SubscriptionStatus;
use crate::providers::StripeProvider;
use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::Extension;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::middleware;
use axum::routing::post;
use std::sync::Arc;
use tower::ServiceExt;

fn payload() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": "customer.subscription.updated",
        "data": {
            "object": {
                "object": "subscription",
                "id": "sub_axum",
                "customer": "cus_axum",
                "customer_email": "axum@example.com",
                "items": { "has_more": false, "data": [{ "price": { "id": "price_axum" } }] },
                "status": "active",
                "current_period_end": 1_900_000_000_i64
            }
        }
    }))
    .expect("fixture serialization must succeed")
}

async fn verified_handler(Extension(event): Extension<WebhookEvent>, body: Bytes) -> StatusCode {
    if body == payload()
        && event.subscription_id == "sub_axum"
        && event.customer_id == "cus_axum"
        && event.plan_id == "price_axum"
        && event.status == SubscriptionStatus::Active
    {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

fn state(allow_mock: bool) -> WebhookMiddlewareState {
    let provider = Arc::new(StripeProvider::new("mock_api", "mock_axum_signature"));
    let replay_store = Arc::new(InMemoryWebhookReplayStore::default());
    if allow_mock {
        WebhookMiddlewareState::local_mock_with_provider(provider, replay_store)
    } else {
        WebhookMiddlewareState::production_with_provider(provider, replay_store)
    }
}

fn app(state: WebhookMiddlewareState) -> Router {
    Router::new()
        .route("/webhook", post(verified_handler))
        .layer(middleware::from_fn_with_state(
            state,
            verify_webhook_with_state,
        ))
}

fn request(body: impl Into<Body>, signature: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method("POST").uri("/webhook");
    if let Some(signature) = signature {
        builder = builder.header("stripe-signature", signature);
    }
    builder.body(body.into()).expect("valid fixture request")
}

#[tokio::test]
async fn duplicate_standard_webhook_headers_are_rejected_before_dispatch() {
    for name in ["webhook-id", "webhook-timestamp", "webhook-signature"] {
        let mut duplicated = request(payload(), Some("mock_axum_signature"));
        duplicated
            .headers_mut()
            .append(name, HeaderValue::from_static("first"));
        duplicated
            .headers_mut()
            .append(name, HeaderValue::from_static("second"));
        assert_eq!(
            app(state(true)).oneshot(duplicated).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn state_middleware_preserves_body_inserts_event_and_rejects_replay() {
    let app = app(state(true));
    let mut accepted = request(payload(), Some("mock_axum_signature"));
    accepted
        .headers_mut()
        .insert("x-invalid-utf8", HeaderValue::from_bytes(&[0xff]).unwrap());
    assert_eq!(
        app.clone().oneshot(accepted).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );

    let replay = request(payload(), Some("mock_axum_signature"));
    assert_eq!(
        app.oneshot(replay).await.unwrap().status(),
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn state_middleware_maps_signature_payload_configuration_and_mock_errors() {
    let local = app(state(true));
    assert_eq!(
        local
            .clone()
            .oneshot(request(payload(), Some("wrong")))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        local
            .oneshot(request("not-json", Some("mock_axum_signature")))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );

    assert_eq!(
        app(state(false))
            .oneshot(request(payload(), Some("mock_axum_signature")))
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );

    let missing_secret = WebhookMiddlewareState::production_with_provider(
        Arc::new(StripeProvider::new("live_api_fixture", "")),
        Arc::new(InMemoryWebhookReplayStore::default()),
    );
    assert_eq!(
        app(missing_secret)
            .oneshot(request(payload(), None))
            .await
            .unwrap()
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn state_middleware_rejects_an_oversized_body_before_provider_dispatch() {
    let oversized = vec![b'x'; MAX_WEBHOOK_PAYLOAD_BYTES + 1];
    assert_eq!(
        app(state(true))
            .oneshot(request(oversized, Some("mock_axum_signature")))
            .await
            .unwrap()
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[test]
fn state_constructors_debug_resolution_statuses_and_poisoning_fail_closed() {
    let replay_store = Arc::new(InMemoryWebhookReplayStore::default());
    let production = WebhookMiddlewareState::production(replay_store.clone());
    let local = WebhookMiddlewareState::local_mock(replay_store);
    assert!(format!("{production:?}").contains("allow_mock: false"));
    assert!(format!("{local:?}").contains("allow_mock: true"));
    let _ = production.resolved_provider();

    for (error, expected) in [
        (CapitalError::General("internal".to_string()), 500),
        (CapitalError::StaleWebhook("old".to_string()), 401),
        (CapitalError::WebhookReplay("duplicate".to_string()), 409),
        (CapitalError::InvalidInvoice("invalid".to_string()), 400),
        (
            CapitalError::UnsupportedOperation("unsupported".to_string()),
            503,
        ),
    ] {
        assert_eq!(capital_error_status_code(&error), expected);
    }

    let poisoned = Arc::new(InMemoryWebhookReplayStore::default());
    let poison_target = poisoned.clone();
    assert!(
        std::thread::spawn(move || {
            let _guard = poison_target.entries.lock().unwrap();
            panic!("poison replay-store fixture");
        })
        .join()
        .is_err()
    );
    assert!(matches!(
        poisoned.check_and_record("after-poison"),
        Err(CapitalError::General(message)) if message.contains("poisoned")
    ));
}

async fn reached_handler() -> StatusCode {
    StatusCode::NO_CONTENT
}

#[tokio::test]
async fn global_middlewares_reject_duplicate_headers_before_provider_lookup() {
    let production = Router::new()
        .route("/webhook", post(reached_handler))
        .layer(middleware::from_fn(verify_webhook));
    let local = Router::new()
        .route("/webhook", post(reached_handler))
        .layer(middleware::from_fn(verify_webhook_mock_local));
    for app in [production, local] {
        let mut duplicated = request(payload(), Some("mock_axum_signature"));
        for value in ["first", "second"] {
            duplicated
                .headers_mut()
                .append("webhook-id", HeaderValue::from_static(value));
        }
        assert_eq!(
            app.oneshot(duplicated).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn a_single_standard_webhook_header_is_accepted() {
    for name in ["webhook-id", "webhook-timestamp", "webhook-signature"] {
        let mut single = request(payload(), Some("mock_axum_signature"));
        single
            .headers_mut()
            .insert(name, HeaderValue::from_static("only"));
        assert_eq!(
            app(state(true)).oneshot(single).await.unwrap().status(),
            StatusCode::NO_CONTENT,
            "{name}"
        );
    }
}

#[tokio::test]
async fn a_body_of_exactly_two_mebibytes_reaches_signature_verification() {
    let body = vec![b'x'; 2_097_152];
    assert_eq!(
        app(state(true))
            .oneshot(request(body, Some("wrong")))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

/// Answers each delivery with the next status of `statuses`, then 204.
fn scripted_app(state: WebhookMiddlewareState, statuses: Vec<StatusCode>) -> Router {
    let statuses = Arc::new(std::sync::Mutex::new(statuses.into_iter()));
    Router::new()
        .route(
            "/webhook",
            post(move || {
                let statuses = Arc::clone(&statuses);
                async move {
                    statuses
                        .lock()
                        .unwrap()
                        .next()
                        .unwrap_or(StatusCode::NO_CONTENT)
                }
            }),
        )
        .layer(middleware::from_fn_with_state(
            state,
            verify_webhook_with_state,
        ))
}

#[tokio::test]
async fn a_server_error_releases_the_claim_so_the_retry_is_accepted() {
    let app = scripted_app(
        state(true).with_retry_after_server_error(),
        vec![
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::SERVICE_UNAVAILABLE,
        ],
    );
    let deliver = || request(payload(), Some("mock_axum_signature"));
    for expected in [
        StatusCode::INTERNAL_SERVER_ERROR,
        StatusCode::SERVICE_UNAVAILABLE,
        StatusCode::NO_CONTENT,
        // Once the handler succeeded, the payload is a replay again.
        StatusCode::CONFLICT,
    ] {
        assert_eq!(
            app.clone().oneshot(deliver()).await.unwrap().status(),
            expected
        );
    }
}

#[tokio::test]
async fn a_client_error_keeps_the_claim() {
    let app = scripted_app(
        state(true).with_retry_after_server_error(),
        vec![StatusCode::UNPROCESSABLE_ENTITY],
    );
    let deliver = || request(payload(), Some("mock_axum_signature"));
    assert_eq!(
        app.clone().oneshot(deliver()).await.unwrap().status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        app.oneshot(deliver()).await.unwrap().status(),
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn without_the_opt_in_a_server_error_keeps_the_claim() {
    let app = scripted_app(state(true), vec![StatusCode::INTERNAL_SERVER_ERROR]);
    let deliver = || request(payload(), Some("mock_axum_signature"));
    assert_eq!(
        app.clone().oneshot(deliver()).await.unwrap().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        app.oneshot(deliver()).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    assert!(format!("{:?}", state(true)).contains("retry_after_server_error: false"));
    assert!(
        format!("{:?}", state(true).with_retry_after_server_error())
            .contains("retry_after_server_error: true")
    );
}

#[cfg(feature = "webhook-sql")]
#[tokio::test]
async fn the_sql_store_releases_the_claim_after_a_server_error() {
    let store = Arc::new(
        SqlWebhookReplayStore::connect("sqlite::memory:", 8, std::time::Duration::from_secs(60))
            .await
            .unwrap(),
    );
    store.prepare_schema().await.unwrap();
    let state = WebhookMiddlewareState::local_mock_with_provider(
        Arc::new(StripeProvider::new("mock_api", "mock_axum_signature")),
        Arc::clone(&store),
    )
    .with_retry_after_server_error();
    let app = scripted_app(state, vec![StatusCode::INTERNAL_SERVER_ERROR]);
    let deliver = || request(payload(), Some("mock_axum_signature"));
    assert_eq!(
        app.clone().oneshot(deliver()).await.unwrap().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    // The release runs in a task of its own; until it ends the retry is a replay.
    let mut status = StatusCode::CONFLICT;
    for _ in 0..200 {
        status = app.clone().oneshot(deliver()).await.unwrap().status();
        if status != StatusCode::CONFLICT {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        app.oneshot(deliver()).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    store.close().await;
}
