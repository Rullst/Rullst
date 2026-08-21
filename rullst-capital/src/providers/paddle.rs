use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
use crate::error::CapitalError;
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// Billing provider implementation for Paddle Billing (Enterprise Global MoR).
pub struct PaddleProvider {
    api_key: String,
    webhook_secret: String,
}

impl PaddleProvider {
    /// Creates a new `PaddleProvider` instance.
    pub fn new(api_key: String, webhook_secret: String) -> Self {
        Self {
            api_key,
            webhook_secret,
        }
    }

    /// Verifies the `Paddle-Signature` header (`ts=1690000000;h1=abcd...`).
    pub fn verify_signature(
        &self,
        payload: &[u8],
        signature_header: &str,
    ) -> Result<(), CapitalError> {
        if self.webhook_secret.is_empty() {
            return Ok(());
        }

        let mut timestamp = "";
        let mut signature_hex = "";

        for part in signature_header.split(';') {
            let mut kv = part.splitn(2, '=');
            let k = kv.next().unwrap_or("").trim();
            let v = kv.next().unwrap_or("").trim();
            if k == "ts" {
                timestamp = v;
            } else if k == "h1" {
                signature_hex = v;
            }
        }

        if timestamp.is_empty() || signature_hex.is_empty() {
            return Err(CapitalError::InvalidSignature(
                "Invalid Paddle-Signature header format".to_string(),
            ));
        }

        let sig_bytes = hex::decode(signature_hex)
            .map_err(|e| CapitalError::InvalidSignature(format!("Invalid hex signature: {}", e)))?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());
        let mut ctx = hmac::Context::with_key(&key);
        ctx.update(timestamp.as_bytes());
        ctx.update(b":");
        ctx.update(payload);

        let tag = ctx.sign();
        if tag.as_ref().ct_eq(&sig_bytes).unwrap_u8() == 0 {
            return Err(CapitalError::InvalidSignature(
                "Paddle signature verification failed".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for PaddleProvider {
    fn name(&self) -> &'static str {
        "paddle"
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
                "https://checkout.paddle.com/pay/mock_session?email={}&price_id={}&return_url={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "items": [{
                "price_id": plan_id,
                "quantity": 1
            }],
            "customer": {
                "email": customer_email
            },
            "return_url": redirect_url
        });

        let res = client
            .post("https://api.paddle.com/transactions")
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CapitalError::ProviderRequestFailed(format!("Network error: {}", e)))?;

        if !res.status().is_success() {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "Paddle API error: HTTP {}",
                res.status()
            )));
        }

        let body: Value = res.json().await.map_err(|e| {
            CapitalError::PayloadParseError(format!("Failed to parse response: {}", e))
        })?;

        body["data"]["checkout"]["url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                CapitalError::PayloadParseError(
                    "Missing checkout URL in Paddle response".to_string(),
                )
            })
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let sig_header = headers.get("paddle-signature");

        if let Some(sig) = sig_header {
            self.verify_signature(payload, sig)?;
        } else if !self.webhook_secret.is_empty() {
            return Err(CapitalError::InvalidSignature(
                "Missing paddle-signature header".to_string(),
            ));
        }

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        let data = &json["data"];
        let subscription_id = data["id"].as_str().unwrap_or("").to_string();
        let customer_id = data["customer_id"].as_str().unwrap_or("").to_string();
        let customer_email = data["customer"]["email"].as_str().unwrap_or("").to_string();

        let plan_id = data["items"][0]["price"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let status_str = data["status"].as_str().unwrap_or("active");
        let ends_at = data["current_billing_period"]["ends_at"]
            .as_str()
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|dt| dt.timestamp())
            });

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
            "https://paddle.com/portal?email={}",
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
                .post(format!(
                    "https://api.paddle.com/subscriptions/{}/cancel",
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
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            let client = reqwest::Client::new();
            let res = client
                .post(format!(
                    "https://api.paddle.com/subscriptions/{}/pause",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_paddle_provider_methods() {
        let provider = PaddleProvider::new("mock_key", "sec_paddle123");

        // 1. Checkout session
        let url = provider
            .create_checkout_session(
                "customer@paddle.com",
                "pri_pro_plan",
                "https://app.com/return",
            )
            .await
            .unwrap();
        assert!(url.contains("checkout.paddle.com"));
        assert!(url.contains("pri_pro_plan"));

        // 2. Checkout validation
        assert!(
            provider
                .create_checkout_session("", "plan", "url")
                .await
                .is_err()
        );
        assert!(
            provider
                .create_checkout_session("email", "", "url")
                .await
                .is_err()
        );

        // 3. Customer Portal
        let portal = provider
            .create_customer_portal("customer@paddle.com", "https://app.com")
            .await
            .unwrap();
        assert!(portal.contains("paddle.com/portal"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // 4. Subscription actions
        assert!(provider.cancel_subscription("sub_123").await.is_ok());
        assert!(provider.cancel_subscription("").await.is_err());

        assert!(provider.pause_subscription("sub_123").await.is_ok());
        assert!(provider.pause_subscription("").await.is_err());

        assert!(provider.report_usage("sub_123", "api", 10).await.is_ok());
        assert!(provider.report_usage("", "api", 10).await.is_err());

        assert!(provider.apply_coupon("sub_123", "SAVE20").await.is_ok());
        assert!(provider.apply_coupon("", "SAVE20").await.is_err());
        assert!(provider.apply_coupon("sub_123", "").await.is_err());

        assert!(provider.extend_trial("sub_123", 1800000000).await.is_ok());
        assert!(provider.extend_trial("", 1800000000).await.is_err());
        assert!(provider.extend_trial("sub_123", -1).await.is_err());
    }
}
