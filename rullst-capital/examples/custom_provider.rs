//! Writing your own payment provider: a complete example that runs offline.
//!
//! `AcmePay` is a fictitious gateway. The adapter implements `BillingProvider`,
//! authenticates webhooks inside `handle_webhook`, and is mounted behind the
//! canonical `verify_webhook_with_state` middleware, which adds the payload
//! bound, mock-mode policy and replay protection. Run it with:
//!
//! ```text
//! cargo run -p rullst-capital --example custom_provider
//! ```
//!
//! The walkthrough is `docs/src/capital-custom-provider.md`.

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Extension, Router, middleware};
use ring::hmac;
use rullst_capital::{
    Billable, BillingProvider, CapitalError, InMemoryWebhookReplayStore, SubscriptionStatus,
    WebhookEvent, WebhookMiddlewareState, WebhookVerificationMode, verify_webhook_with_state,
};
use std::collections::HashMap;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tower::ServiceExt;

// ANCHOR: provider
/// Largest clock difference accepted between AcmePay and this server.
const WEBHOOK_TOLERANCE_SECONDS: i64 = 5 * 60;

/// Adapter for the fictitious AcmePay gateway.
pub struct AcmePayProvider {
    api_key: String,
    webhook_secret: String,
}

impl AcmePayProvider {
    pub fn new(api_key: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            webhook_secret: webhook_secret.into(),
        }
    }

    /// Empty or `mock_*` credentials select the deterministic offline fixture.
    fn is_offline(&self) -> bool {
        self.api_key.is_empty() || self.api_key.starts_with("mock_")
    }

    /// Checks `acme-signature: t=<unix>,v1=<hex HMAC-SHA256("<t>.<body>")>`.
    fn verify_signature(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
        now: i64,
    ) -> Result<(), CapitalError> {
        let header = headers
            .get("acme-signature")
            .ok_or_else(|| CapitalError::InvalidSignature("missing acme-signature".into()))?;
        if self.webhook_verification_mode()? == WebhookVerificationMode::Mock {
            // Local fixtures still require the configured `mock_*` secret.
            return match header
                .as_bytes()
                .ct_eq(self.webhook_secret.as_bytes())
                .into()
            {
                true => Ok(()),
                false => Err(CapitalError::InvalidSignature("mock signature".into())),
            };
        }
        let mut timestamp = None;
        let mut signature = None;
        for part in header.split(',') {
            match part.split_once('=') {
                Some(("t", value)) => timestamp = value.parse::<i64>().ok(),
                Some(("v1", value)) => signature = hex::decode(value).ok(),
                _ => {}
            }
        }
        let (Some(timestamp), Some(signature)) = (timestamp, signature) else {
            return Err(CapitalError::InvalidSignature(
                "malformed acme-signature".into(),
            ));
        };
        if (now - timestamp).abs() > WEBHOOK_TOLERANCE_SECONDS {
            return Err(CapitalError::StaleWebhook("acmepay timestamp".into()));
        }
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());
        let mut signed = format!("{timestamp}.").into_bytes();
        signed.extend_from_slice(payload);
        // `ring::hmac::verify` compares in constant time.
        hmac::verify(&key, &signed, &signature)
            .map_err(|_| CapitalError::InvalidSignature("acmepay signature".into()))
    }
}

#[async_trait]
impl BillingProvider for AcmePayProvider {
    fn name(&self) -> &'static str {
        "acmepay"
    }

    // Without this override the canonical middleware refuses the provider.
    fn webhook_verification_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        match self.webhook_secret.trim() {
            "" => Err(CapitalError::ConfigurationError(
                "acmepay webhook secret cannot be empty".into(),
            )),
            secret if secret.starts_with("mock_") => Ok(WebhookVerificationMode::Mock),
            _ => Ok(WebhookVerificationMode::Real),
        }
    }

    async fn create_checkout_session(
        &self,
        _customer_email: &str,
        plan_id: &str,
        _redirect_url: &str,
    ) -> Result<String, CapitalError> {
        if plan_id.is_empty()
            || !plan_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(CapitalError::ConfigurationError("invalid plan ID".into()));
        }
        if self.is_offline() {
            // A reserved `.invalid` host never reaches a real gateway.
            return Ok(format!("https://mock.acmepay.invalid/checkout/{plan_id}"));
        }
        // Call the gateway here, bind the response to the request and return
        // only a validated HTTPS URL. Never report success you did not verify.
        Err(CapitalError::UnsupportedOperation(
            "acmepay live checkout is not implemented in this example".into(),
        ))
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        // Authenticate the exact bytes before parsing anything.
        self.verify_signature(payload, headers, chrono::Utc::now().timestamp())?;
        let json: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|_| CapitalError::PayloadParseError("acmepay JSON".into()))?;
        let text = |field: &str| {
            json["data"][field]
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| CapitalError::PayloadParseError(format!("acmepay {field}")))
        };
        // Map only documented states; an unknown state is an error, not access.
        let status = match text("status")?.as_str() {
            "active" => SubscriptionStatus::Active,
            "trialing" => SubscriptionStatus::Trialing,
            "past_due" => SubscriptionStatus::PastDue,
            "canceled" => SubscriptionStatus::Canceled,
            _ => return Err(CapitalError::PayloadParseError("acmepay status".into())),
        };
        Ok(WebhookEvent {
            subscription_id: text("subscription")?,
            customer_id: text("customer")?,
            customer_email: text("email")?,
            plan_id: text("plan")?,
            status,
            ends_at: json["data"]["current_period_end"].as_i64(),
        })
    }

    async fn create_customer_portal(
        &self,
        _customer_email: &str,
        _return_url: &str,
    ) -> Result<String, CapitalError> {
        if self.is_offline() {
            return Ok("https://mock.acmepay.invalid/portal".into());
        }
        Err(CapitalError::UnsupportedOperation("acmepay portal".into()))
    }

    async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.is_empty() {
            return Err(CapitalError::SubscriptionError(
                "empty subscription ID".into(),
            ));
        }
        if self.is_offline() {
            return Ok(());
        }
        Err(CapitalError::UnsupportedOperation("acmepay cancel".into()))
    }

    // Operations the gateway does not offer fail explicitly.
    async fn pause_subscription(&self, _subscription_id: &str) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation("acmepay pause".into()))
    }

    async fn report_usage(&self, _: &str, _: &str, _: u64) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation("acmepay usage".into()))
    }

    async fn apply_coupon(&self, _: &str, _: &str) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation("acmepay coupons".into()))
    }

    async fn extend_trial(&self, _: &str, _: i64) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation("acmepay trials".into()))
    }
}
// ANCHOR_END: provider

// ANCHOR: billable
/// The subscription owner, loaded from authenticated application state.
struct Workspace {
    billing_email: String,
    subscription_id: Option<String>,
}

#[async_trait]
impl Billable for Workspace {
    fn email(&self) -> String {
        self.billing_email.clone()
    }

    fn subscription_id(&self) -> Option<String> {
        self.subscription_id.clone()
    }
}
// ANCHOR_END: billable

// ANCHOR: webhook
async fn billing_webhook(Extension(event): Extension<WebhookEvent>) -> StatusCode {
    // Only events that passed the provider check and replay claim arrive here.
    // Persist them against the owner you already know; never trust the email alone.
    println!(
        "verified {} event for {}",
        event.status.as_str(),
        event.subscription_id
    );
    StatusCode::OK
}

fn billing_router(provider: Arc<AcmePayProvider>) -> Router {
    let replay_store = Arc::new(InMemoryWebhookReplayStore::default());
    // Production uses `WebhookMiddlewareState::production_with_provider`,
    // which rejects `mock_*` secrets; this offline example opts in to mocks.
    let state = WebhookMiddlewareState::local_mock_with_provider(provider, replay_store);
    Router::new()
        .route("/billing/webhook", post(billing_webhook))
        .layer(middleware::from_fn_with_state(
            state,
            verify_webhook_with_state,
        ))
}
// ANCHOR_END: webhook

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: checkout
    let provider = Arc::new(AcmePayProvider::new("mock_api_key", "mock_webhook_secret"));
    let workspace = Workspace {
        billing_email: "owner@example.com".into(),
        subscription_id: Some("sub_42".into()),
    };

    // Checkout through the trait object, exactly as for a built-in adapter.
    let checkout = provider
        .create_checkout_session(
            &workspace.email(),
            "plan_pro",
            "https://app.example/billing",
        )
        .await?;
    assert_eq!(checkout, "https://mock.acmepay.invalid/checkout/plan_pro");

    // Subscription operations with static dispatch on the explicit provider.
    workspace
        .subscription_with(provider.as_ref())?
        .cancel()
        .await?;
    let paused = workspace
        .subscription_with(provider.as_ref())?
        .pause()
        .await;
    assert!(matches!(paused, Err(CapitalError::UnsupportedOperation(_))));

    // `charge_with` keeps the trait's fail-closed default for direct charges.
    let charge = workspace
        .charge_with(provider.as_ref(), 4_990, "BRL", "cus_1", "pm_1", "order_1")
        .await;
    assert!(matches!(charge, Err(CapitalError::UnsupportedOperation(_))));
    // ANCHOR_END: checkout

    // A signed delivery passes once; the identical replay is refused.
    let body = r#"{"data":{"subscription":"sub_42","customer":"cus_1","email":"owner@example.com","plan":"plan_pro","status":"active"}}"#;
    let app = billing_router(Arc::clone(&provider));
    let delivery = || {
        Request::builder()
            .method("POST")
            .uri("/billing/webhook")
            .header("acme-signature", "mock_webhook_secret")
            .body(Body::from(body))
    };
    assert_eq!(
        app.clone().oneshot(delivery()?).await?.status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone().oneshot(delivery()?).await?.status(),
        StatusCode::CONFLICT
    );

    // A wrong signature never reaches the handler.
    let forged = Request::builder()
        .method("POST")
        .uri("/billing/webhook")
        .header("acme-signature", "mock_wrong")
        .body(Body::from(body))?;
    assert_eq!(
        app.oneshot(forged).await?.status(),
        StatusCode::UNAUTHORIZED
    );

    // The live HMAC path: sign `<t>.<body>` with the real secret.
    let live = AcmePayProvider::new("live_key", "whsec_live_fixture");
    let now = chrono::Utc::now().timestamp();
    let key = hmac::Key::new(hmac::HMAC_SHA256, b"whsec_live_fixture");
    let signature = hex::encode(hmac::sign(&key, format!("{now}.{body}").as_bytes()));
    let headers = HashMap::from([(
        "acme-signature".to_string(),
        format!("t={now},v1={signature}"),
    )]);
    let event = live.handle_webhook(body.as_bytes(), &headers)?;
    assert_eq!(event.status, SubscriptionStatus::Active);
    let stale = HashMap::from([(
        "acme-signature".to_string(),
        format!("t={},v1={signature}", now - 3_600),
    )]);
    assert!(live.handle_webhook(body.as_bytes(), &stale).is_err());

    println!("custom provider example passed");
    Ok(())
}
