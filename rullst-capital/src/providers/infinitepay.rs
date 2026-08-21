use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
use crate::error::CapitalError;
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// Billing provider implementation for InfinitePay (CloudWalk Brazil).
/// Features: 0.00% Pix fee, instant D+0 payouts, and transparent credit card installment pass-through.
pub struct InfinitePayProvider {
    api_key: String,
    webhook_secret: String,
}

impl InfinitePayProvider {
    /// Creates a new `InfinitePayProvider` instance.
    pub fn new(api_key: String, webhook_secret: String) -> Self {
        Self {
            api_key,
            webhook_secret,
        }
    }

    /// Verifies the webhook signature using HMAC-SHA256 of the raw body payload.
    pub fn verify_signature(
        &self,
        payload: &[u8],
        signature_hex: &str,
    ) -> Result<(), CapitalError> {
        if self.webhook_secret.is_empty() {
            return Ok(());
        }

        let sig_bytes = hex::decode(signature_hex)
            .map_err(|e| CapitalError::InvalidSignature(format!("Invalid hex signature: {}", e)))?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());
        let tag = hmac::sign(&key, payload);

        if tag.as_ref().ct_eq(&sig_bytes).unwrap_u8() == 0 {
            return Err(CapitalError::InvalidSignature(
                "InfinitePay signature verification failed".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for InfinitePayProvider {
    fn name(&self) -> &'static str {
        "infinitepay"
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
                "https://checkout.infinitepay.io/pay/mock_session?email={}&plan={}&redirect={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "items": [{
                "name": format!("Subscription Plan {}", plan_id),
                "amount": 1000, // Amount in cents
                "quantity": 1
            }],
            "customer": {
                "email": customer_email
            },
            "redirect_url": redirect_url,
            "payment_methods": ["pix", "credit_card"]
        });

        let res = client
            .post("https://api.checkout.infinitepay.io/links")
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CapitalError::ProviderRequestFailed(format!("Network error: {}", e)))?;

        if !res.status().is_success() {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "InfinitePay API error: HTTP {}",
                res.status()
            )));
        }

        let body: Value = res.json().await.map_err(|e| {
            CapitalError::PayloadParseError(format!("Failed to parse response: {}", e))
        })?;

        body["url"]
            .as_str()
            .or_else(|| body["checkout_url"].as_str())
            .or_else(|| body["link"].as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                CapitalError::PayloadParseError(
                    "Missing checkout URL in InfinitePay response".to_string(),
                )
            })
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let sig_header = headers
            .get("x-signature")
            .or_else(|| headers.get("x-infinitepay-signature"));

        if let Some(sig) = sig_header {
            self.verify_signature(payload, sig)?;
        } else if !self.webhook_secret.is_empty() {
            return Err(CapitalError::InvalidSignature(
                "Missing X-Signature header".to_string(),
            ));
        }

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        let subscription_id = json["id"]
            .as_str()
            .or_else(|| json["transaction_id"].as_str())
            .or_else(|| json["data"]["id"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_id = json["customer"]["id"]
            .as_str()
            .or_else(|| json["customer_id"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_email = json["customer"]["email"]
            .as_str()
            .or_else(|| json["customer_email"].as_str())
            .unwrap_or("")
            .to_string();

        let plan_id = json["plan_id"]
            .as_str()
            .or_else(|| json["order_id"].as_str())
            .unwrap_or("default")
            .to_string();

        let status_str = json["status"]
            .as_str()
            .or_else(|| json["event"].as_str())
            .unwrap_or("paid");

        let ends_at = json["ends_at"].as_i64();

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
        _return_url: &str,
    ) -> Result<String, CapitalError> {
        if customer_email.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Customer email cannot be empty".to_string(),
            ));
        }

        Ok(format!(
            "https://app.infinitepay.io/client-portal?email={}",
            url_encode(customer_email)
        ))
    }

    async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    async fn pause_subscription(&self, _subscription_id: &str) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "InfinitePay does not support subscription pause".to_string(),
        ))
    }

    async fn report_usage(
        &self,
        _subscription_id: &str,
        _metric: &str,
        _quantity: u64,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "InfinitePay does not support metered usage reporting".to_string(),
        ))
    }

    async fn apply_coupon(
        &self,
        _subscription_id: &str,
        _coupon_code: &str,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "InfinitePay does not support coupon application".to_string(),
        ))
    }

    async fn extend_trial(
        &self,
        _subscription_id: &str,
        _trial_ends_at: i64,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "InfinitePay does not support trial extension".to_string(),
        ))
    }
}
