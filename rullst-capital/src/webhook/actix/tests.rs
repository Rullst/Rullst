use super::*;
use crate::capital::SubscriptionStatus;
use crate::providers::StripeProvider;
use actix_web::{App, HttpRequest, test};
use std::sync::Arc;

fn payload() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": "customer.subscription.updated",
        "data": {
            "object": {
                "id": "sub_actix",
                "customer": "cus_actix",
                "customer_email": "actix@example.com",
                "items": { "data": [{ "price": { "id": "price_actix" } }] },
                "status": "active",
                "current_period_end": 1_900_000_000_i64
            }
        }
    }))
    .expect("fixture serialization must succeed")
}

async fn verified_handler(req: HttpRequest, body: web::Bytes) -> HttpResponse {
    let Some(event) = req.extensions().get::<WebhookEvent>().cloned() else {
        return HttpResponse::InternalServerError().finish();
    };
    if body != payload()
        || event.subscription_id != "sub_actix"
        || event.customer_id != "cus_actix"
        || event.plan_id != "price_actix"
        || event.status != SubscriptionStatus::Active
    {
        return HttpResponse::InternalServerError().finish();
    }
    HttpResponse::NoContent().finish()
}

fn state() -> WebhookMiddlewareState {
    WebhookMiddlewareState::local_mock_with_provider(
        Arc::new(StripeProvider::new("mock_api", "mock_actix_signature")),
        Arc::new(InMemoryWebhookReplayStore::default()),
    )
}

#[tokio::test]
async fn middleware_preserves_body_inserts_event_and_rejects_replay() {
    // TM-PAY-01: the Actix boundary verifies before dispatch and preserves exact signed bytes.
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state()))
            .wrap(actix_web::middleware::from_fn(
                verify_webhook_actix_with_state,
            ))
            .route("/webhook", web::post().to(verified_handler)),
    )
    .await;

    let request = test::TestRequest::post()
        .uri("/webhook")
        .insert_header(("stripe-signature", "mock_actix_signature"))
        .set_payload(payload())
        .to_request();
    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let replay = test::TestRequest::post()
        .uri("/webhook")
        .insert_header(("stripe-signature", "mock_actix_signature"))
        .set_payload(payload())
        .to_request();
    let response = test::call_service(&app, replay).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn middleware_rejects_signature_payload_and_missing_state() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state()))
            .wrap(actix_web::middleware::from_fn(
                verify_webhook_actix_with_state,
            ))
            .route("/webhook", web::post().to(verified_handler)),
    )
    .await;

    let invalid_signature = test::TestRequest::post()
        .uri("/webhook")
        .insert_header(("stripe-signature", "wrong"))
        .set_payload(payload())
        .to_request();
    assert_eq!(
        test::call_service(&app, invalid_signature).await.status(),
        StatusCode::UNAUTHORIZED
    );

    let invalid_payload = test::TestRequest::post()
        .uri("/webhook")
        .insert_header(("stripe-signature", "mock_actix_signature"))
        .set_payload(b"not-json".as_slice())
        .to_request();
    assert_eq!(
        test::call_service(&app, invalid_payload).await.status(),
        StatusCode::BAD_REQUEST
    );

    let app_without_state = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(
                verify_webhook_actix_with_state,
            ))
            .route("/webhook", web::post().to(verified_handler)),
    )
    .await;
    let request = test::TestRequest::post()
        .uri("/webhook")
        .insert_header(("stripe-signature", "mock_actix_signature"))
        .set_payload(payload())
        .to_request();
    assert!(
        test::try_call_service(&app_without_state, request)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn middleware_bounds_the_request_body_before_verification() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state()))
            .wrap(actix_web::middleware::from_fn(
                verify_webhook_actix_with_state,
            ))
            .route("/webhook", web::post().to(verified_handler)),
    )
    .await;
    let oversized = vec![b'x'; MAX_WEBHOOK_PAYLOAD_BYTES + 1];
    let request = test::TestRequest::post()
        .uri("/webhook")
        .insert_header(("stripe-signature", "mock_actix_signature"))
        .set_payload(oversized)
        .to_request();

    assert_eq!(
        test::call_service(&app, request).await.status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}
