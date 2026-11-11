//! Unauthenticated notifications confirmed at the source never become events.

use super::*;
use crate::capital::SubscriptionStatus;
use crate::providers::InfinitePayProvider;
use axum::Router;
use axum::extract::Extension;
use axum::http::{Request as HttpRequest, StatusCode};
use axum::routing::post;
use std::sync::atomic::{AtomicBool, Ordering};
use tower::ServiceExt;

/// Declares the unsigned mode but would report every body as paid.
#[derive(Default)]
struct Unsigned {
    handled: AtomicBool,
    parses: bool,
}

#[async_trait::async_trait]
impl BillingProvider for Unsigned {
    fn name(&self) -> &'static str {
        "unsigned"
    }
    fn webhook_verification_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        Ok(WebhookVerificationMode::ConfirmedAtSource)
    }
    fn parse_payment_notification(
        &self,
        payload: &[u8],
        _headers: &HashMap<String, String>,
    ) -> Result<PaymentNotification, CapitalError> {
        if !self.parses {
            return Err(CapitalError::UnsupportedOperation("not implemented".into()));
        }
        PaymentNotification::new(String::from_utf8_lossy(payload).trim())
    }
    async fn create_checkout_session(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<String, CapitalError> {
        Err(CapitalError::UnsupportedOperation("fixture".into()))
    }
    fn handle_webhook(
        &self,
        _payload: &[u8],
        _headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        self.handled.store(true, Ordering::SeqCst);
        Ok(WebhookEvent {
            subscription_id: "sub".into(),
            customer_id: "cus".into(),
            customer_email: "attacker@example.com".into(),
            plan_id: "plan".into(),
            status: SubscriptionStatus::Active,
            ends_at: None,
        })
    }
    async fn create_customer_portal(&self, _: &str, _: &str) -> Result<String, CapitalError> {
        Err(CapitalError::UnsupportedOperation("fixture".into()))
    }
    async fn cancel_subscription(&self, _: &str) -> Result<(), CapitalError> {
        Ok(())
    }
    async fn pause_subscription(&self, _: &str) -> Result<(), CapitalError> {
        Ok(())
    }
    async fn report_usage(&self, _: &str, _: &str, _: u64) -> Result<(), CapitalError> {
        Ok(())
    }
    async fn apply_coupon(&self, _: &str, _: &str) -> Result<(), CapitalError> {
        Ok(())
    }
    async fn extend_trial(&self, _: &str, _: i64) -> Result<(), CapitalError> {
        Ok(())
    }
}

async fn handler(
    event: Option<Extension<WebhookEvent>>,
    Extension(notification): Extension<PaymentNotification>,
) -> (StatusCode, String) {
    if event.is_some() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "event inserted".into());
    }
    (StatusCode::OK, notification.reference().to_string())
}

fn app<P: BillingProvider + 'static>(provider: Arc<P>) -> Router {
    let state = WebhookMiddlewareState::production_with_provider(
        provider,
        Arc::new(InMemoryWebhookReplayStore::default()),
    );
    Router::new()
        .route("/webhook", post(handler))
        .layer(axum::middleware::from_fn_with_state(
            state,
            verify_webhook_with_state,
        ))
}

async fn post_body(app: &Router, body: &'static str) -> (StatusCode, String) {
    let request = HttpRequest::builder()
        .method("POST")
        .uri("/webhook")
        .body(Body::from(body))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn middleware_inserts_a_notification_and_never_calls_handle_webhook() {
    let provider = Arc::new(Unsigned {
        parses: true,
        ..Unsigned::default()
    });
    let app = app(provider.clone());
    assert_eq!(
        post_body(&app, "order_42").await,
        (StatusCode::OK, "order_42".into())
    );
    // A repeated notification only repeats the confirmation: no replay 409.
    assert_eq!(post_body(&app, "order_42").await.0, StatusCode::OK);
    assert_eq!(post_body(&app, "order 42").await.0, StatusCode::BAD_REQUEST);
    assert!(!provider.handled.load(Ordering::SeqCst));

    // The signed-event path refuses the unsigned mode outright.
    let store = WebhookReplayBackend::Memory(Arc::new(InMemoryWebhookReplayStore::default()));
    assert!(matches!(
        verify_payload(
            provider.as_ref(),
            b"order_42",
            &HashMap::new(),
            &store,
            true
        )
        .await,
        Err(CapitalError::UnsupportedOperation(_))
    ));
    assert!(!provider.handled.load(Ordering::SeqCst));
}

#[tokio::test]
async fn a_provider_without_a_notification_reader_is_unavailable() {
    let app = app(Arc::new(Unsigned::default()));
    assert_eq!(
        post_body(&app, "order_42").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn infinitepay_bodies_reach_the_handler_as_status_free_notifications() {
    for provider in [
        InfinitePayProvider::checkout("loja").unwrap(),
        // Unsigned in mock mode too: its confirmation is a mock answer.
        InfinitePayProvider::checkout("mock_loja").unwrap(),
    ] {
        let app = app(Arc::new(provider));
        let body = r#"{"order_nsu":"order_42","transaction_nsu":"nsu-1","invoice_slug":"s1","paid_amount":1,"status":"paid"}"#;
        assert_eq!(
            post_body(&app, body).await,
            (StatusCode::OK, "order_42".into())
        );
        assert_eq!(post_body(&app, "{}").await.0, StatusCode::BAD_REQUEST);
    }
}
