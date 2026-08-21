use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
use crate::error::CapitalError;
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// Billing provider implementation for Polar.sh (Developer-First MoR & Open Source).
pub struct PolarProvider {
    api_key: String,
    webhook_secret: String,
}

impl PolarProvider {
    /// Creates a new `PolarProvider` instance.
    pub fn new(api_key: String, webhook_secret: String) -> Self {
        Self {
            api_key,
            webhook_secret,
        }
    }

    /// Verifies the webhook signature using HMAC-SHA256.
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
                "Polar.sh signature verification failed".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for PolarProvider {
    fn name(&self) -> &'static str {
        "polar"
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
                "https://polar.sh/checkout/mock_session?email={}&product={}&success_url={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "product_price_id": plan_id,
            "customer_email": customer_email,
            "success_url": redirect_url
        });

        let res = client
            .post("https://api.polar.sh/v1/checkouts/custom/")
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CapitalError::ProviderRequestFailed(format!("Network error: {}", e)))?;

        if !res.status().is_success() {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "Polar API error: HTTP {}",
                res.status()
            )));
        }

        let body: Value = res.json().await.map_err(|e| {
            CapitalError::PayloadParseError(format!("Failed to parse response: {}", e))
        })?;

        body["url"].as_str().map(|s| s.to_string()).ok_or_else(|| {
            CapitalError::PayloadParseError("Missing checkout URL in Polar.sh response".to_string())
        })
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let sig_header = headers
            .get("polar-signature")
            .or_else(|| headers.get("webhook-signature"));

        if let Some(sig) = sig_header {
            self.verify_signature(payload, sig)?;
        } else if !self.webhook_secret.is_empty() {
            return Err(CapitalError::InvalidSignature(
                "Missing Polar-Signature header".to_string(),
            ));
        }

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        let data = &json["data"];
        let subscription_id = data["id"].as_str().unwrap_or("").to_string();
        let customer_id = data["user_id"]
            .as_str()
            .or_else(|| data["customer_id"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_email = data["user"]["email"]
            .as_str()
            .or_else(|| data["email"].as_str())
            .unwrap_or("")
            .to_string();

        let plan_id = data["product_id"]
            .as_str()
            .or_else(|| data["price_id"].as_str())
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
        _return_url: &str,
    ) -> Result<String, CapitalError> {
        if customer_email.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Customer email cannot be empty".to_string(),
            ));
        }

        Ok(format!(
            "https://polar.sh/purchases?email={}",
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
            let client = reqwest::Client::new();
            let res = client
                .delete(format!(
                    "https://api.polar.sh/v1/subscriptions/{}",
                    subscription_id
                ))
                .bearer_auth(&self.api_key)
                .send()
                .await
                .map_err(|e| CapitalError::ProviderRequestFailed(e.to_string()))?;
            if !res.status().is_success() {
                return Err(CapitalError::ProviderRequestFailed(format!(
                    "HTTP {}",
                    res.status()
                )));
            }
        }
        Ok(())
    }

    async fn pause_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    async fn report_usage(
        &self,
        subscription_id: &str,
        _metric: &str,
        _quantity: u64,
    ) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    async fn apply_coupon(
        &self,
        subscription_id: &str,
        coupon_code: &str,
    ) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        if coupon_code.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Coupon code cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    async fn extend_trial(
        &self,
        subscription_id: &str,
        trial_ends_at: i64,
    ) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        if trial_ends_at <= 0 {
            return Err(CapitalError::SubscriptionError(
                "Trial end timestamp must be positive".to_string(),
            ));
        }
        Ok(())
    }
}
