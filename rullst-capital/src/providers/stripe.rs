use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
use crate::error::CapitalError;
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// Billing provider implementation for Stripe.
pub struct StripeProvider {
    api_key: String,
    webhook_secret: String,
}

impl StripeProvider {
    /// Creates a new `StripeProvider` instance.
    pub fn new(api_key: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            webhook_secret: webhook_secret.into(),
        }
    }

    /// Verifies the `Stripe-Signature` header signature (`t=1492774577,v1=604956efe...`).
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

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for StripeProvider {
    fn name(&self) -> &'static str {
        "stripe"
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

        let client = reqwest::Client::new();
        let body_str = format!(
            "mode=subscription&success_url={}&cancel_url={}&customer_email={}&line_items[0][price]={}&line_items[0][quantity]=1",
            url_encode(redirect_url),
            url_encode(redirect_url),
            url_encode(customer_email),
            url_encode(plan_id)
        );

        let res = client
            .post("https://api.stripe.com/v1/checkout/sessions")
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body_str)
            .send()
            .await
            .map_err(|e| CapitalError::ProviderRequestFailed(format!("Network error: {}", e)))?;

        if !res.status().is_success() {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "Stripe API error: HTTP {}",
                res.status()
            )));
        }

        let body: Value = res.json().await.map_err(|e| {
            CapitalError::PayloadParseError(format!("Failed to parse response: {}", e))
        })?;

        body["url"].as_str().map(|s| s.to_string()).ok_or_else(|| {
            CapitalError::PayloadParseError("Missing checkout URL in Stripe response".to_string())
        })
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        if let Some(sig_header) = headers.get("stripe-signature") {
            self.verify_signature(payload, sig_header)?;
        } else if !self.webhook_secret.is_empty() {
            return Err(CapitalError::InvalidSignature(
                "Missing stripe-signature header".to_string(),
            ));
        }

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
            let client = reqwest::Client::new();
            let res = client
                .delete(format!(
                    "https://api.stripe.com/v1/subscriptions/{}",
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
                    "https://api.stripe.com/v1/subscriptions/{}",
                    subscription_id
                ))
                .bearer_auth(&self.api_key)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body("pause_collection[behavior]=void")
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
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            let client = reqwest::Client::new();
            let body = format!("quantity={}&action=increment", quantity);
            let res = client
                .post(format!(
                    "https://api.stripe.com/v1/subscription_items/{}/usage_records",
                    subscription_id
                ))
                .bearer_auth(&self.api_key)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
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
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            let client = reqwest::Client::new();
            let body = format!("coupon={}", coupon_code);
            let res = client
                .post(format!(
                    "https://api.stripe.com/v1/subscriptions/{}",
                    subscription_id
                ))
                .bearer_auth(&self.api_key)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
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
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            let client = reqwest::Client::new();
            let body = format!("trial_end={}", trial_ends_at);
            let res = client
                .post(format!(
                    "https://api.stripe.com/v1/subscriptions/{}",
                    subscription_id
                ))
                .bearer_auth(&self.api_key)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stripe_provider_methods() {
        let provider = StripeProvider::new("mock_key", "whsec_stripe123");
        assert_eq!(provider.name(), "stripe");

        // 1. Checkout session
        let url = provider
            .create_checkout_session(
                "user@stripe.com",
                "price_pro_month",
                "https://app.com/success",
            )
            .await
            .unwrap();
        assert!(url.contains("checkout.stripe.com"));
        assert!(url.contains("price_pro_month"));

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
            .create_customer_portal("user@stripe.com", "https://app.com")
            .await
            .unwrap();
        assert!(portal.contains("billing.stripe.com/p/session/mock_portal"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // 4. Subscription actions
        assert!(provider.cancel_subscription("sub_str").await.is_ok());
        assert!(provider.cancel_subscription("").await.is_err());

        assert!(provider.pause_subscription("sub_str").await.is_ok());
        assert!(provider.pause_subscription("").await.is_err());

        assert!(provider.report_usage("sub_str", "api", 10).await.is_ok());
        assert!(provider.report_usage("", "api", 10).await.is_err());

        assert!(provider.apply_coupon("sub_str", "PROMO10").await.is_ok());
        assert!(provider.apply_coupon("", "PROMO10").await.is_err());
        assert!(provider.apply_coupon("sub_str", "").await.is_err());

        assert!(provider.extend_trial("sub_str", 1800000000).await.is_ok());
        assert!(provider.extend_trial("", 1800000000).await.is_err());
        assert!(provider.extend_trial("sub_str", -1).await.is_err());

        // 5. Signature verification
        let secret = "whsec_stripe123";
        let timestamp = "1690000000";
        let payload = br#"{"data":{"object":{"id":"sub_str_100","customer":"cus_123","customer_email":"user@stripe.com","status":"active","items":{"data":[{"price":{"id":"price_pro"}}]}}}}"#;

        let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
        let mut ctx = hmac::Context::with_key(&key);
        ctx.update(timestamp.as_bytes());
        ctx.update(b".");
        ctx.update(payload);
        let sig_hex = hex::encode(ctx.sign().as_ref());
        let valid_header = format!("t={},v1={}", timestamp, sig_hex);

        assert!(provider.verify_signature(payload, &valid_header).is_ok());

        // Signature error paths
        let no_sec = StripeProvider::new("k", "");
        assert!(no_sec.verify_signature(payload, "").is_ok());
        assert!(
            provider
                .verify_signature(payload, "invalid_header")
                .is_err()
        );
        assert!(
            provider
                .verify_signature(payload, "t=123,v1=invalid_hex_!")
                .is_err()
        );
        assert!(
            provider
                .verify_signature(
                    payload,
                    "t=123,v1=00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
                )
                .is_err()
        );

        // 6. Handle webhook
        let mut headers = HashMap::new();
        headers.insert("stripe-signature".to_string(), valid_header);

        let event = provider.handle_webhook(payload, &headers).unwrap();
        assert_eq!(event.subscription_id, "sub_str_100");
        assert_eq!(event.customer_id, "cus_123");
        assert_eq!(event.customer_email, "user@stripe.com");
        assert_eq!(event.status, SubscriptionStatus::Active);

        // Webhook error paths
        let empty_headers = HashMap::new();
        assert!(provider.handle_webhook(payload, &empty_headers).is_err());
        assert!(provider.handle_webhook(b"invalid json", &headers).is_err());
    }
}
