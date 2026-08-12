use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
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
    pub fn verify_signature(&self, payload: &[u8], signature_header: &str) -> Result<(), String> {
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
            return Err("Invalid Paddle-Signature header format".to_string());
        }

        let sig_bytes =
            hex::decode(signature_hex).map_err(|e| format!("Invalid hex signature: {}", e))?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());
        let mut ctx = hmac::Context::with_key(&key);
        ctx.update(timestamp.as_bytes());
        ctx.update(b":");
        ctx.update(payload);

        let tag = ctx.sign();
        if tag.as_ref().ct_eq(&sig_bytes).unwrap_u8() == 0 {
            return Err("Paddle signature verification failed".to_string());
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
    ) -> Result<String, String> {
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
            .map_err(|e| format!("Network error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Paddle API error: HTTP {}", res.status()));
        }

        let body: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        body["data"]["checkout"]["url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing checkout URL in Paddle response".to_string())
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, String> {
        let sig_header = headers
            .get("paddle-signature")
            .ok_or_else(|| "Missing paddle-signature header".to_string())?;

        self.verify_signature(payload, sig_header)?;

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| format!("Invalid JSON payload: {}", e))?;

        let data = &json["data"];
        let subscription_id = data["id"].as_str().unwrap_or("").to_string();
        let customer_id = data["customer_id"].as_str().unwrap_or("").to_string();
        let customer_email = data["customer"]["email"]
            .as_str()
            .unwrap_or("")
            .to_string();

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
    ) -> Result<String, String> {
        Ok(format!(
            "https://paddle.com/portal?email={}",
            url_encode(customer_email)
        ))
    }

    async fn cancel_subscription(&self, _subscription_id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn pause_subscription(&self, _subscription_id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn report_usage(
        &self,
        _subscription_id: &str,
        _metric: &str,
        _quantity: u64,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn apply_coupon(&self, _subscription_id: &str, _coupon_code: &str) -> Result<(), String> {
        Ok(())
    }

    async fn extend_trial(&self, _subscription_id: &str, _trial_ends_at: i64) -> Result<(), String> {
        Ok(())
    }
}
