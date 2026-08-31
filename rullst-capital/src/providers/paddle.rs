use super::{
    BillingProvider, DEFAULT_WEBHOOK_TOLERANCE, SubscriptionStatus, WebhookEvent,
    WebhookVerificationMode, ensure_fresh_timestamp, url_encode, verify_explicit_mock_signature,
    webhook_mode_from_secret,
};
use crate::error::CapitalError;
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use subtle::ConstantTimeEq;

/// Billing provider implementation for Paddle Billing (Enterprise Global MoR).
pub struct PaddleProvider {
    api_key: String,
    webhook_secret: String,
    webhook_tolerance: Duration,
}

impl PaddleProvider {
    /// Creates a new `PaddleProvider` instance.
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

    /// Verifies the `Paddle-Signature` header (`ts=1690000000;h1=abcd...`).
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
impl BillingProvider for PaddleProvider {
    fn name(&self) -> &'static str {
        "paddle"
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
                "https://checkout.paddle.com/pay/mock_session?email={}&price_id={}&return_url={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = crate::providers::http_client();
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
        let _ = self.webhook_verification_mode()?;
        let sig_header = headers.get("paddle-signature").ok_or_else(|| {
            CapitalError::InvalidSignature("Missing paddle-signature header".to_string())
        })?;
        self.verify_signature(payload, sig_header)?;

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
            let client = crate::providers::http_client();
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
            let client = crate::providers::http_client();
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
        crate::subscription::validate_provider_subscription_id(subscription_id)?;
        let _coupon = crate::subscription::validate_coupon_code(coupon_code)?;
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            return Err(CapitalError::UnsupportedOperation(
                "Paddle coupon application has no reviewed live contract".to_string(),
            ));
        }
        Ok(())
    }

    async fn extend_trial(
        &self,
        subscription_id: &str,
        trial_ends_at: i64,
    ) -> Result<(), CapitalError> {
        crate::subscription::validate_provider_subscription_id(subscription_id)?;
        crate::subscription::validate_trial_end(trial_ends_at)?;
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            return Err(CapitalError::UnsupportedOperation(
                "Paddle trial extension has no reviewed live contract".to_string(),
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
        assert_eq!(provider.name(), "paddle");

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

        // 5. Signature verification
        let secret = "sec_paddle123";
        let now = chrono::Utc::now().timestamp();
        let timestamp = now.to_string();
        let payload = br#"{"data":{"id":"sub_pad_100","customer_id":"ct_999","customer":{"email":"pad@test.com"},"items":[{"price":{"id":"pri_pro"}}],"status":"active","current_billing_period":{"ends_at":"2026-12-31T23:59:59Z"}}}"#;

        let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
        let mut ctx = hmac::Context::with_key(&key);
        ctx.update(timestamp.as_bytes());
        ctx.update(b":");
        ctx.update(payload);
        let sig_hex = hex::encode(ctx.sign().as_ref());
        let valid_header = format!("ts={};h1={}", timestamp, sig_hex);

        assert!(
            provider
                .verify_signature_at(payload, &valid_header, now)
                .is_ok()
        );
        assert!(matches!(
            provider.verify_signature_at(payload, &valid_header, now + 301),
            Err(CapitalError::StaleWebhook(_))
        ));

        // Signature error paths
        let no_secret_provider = PaddleProvider::new("key", "");
        assert!(matches!(
            no_secret_provider.verify_signature(payload, ""),
            Err(CapitalError::ConfigurationError(_))
        ));
        assert!(provider.verify_signature(payload, "bad_header").is_err());
        assert!(
            provider
                .verify_signature(payload, "ts=123;h1=invalid_hex_!")
                .is_err()
        );
        assert!(
            provider
                .verify_signature(
                    payload,
                    "ts=123;h1=00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
                )
                .is_err()
        );

        // 6. Handle webhook
        let mut headers = HashMap::new();
        headers.insert("paddle-signature".to_string(), valid_header);

        let event = provider.handle_webhook(payload, &headers).unwrap();
        assert_eq!(event.subscription_id, "sub_pad_100");
        assert_eq!(event.customer_id, "ct_999");
        assert_eq!(event.customer_email, "pad@test.com");
        assert_eq!(event.plan_id, "pri_pro");
        assert_eq!(event.status, SubscriptionStatus::Active);
        assert!(event.ends_at.is_some());

        // Webhook error paths
        let empty_headers = HashMap::new();
        assert!(provider.handle_webhook(payload, &empty_headers).is_err());
        assert!(provider.handle_webhook(b"invalid json", &headers).is_err());
    }
}
