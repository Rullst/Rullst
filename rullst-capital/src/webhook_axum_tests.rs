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
                "id": "sub_axum",
                "customer": "cus_axum",
                "customer_email": "axum@example.com",
                "items": { "data": [{ "price": { "id": "price_axum" } }] },
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
