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
                "object": "subscription",
                "id": "sub_actix",
                "customer": "cus_actix",
                "customer_email": "actix@example.com",
                "items": { "has_more": false, "data": [{ "price": { "id": "price_actix" } }] },
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
async fn middleware_rejects_duplicate_standard_webhook_headers() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state()))
            .wrap(actix_web::middleware::from_fn(
                verify_webhook_actix_with_state,
            ))
            .route("/webhook", web::post().to(verified_handler)),
    )
    .await;
    for name in ["webhook-id", "webhook-timestamp", "webhook-signature"] {
        let request = test::TestRequest::post()
            .uri("/webhook")
            .insert_header(("stripe-signature", "mock_actix_signature"))
            .append_header((name, "first"))
            .append_header((name, "second"))
            .set_payload(payload())
            .to_request();
        assert_eq!(
            test::call_service(&app, request).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
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

#[tokio::test]
async fn middleware_accepts_one_standard_header_and_a_two_mebibyte_body() {
    for name in ["webhook-id", "webhook-timestamp", "webhook-signature"] {
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
            .insert_header((name, "only"))
            .set_payload(payload())
            .to_request();
        assert_eq!(
            test::call_service(&app, request).await.status(),
            StatusCode::NO_CONTENT,
            "{name}"
        );
    }

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
        .insert_header(("stripe-signature", "wrong"))
        .set_payload(vec![b'x'; 2_097_152])
        .to_request();
    assert_eq!(
        test::call_service(&app, request).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

async fn notification_handler(req: HttpRequest) -> HttpResponse {
    if req.extensions().get::<WebhookEvent>().is_some() {
        return HttpResponse::InternalServerError().finish();
    }
    match req
        .extensions()
        .get::<crate::PaymentNotification>()
        .map(|notification| notification.reference().to_string())
    {
        Some(reference) => HttpResponse::Ok().body(reference),
        None => HttpResponse::InternalServerError().finish(),
    }
}

#[tokio::test]
async fn unsigned_notifications_reach_the_handler_without_an_event() {
    let state = WebhookMiddlewareState::production_with_provider(
        Arc::new(crate::InfinitePayProvider::checkout("loja").unwrap()),
        Arc::new(InMemoryWebhookReplayStore::default()),
    );
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .wrap(actix_web::middleware::from_fn(
                verify_webhook_actix_with_state,
            ))
            .route("/webhook", web::post().to(notification_handler)),
    )
    .await;
    let body =
        r#"{"order_nsu":"order_42","transaction_nsu":"nsu-1","invoice_slug":"s1","status":"paid"}"#;
    for _ in 0..2 {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/webhook")
                .set_payload(body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(test::read_body(response).await, "order_42");
    }
    let rejected = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/webhook")
            .set_payload("{}")
            .to_request(),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
}
