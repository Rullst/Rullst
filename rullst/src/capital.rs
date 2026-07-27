//! # Rullst Capital
//!
//! SaaS Billing & Subscription Engine for Rullst applications.
//! Supports Stripe and LemonSqueezy out of the box with secure webhook validation.

use async_trait::async_trait;
use ring::hmac;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// The semantic status of a SaaS Subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// The subscription is active and in good standing.
    Active,
    /// The subscription was canceled.
    Canceled,
    /// The subscription is past due but not yet unpaid.
    PastDue,
    /// The subscription is unpaid and access is revoked.
    Unpaid,
    /// The subscription is currently in a free trial period.
    Trialing,
    /// The subscription has been paused.
    Paused,
}

impl SubscriptionStatus {
    /// Returns the static string representation of the subscription status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Canceled => "canceled",
            Self::PastDue => "past_due",
            Self::Unpaid => "unpaid",
            Self::Trialing => "trialing",
            Self::Paused => "paused",
        }
    }

    /// Parses a string representation of a subscription status.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn parse_status(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" => Self::Active,
            "canceled" | "cancelled" => Self::Canceled,
            "past_due" => Self::PastDue,
            "unpaid" => Self::Unpaid,
            "trialing" => Self::Trialing,
            "paused" => Self::Paused,
            _ => Self::Unpaid,
        }
    }
}

/// Unified model representing a webhook event for subscription changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Unique identifier of the subscription in the provider system.
    pub subscription_id: String,
    /// Unique customer ID in the provider system.
    pub customer_id: String,
    /// Email of the customer.
    pub customer_email: String,
    /// The ID of the plan / product / price.
    pub plan_id: String,
    /// The status of the subscription.
    pub status: SubscriptionStatus,
    /// Expiration / end date timestamp (if applicable).
    pub ends_at: Option<i64>,
}

/// Dynamic trait to handle billing provider interactions.
#[async_trait]
pub trait BillingProvider: Send + Sync {
    /// Return the name of the billing provider (e.g. "stripe", "lemonsqueezy").
    fn name(&self) -> &'static str;

    /// Create a checkout session URL for a customer.
    async fn create_checkout_session(
        &self,
        customer_email: &str,
        plan_id: &str,
        redirect_url: &str,
    ) -> Result<String, String>;

    /// Verify the signature and extract subscription data from webhook request.
    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, String>;
}

// ─── Utility Helpers ──────────────────────────────────────────────────────────

/// Helper to url-encode string values without relying on external dependencies.
fn url_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                let _ = std::fmt::Write::write_fmt(&mut encoded, format_args!("%{:02X}", b));
            }
        }
    }
    encoded
}

// ─── Stripe Provider Implementation ──────────────────────────────────────────

/// Billing provider implementation for Stripe.
pub struct StripeProvider {
    api_key: String,
    webhook_secret: String,
}

impl StripeProvider {
    /// Creates a new `StripeProvider` instance.
    pub fn new(api_key: String, webhook_secret: String) -> Self {
        Self {
            api_key,
            webhook_secret,
        }
    }

    /// Verifies the `Stripe-Signature` header signature.
    /// Stripe-Signature header looks like: `t=1492774577,v1=604956efe...`
    fn verify_signature(&self, payload: &[u8], signature_header: &str) -> Result<(), String> {
        if self.webhook_secret.is_empty() {
            return Ok(()); // Skip verification if secret not configured
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
            return Err("Invalid Stripe-Signature header format".to_string());
        }

        let sig_bytes =
            hex::decode(signature_hex).map_err(|e| format!("Invalid hex signature: {}", e))?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());
        let mut ctx = hmac::Context::with_key(&key);
        ctx.update(timestamp.as_bytes());
        ctx.update(b".");
        ctx.update(payload);

        let tag = ctx.sign();
        if tag.as_ref().ct_eq(&sig_bytes).unwrap_u8() == 0 {
            return Err("Stripe signature verification failed".to_string());
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
    ) -> Result<String, String> {
        if self.api_key.is_empty() || self.api_key.starts_with("mock_") {
            // High-fidelity Developer Experience fallback checkout mock
            return Ok(format!(
                "https://checkout.stripe.com/pay/mock_session?email={}&plan={}&redirect={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = reqwest::Client::new();

        // Construct the form body manually to avoid reqwest optional "form" dependency feature
        let body_str = format!(
            "mode=subscription&success_url={}&cancel_url={}&customer_email={}&line_items[0][price]={}&line_items[0][quantity]=1",
            url_encode(redirect_url),
            url_encode(redirect_url),
            url_encode(customer_email),
            url_encode(plan_id)
        );

        let res = client
            .post("https://api.stripe.com/v1/checkout/sessions")
            .basic_auth(&self.api_key, Some(""))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body_str)
            .send()
            .await
            .map_err(|e| format!("Stripe API connection failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Stripe API returned error {}: {}",
                status, err_text
            ));
        }

        #[derive(Deserialize)]
        struct StripeSession {
            url: String,
        }

        let session: StripeSession = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse Stripe session JSON: {}", e))?;

        Ok(session.url)
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, String> {
        let sig = headers
            .get("stripe-signature")
            .or_else(|| headers.get("Stripe-Signature"));

        if let Some(s) = sig {
            self.verify_signature(payload, s)?;
        } else if !self.webhook_secret.is_empty() {
            return Err("Missing stripe-signature header".to_string());
        }

        let val: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| format!("Invalid JSON payload: {}", e))?;

        let event_type = val["type"].as_str().unwrap_or("");
        if !event_type.starts_with("customer.subscription.") {
            return Err(format!("Uninteresting event type: {}", event_type));
        }

        let obj = &val["data"]["object"];
        let subscription_id = obj["id"].as_str().unwrap_or("").to_string();
        let customer_id = obj["customer"].as_str().unwrap_or("").to_string();
        let status_str = obj["status"].as_str().unwrap_or("");

        let plan_id = obj["items"]["data"][0]["price"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let ends_at = obj["current_period_end"].as_i64();

        // Try to fetch customer email if present, or fetch dummy
        let customer_email = obj["customer_details"]["email"]
            .as_str()
            .or_else(|| obj["email"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(WebhookEvent {
            subscription_id,
            customer_id,
            customer_email,
            plan_id,
            status: SubscriptionStatus::parse_status(status_str),
            ends_at,
        })
    }
}

// ─── LemonSqueezy Provider Implementation ────────────────────────────────────

/// Billing provider implementation for LemonSqueezy.
pub struct LemonSqueezyProvider {
    api_key: String,
    webhook_secret: String,
}

impl LemonSqueezyProvider {
    /// Creates a new `LemonSqueezyProvider` instance.
    pub fn new(api_key: String, webhook_secret: String) -> Self {
        Self {
            api_key,
            webhook_secret,
        }
    }

    /// Verifies the `X-Signature` header signature using HMAC-SHA256 of the raw body.
    fn verify_signature(&self, payload: &[u8], signature_hex: &str) -> Result<(), String> {
        if self.webhook_secret.is_empty() {
            return Ok(());
        }

        let sig_bytes =
            hex::decode(signature_hex).map_err(|e| format!("Invalid hex signature: {}", e))?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());

        hmac::verify(&key, payload, &sig_bytes)
            .map_err(|_| "LemonSqueezy signature verification failed".to_string())?;

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for LemonSqueezyProvider {
    fn name(&self) -> &'static str {
        "lemonsqueezy"
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn create_checkout_session(
        &self,
        customer_email: &str,
        plan_id: &str,
        redirect_url: &str,
    ) -> Result<String, String> {
        if self.api_key.is_empty() || self.api_key.starts_with("mock_") {
            // High-fidelity Developer Experience fallback checkout mock
            return Ok(format!(
                "https://checkout.lemonsqueezy.com/checkout/mock_session?email={}&variant={}&redirect={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = reqwest::Client::new();

        // We need the LemonSqueezy Store ID to create custom checkouts.
        // It can be passed or extracted. Let's look up the STORE_ID env var, default to a mock/1.
        let store_id = std::env::var("LEMONSQUEEZY_STORE_ID").unwrap_or_else(|_| "1".to_string());

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
                            "id": store_id
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
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/vnd.api+json")
            .header("Content-Type", "application/vnd.api+json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("LemonSqueezy API connection failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "LemonSqueezy API returned error {}: {}",
                status, err_text
            ));
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse LemonSqueezy checkout JSON: {}", e))?;

        let url = body["data"]["attributes"]["url"]
            .as_str()
            .ok_or_else(|| "Missing URL field in LemonSqueezy response attributes".to_string())?
            .to_string();

        Ok(url)
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, String> {
        let sig = headers
            .get("x-signature")
            .or_else(|| headers.get("X-Signature"));

        if let Some(signature_hex) = sig {
            self.verify_signature(payload, signature_hex)?;
        } else if !self.webhook_secret.is_empty() {
            return Err("Missing X-Signature header".to_string());
        }

        let val: serde_json::Value =
            serde_json::from_slice(payload).map_err(|e| format!("Invalid JSON payload: {}", e))?;

        let event_name = val["meta"]["event_name"].as_str().unwrap_or("");
        if !event_name.starts_with("subscription_") {
            return Err(format!("Uninteresting event name: {}", event_name));
        }

        let data = &val["data"];
        let subscription_id = data["id"].as_str().unwrap_or("").to_string();
        let attrs = &data["attributes"];

        let customer_id = attrs["customer_id"]
            .as_u64()
            .map(|id| id.to_string())
            .or_else(|| attrs["customer_id"].as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let customer_email = attrs["user_email"].as_str().unwrap_or("").to_string();
        let plan_id = attrs["variant_id"]
            .as_u64()
            .map(|id| id.to_string())
            .or_else(|| attrs["variant_id"].as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let status_str = attrs["status"].as_str().unwrap_or("");
        let ends_at = attrs["ends_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp());

        Ok(WebhookEvent {
            subscription_id,
            customer_id,
            customer_email,
            plan_id,
            status: SubscriptionStatus::parse_status(status_str),
            ends_at,
        })
    }
}

// Helper module for hex-encoding/decoding since hex crate is in target target_arch="wasm32" but we can implement it simply.
mod hex {
    #[cfg_attr(mutants, mutants::skip)]
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::with_capacity(s.len() / 2);
        let mut chars = s.chars();
        while let (Some(c1), Some(c2)) = (chars.next(), chars.next()) {
            let b1 = c1.to_digit(16).ok_or("Invalid hex character")? as u8;
            let b2 = c2.to_digit(16).ok_or("Invalid hex character")? as u8;
            bytes.push((b1 << 4) | b2);
        }
        Ok(bytes)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_stripe_provider() {
        let provider = StripeProvider::new("mock_key".to_string(), "mock_secret".to_string());
        assert_eq!(provider.name(), "stripe");

        let url = provider
            .create_checkout_session("test@user.com", "price_123", "https://app.com/success")
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
        assert!(url.contains("test%40user.com"));
    }

    #[tokio::test]
    async fn test_mock_lemonsqueezy_provider() {
        let provider = LemonSqueezyProvider::new("mock_key".to_string(), "mock_secret".to_string());
        assert_eq!(provider.name(), "lemonsqueezy");

        let url = provider
            .create_checkout_session("test@user.com", "456", "https://app.com/success")
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
        assert!(url.contains("test%40user.com"));
    }

    #[test]
    fn test_subscription_status_parsing() {
        assert_eq!(
            SubscriptionStatus::parse_status("active"),
            SubscriptionStatus::Active
        );
        assert_eq!(
            SubscriptionStatus::parse_status("Canceled"),
            SubscriptionStatus::Canceled
        );
        assert_eq!(
            SubscriptionStatus::parse_status("trialing"),
            SubscriptionStatus::Trialing
        );
        // Added for mutants
        assert_eq!(
            SubscriptionStatus::parse_status("past_due"),
            SubscriptionStatus::PastDue
        );
        assert_eq!(
            SubscriptionStatus::parse_status("unpaid"),
            SubscriptionStatus::Unpaid
        );
        assert_eq!(
            SubscriptionStatus::parse_status("paused"),
            SubscriptionStatus::Paused
        );
        assert_eq!(
            SubscriptionStatus::parse_status("unknown_garbage"),
            SubscriptionStatus::Unpaid
        );
    }

    #[test]
    fn test_subscription_status_as_str() {
        assert_eq!(SubscriptionStatus::Active.as_str(), "active");
        assert_eq!(SubscriptionStatus::Canceled.as_str(), "canceled");
        assert_eq!(SubscriptionStatus::PastDue.as_str(), "past_due");
        assert_eq!(SubscriptionStatus::Unpaid.as_str(), "unpaid");
        assert_eq!(SubscriptionStatus::Trialing.as_str(), "trialing");
        assert_eq!(SubscriptionStatus::Paused.as_str(), "paused");
    }

    #[test]
    fn test_hex_decode() {
        assert_eq!(
            hex::decode("00010a0f").unwrap(),
            vec![0x00, 0x01, 0x0a, 0x0f]
        );
        assert_eq!(hex::decode("ffFF").unwrap(), vec![0xff, 0xff]);
        assert!(hex::decode("zz").is_err());
    }

    #[test]
    #[cfg(not(miri))]
    fn test_stripe_signature_verification() {
        let provider = StripeProvider::new("mock".to_string(), "secret".to_string());

        // Missing stripe-signature header
        let mut headers = HashMap::new();
        let res = provider.handle_webhook(b"{}", &headers);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "Missing stripe-signature header");

        // Invalid signature format (missing parts)
        headers.insert("stripe-signature".to_string(), "invalid_format".to_string());
        let res2 = provider.handle_webhook(b"{}", &headers);
        assert!(res2.is_err());
        assert_eq!(res2.unwrap_err(), "Invalid Stripe-Signature header format");

        headers.insert("stripe-signature".to_string(), "t=123".to_string());
        let res_missing_v1 = provider.handle_webhook(b"{}", &headers);
        assert!(res_missing_v1.is_err());
        assert_eq!(
            res_missing_v1.unwrap_err(),
            "Invalid Stripe-Signature header format"
        );

        headers.insert("stripe-signature".to_string(), "v1=deadbeef".to_string());
        let res_missing_t = provider.handle_webhook(b"{}", &headers);
        assert!(res_missing_t.is_err());
        assert_eq!(
            res_missing_t.unwrap_err(),
            "Invalid Stripe-Signature header format"
        );

        // Valid timestamp but invalid hex characters
        headers.insert(
            "stripe-signature".to_string(),
            "t=123,v1=not_hex!!".to_string(),
        );
        let res3 = provider.handle_webhook(b"{}", &headers);
        assert!(res3.is_err());
        assert_eq!(
            res3.unwrap_err(),
            "Invalid hex signature: Invalid hex character"
        );

        // Hex decodes but doesn't match
        headers.insert(
            "stripe-signature".to_string(),
            "t=123,v1=deadbeef".to_string(),
        );
        let res4 = provider.handle_webhook(b"{}", &headers);
        assert!(res4.is_err());
        assert_eq!(res4.unwrap_err(), "Stripe signature verification failed");
    }

    #[test]
    fn test_stripe_signature_empty_secret() {
        let provider = StripeProvider::new("mock".to_string(), "".to_string());
        let res = provider.verify_signature(b"{}", "invalid_signature");
        assert!(res.is_ok());
    }

    #[test]
    #[cfg(not(miri))]
    fn test_lemonsqueezy_signature_verification() {
        let provider = LemonSqueezyProvider::new("mock".to_string(), "secret".to_string());

        // Missing x-signature
        let mut headers = HashMap::new();
        let res = provider.handle_webhook(b"{}", &headers);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "Missing X-Signature header");

        // Invalid signature hex
        headers.insert("x-signature".to_string(), "invalid".to_string());
        let res2 = provider.handle_webhook(b"{}", &headers);
        assert!(res2.is_err());
        assert_eq!(
            res2.unwrap_err(),
            "Invalid hex signature: Invalid hex character"
        );

        // Hex decodes but doesn't match
        headers.insert("x-signature".to_string(), "deadbeef".to_string());
        let res3 = provider.handle_webhook(b"{}", &headers);
        assert!(res3.is_err());
        assert_eq!(
            res3.unwrap_err(),
            "LemonSqueezy signature verification failed"
        );
    }
}
