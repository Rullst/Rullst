use super::{
    BillingProvider, DEFAULT_WEBHOOK_TOLERANCE, SubscriptionStatus, WebhookEvent,
    WebhookVerificationMode, ensure_fresh_timestamp, url_encode, verify_explicit_mock_signature,
    webhook_mode_from_secret,
};
use crate::error::CapitalError;
use crate::{ChargeReceipt, ChargeRequest};
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use subtle::ConstantTimeEq;

/// Billing provider implementation for Stripe.
pub struct StripeProvider {
    api_key: String,
    webhook_secret: String,
    webhook_tolerance: Duration,
}

impl StripeProvider {
    /// Creates a new `StripeProvider` instance.
    pub fn new(api_key: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            webhook_secret: webhook_secret.into(),
            webhook_tolerance: DEFAULT_WEBHOOK_TOLERANCE,
        }
    }

    /// Overrides the default five-minute webhook timestamp acceptance window.
    pub fn with_webhook_tolerance(mut self, tolerance: Duration) -> Self {
        self.webhook_tolerance = tolerance;
        self
    }

    pub(super) fn usage_api_key(&self) -> &str {
        &self.api_key
    }

    /// Verifies the `Stripe-Signature` header signature (`t=1492774577,v1=604956efe...`).
    pub fn verify_signature(
        &self,
        payload: &[u8],
        signature_header: &str,
    ) -> Result<(), CapitalError> {
        self.verify_signature_at(payload, signature_header, chrono::Utc::now().timestamp())
    }

    /// Verifies a signature against an explicit clock value for deterministic tests.
    pub fn verify_signature_at(
        &self,
        payload: &[u8],
        signature_header: &str,
        now_unix_seconds: i64,
    ) -> Result<(), CapitalError> {
        match self.webhook_verification_mode()? {
            WebhookVerificationMode::Mock => {
                return verify_explicit_mock_signature(
                    self.name(),
                    &self.webhook_secret,
                    signature_header,
                );
            }
            WebhookVerificationMode::Real => {}
        }

        let mut timestamp = "";
        let mut signature_hex = "";

        for part in signature_header.split(',') {
            let mut kv = part.splitn(2, '=');
            let k = kv.next().unwrap_or("").trim();
            let v = kv.next().unwrap_or("").trim();
            if k == "t" {
                timestamp = v;
            } else if k == "v1" {
                signature_hex = v;
            }
        }

        if timestamp.is_empty() || signature_hex.is_empty() {
            return Err(CapitalError::InvalidSignature(
                "Invalid Stripe-Signature header format".to_string(),
            ));
        }

        let sig_bytes = hex::decode(signature_hex)
            .map_err(|e| CapitalError::InvalidSignature(format!("Invalid hex signature: {}", e)))?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());
        let mut ctx = hmac::Context::with_key(&key);
        ctx.update(timestamp.as_bytes());
        ctx.update(b".");
        ctx.update(payload);

        let tag = ctx.sign();
        if tag.as_ref().ct_eq(&sig_bytes).unwrap_u8() == 0 {
            return Err(CapitalError::InvalidSignature(
                "Stripe signature verification failed".to_string(),
            ));
        }

        ensure_fresh_timestamp(
            self.name(),
            timestamp,
            now_unix_seconds,
            self.webhook_tolerance,
            false,
        )?;

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for StripeProvider {
    fn name(&self) -> &'static str {
        "stripe"
    }

    fn webhook_verification_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        webhook_mode_from_secret(self.name(), &self.webhook_secret)
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn create_checkout_session(
        &self,
        customer_email: &str,
        plan_id: &str,
        redirect_url: &str,
    ) -> Result<String, CapitalError> {
        if customer_email.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Customer email cannot be empty".to_string(),
            ));
        }
        if plan_id.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Plan ID cannot be empty".to_string(),
            ));
        }

        if self.api_key.is_empty() || self.api_key.starts_with("mock_") {
            return Ok(format!(
                "https://checkout.stripe.com/pay/mock_session?email={}&plan={}&redirect={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = crate::providers::http_client()?;
        let body_str = format!(
            "mode=subscription&success_url={}&cancel_url={}&customer_email={}&line_items[0][price]={}&line_items[0][quantity]=1",
            url_encode(redirect_url),
            url_encode(redirect_url),
            url_encode(customer_email),
            url_encode(plan_id)
        );

        let body: Value = crate::providers::send_http_json(
            client
                .post("https://api.stripe.com/v1/checkout/sessions")
                .bearer_auth(&self.api_key)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body_str),
            "stripe",
            "create checkout",
        )
        .await?;

        let url = body["url"].as_str().ok_or_else(|| {
            CapitalError::from(crate::ProviderFailure::contract_mismatch(
                "stripe",
                "create checkout",
            ))
        })?;
        crate::providers::validate_checkout_url("stripe", url)
    }

    async fn charge(&self, request: &ChargeRequest) -> Result<ChargeReceipt, CapitalError> {
        super::stripe_charge::execute(&self.api_key, request).await
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let _ = self.webhook_verification_mode()?;
        let sig_header = headers.get("stripe-signature").ok_or_else(|| {
            CapitalError::InvalidSignature("Missing stripe-signature header".to_string())
        })?;
        self.verify_signature(payload, sig_header)?;

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        if let Some(event_type) = json["type"].as_str()
            && !event_type.starts_with("customer.subscription.")
        {
            return Err(CapitalError::PayloadParseError(format!(
                "Uninteresting event: {}",
                event_type
            )));
        }

        let data = &json["data"]["object"];
        let subscription_id = data["id"].as_str().unwrap_or("").to_string();
        let customer_id = data["customer"].as_str().unwrap_or("").to_string();
        let customer_email = data["customer_email"]
            .as_str()
            .or_else(|| data["customer_details"]["email"].as_str())
            .or_else(|| data["email"].as_str())
            .unwrap_or("")
            .to_string();

        let plan_id = data["lines"]["data"][0]["price"]["id"]
            .as_str()
            .or_else(|| data["items"]["data"][0]["price"]["id"].as_str())
            .or_else(|| data["plan"]["id"].as_str())
            .unwrap_or("")
            .to_string();

        let status_str = data["status"].as_str().unwrap_or("active");
        let ends_at = data["current_period_end"].as_i64();

        Ok(WebhookEvent {
            subscription_id,
            customer_id,
            customer_email,
            plan_id,
            status: SubscriptionStatus::parse_status(status_str),
            ends_at,
        })
    }

    async fn create_customer_portal(
        &self,
        customer_email: &str,
        return_url: &str,
    ) -> Result<String, CapitalError> {
        if customer_email.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Customer email cannot be empty".to_string(),
            ));
        }

        if self.api_key.is_empty() || self.api_key.starts_with("mock_") {
            return Ok(format!(
                "https://billing.stripe.com/p/session/mock_portal?email={}&return_url={}",
                url_encode(customer_email),
                url_encode(return_url)
            ));
        }

        Ok(format!(
            "https://billing.stripe.com/p/session/portal?email={}",
            url_encode(customer_email)
        ))
    }

    async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            crate::subscription::validate_provider_subscription_id(subscription_id)?;
            let client = crate::providers::http_client()?;
            crate::providers::send_http(
                client
                    .delete(format!(
                        "https://api.stripe.com/v1/subscriptions/{}",
                        subscription_id
                    ))
                    .bearer_auth(&self.api_key),
                "stripe",
                "cancel subscription",
            )
            .await?;
        }
        Ok(())
    }

    async fn pause_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            crate::subscription::validate_provider_subscription_id(subscription_id)?;
            let client = crate::providers::http_client()?;
            crate::providers::send_http(
                client
                    .post(format!(
                        "https://api.stripe.com/v1/subscriptions/{}",
                        subscription_id
                    ))
                    .bearer_auth(&self.api_key)
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body("pause_collection[behavior]=void"),
                "stripe",
                "pause subscription",
            )
            .await?;
        }
        Ok(())
    }

    async fn report_usage(
        &self,
        subscription_id: &str,
        metric: &str,
        quantity: u64,
    ) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        if metric.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Metric name cannot be empty".to_string(),
            ));
        }
        if quantity == 0 {
            return Err(CapitalError::InvalidUsage(
                "quantity must be greater than zero".to_string(),
            ));
        }
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            return Err(CapitalError::UnsupportedOperation(
                "Stripe's current meter-event API requires a customer, event name, timestamp and idempotency identifier; use StripeMeterEvent with MeteredBillingProvider"
                    .to_string(),
            ));
        }
        Ok(())
    }

    async fn apply_coupon(
        &self,
        subscription_id: &str,
        coupon_code: &str,
    ) -> Result<(), CapitalError> {
        super::stripe_subscription::apply_coupon(&self.api_key, subscription_id, coupon_code).await
    }

    async fn extend_trial(
        &self,
        subscription_id: &str,
        trial_ends_at: i64,
    ) -> Result<(), CapitalError> {
        super::stripe_subscription::extend_trial(&self.api_key, subscription_id, trial_ends_at)
            .await
    }
}

#[cfg(test)]
#[path = "stripe_tests.rs"]
mod tests;
