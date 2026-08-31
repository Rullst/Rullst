use super::{
    BillingProvider, SubscriptionStatus, WebhookEvent, WebhookVerificationMode, url_encode,
    verify_explicit_mock_signature, webhook_mode_from_secret,
};
use crate::error::CapitalError;
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;

/// Billing provider implementation for LemonSqueezy.
pub struct LemonSqueezyProvider {
    api_key: String,
    webhook_secret: String,
}

impl LemonSqueezyProvider {
    /// Creates a new `LemonSqueezyProvider` instance.
    pub fn new(api_key: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            webhook_secret: webhook_secret.into(),
        }
    }

    pub(super) fn usage_api_key(&self) -> &str {
        &self.api_key
    }

    /// Verifies the `X-Signature` header signature using HMAC-SHA256 of the raw body.
    pub fn verify_signature(
        &self,
        payload: &[u8],
        signature_hex: &str,
    ) -> Result<(), CapitalError> {
        if self.webhook_verification_mode()? == WebhookVerificationMode::Mock {
            return verify_explicit_mock_signature(
                self.name(),
                &self.webhook_secret,
                signature_hex,
            );
        }

        let sig_bytes = hex::decode(signature_hex)
            .map_err(|e| CapitalError::InvalidSignature(format!("Invalid hex signature: {}", e)))?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());

        hmac::verify(&key, payload, &sig_bytes).map_err(|_| {
            CapitalError::InvalidSignature("LemonSqueezy signature verification failed".to_string())
        })?;

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for LemonSqueezyProvider {
    fn name(&self) -> &'static str {
        "lemonsqueezy"
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
                "https://checkout.lemonsqueezy.com/checkout/mock_session?email={}&variant={}&redirect={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = crate::providers::http_client();
        let payload = serde_json::json!({
            "data": {
                "type": "checkouts",
                "attributes": {
                    "checkout_data": {
                        "email": customer_email
                    },
                    "product_options": {
                        "redirect_url": redirect_url
                    }
                },
                "relationships": {
                    "store": {
                        "data": {
                            "type": "stores",
                            "id": "1"
                        }
                    },
                    "variant": {
                        "data": {
                            "type": "variants",
                            "id": plan_id
                        }
                    }
                }
            }
        });

        let res = client
            .post("https://api.lemonsqueezy.com/v1/checkouts")
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/vnd.api+json")
            .header("Accept", "application/vnd.api+json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CapitalError::ProviderRequestFailed(format!("Network error: {}", e)))?;

        if !res.status().is_success() {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "LemonSqueezy API error: HTTP {}",
                res.status()
            )));
        }

        let body: Value = res.json().await.map_err(|e| {
            CapitalError::PayloadParseError(format!("Failed to parse response: {}", e))
        })?;

        body["data"]["attributes"]["url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                CapitalError::PayloadParseError(
                    "Missing checkout URL in LemonSqueezy response".to_string(),
                )
            })
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let _ = self.webhook_verification_mode()?;
        let sig_header = headers
            .get("x-signature")
            .or_else(|| headers.get("X-Signature"))
            .ok_or_else(|| {
                CapitalError::InvalidSignature("Missing X-Signature header".to_string())
            })?;
        self.verify_signature(payload, sig_header)?;

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        if let Some(event_name) = json["meta"]["event_name"].as_str()
            && !event_name.starts_with("subscription_")
        {
            return Err(CapitalError::PayloadParseError(format!(
                "Uninteresting event name: {}",
                event_name
            )));
        }

        let data = &json["data"];
        let attrs = &data["attributes"];

        let subscription_id = data["id"].as_str().unwrap_or("").to_string();
        let customer_id = attrs["customer_id"]
            .as_u64()
            .map(|id| id.to_string())
            .or_else(|| attrs["customer_id"].as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| attrs["customer_id"].to_string());
        let customer_email = attrs["user_email"].as_str().unwrap_or("").to_string();
        let plan_id = attrs["variant_id"]
            .as_u64()
            .map(|id| id.to_string())
            .or_else(|| attrs["variant_id"].as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| attrs["variant_id"].to_string());
        let status_str = attrs["status"].as_str().unwrap_or("active");

        let ends_at = attrs["ends_at"].as_str().and_then(|s| {
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
            "https://app.lemonsqueezy.com/my-orders?email={}",
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
                .delete(format!(
                    "https://api.lemonsqueezy.com/v1/subscriptions/{}",
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
            let payload = serde_json::json!({
                "data": {
                    "type": "subscriptions",
                    "id": subscription_id,
                    "attributes": {
                        "pause": {
                            "mode": "void"
                        }
                    }
                }
            });
            let res = client
                .patch(format!(
                    "https://api.lemonsqueezy.com/v1/subscriptions/{}",
                    subscription_id
                ))
                .bearer_auth(&self.api_key)
                .json(&payload)
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
                "Lemon Squeezy usage requires a subscription-item relationship, aggregation action and application event key; use LemonSqueezyUsageRecord with MeteredBillingProvider"
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
    async fn test_lemonsqueezy_provider_methods() {
        let provider = LemonSqueezyProvider::new("mock_key", "sec_lemon123");
        assert_eq!(provider.name(), "lemonsqueezy");

        // 1. Checkout session
        let url = provider
            .create_checkout_session("user@lemon.com", "variant_123", "https://app.com/success")
            .await
            .unwrap();
        assert!(url.contains("lemonsqueezy.com/checkout"));
        assert!(url.contains("variant_123"));

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

        // 3. Customer portal
        let portal = provider
            .create_customer_portal("user@lemon.com", "https://app.com")
            .await
            .unwrap();
        assert!(portal.contains("lemonsqueezy.com/my-orders"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // 4. Subscription actions
        assert!(provider.cancel_subscription("sub_lmn").await.is_ok());
        assert!(provider.cancel_subscription("").await.is_err());

        assert!(provider.pause_subscription("sub_lmn").await.is_ok());
        assert!(provider.pause_subscription("").await.is_err());

        assert!(provider.report_usage("sub_lmn", "api", 10).await.is_ok());
        assert!(provider.report_usage("", "api", 10).await.is_err());

        assert!(provider.apply_coupon("sub_lmn", "LEMON20").await.is_ok());
        assert!(provider.apply_coupon("", "LEMON20").await.is_err());
        assert!(provider.apply_coupon("sub_lmn", "").await.is_err());

        assert!(provider.extend_trial("sub_lmn", 1800000000).await.is_ok());
        assert!(provider.extend_trial("", 1800000000).await.is_err());
        assert!(provider.extend_trial("sub_lmn", -1).await.is_err());

        // 5. Signature verification
        let secret = "sec_lemon123";
        let payload = br#"{"data":{"id":"sub_lmn_100","attributes":{"customer_id":12,"user_email":"user@lemon.com","variant_id":123,"status":"active","renews_at":"2026-12-31T23:59:59Z"}}}"#;

        let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
        let sig = hmac::sign(&key, payload);
        let sig_hex = hex::encode(sig.as_ref());

        assert!(provider.verify_signature(payload, &sig_hex).is_ok());

        // Signature error paths
        let no_sec = LemonSqueezyProvider::new("k", "");
        assert!(matches!(
            no_sec.verify_signature(payload, ""),
            Err(CapitalError::ConfigurationError(_))
        ));
        assert!(provider.verify_signature(payload, "invalid_hex!").is_err());
        assert!(
            provider
                .verify_signature(
                    payload,
                    "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
                )
                .is_err()
        );

        // 6. Handle webhook
        let mut headers = HashMap::new();
        headers.insert("x-signature".to_string(), sig_hex);

        let event = provider.handle_webhook(payload, &headers).unwrap();
        assert_eq!(event.subscription_id, "sub_lmn_100");
        assert_eq!(event.customer_id, "12");
        assert_eq!(event.customer_email, "user@lemon.com");
        assert_eq!(event.status, SubscriptionStatus::Active);

        // Webhook error paths
        let empty_headers = HashMap::new();
        assert!(provider.handle_webhook(payload, &empty_headers).is_err());
        assert!(provider.handle_webhook(b"invalid json", &headers).is_err());
    }
}
