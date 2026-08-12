use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
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
    pub fn verify_signature(&self, payload: &[u8], signature_hex: &str) -> Result<(), String> {
        if self.webhook_secret.is_empty() {
            return Ok(());
        }

        let sig_bytes =
            hex::decode(signature_hex).map_err(|e| format!("Invalid hex signature: {}", e))?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());
        let tag = hmac::sign(&key, payload);

        if tag.as_ref().ct_eq(&sig_bytes).unwrap_u8() == 0 {
            return Err("Polar.sh signature verification failed".to_string());
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
    ) -> Result<String, String> {
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
            .map_err(|e| format!("Network error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Polar API error: HTTP {}", res.status()));
        }

        let body: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        body["url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing checkout URL in Polar.sh response".to_string())
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, String> {
        let sig_header = headers
            .get("polar-signature")
            .or_else(|| headers.get("webhook-signature"));

        if let Some(sig) = sig_header {
            self.verify_signature(payload, sig)?;
        } else if !self.webhook_secret.is_empty() {
            return Err("Missing Polar-Signature header".to_string());
        }

        let json: Value =
            serde_json::from_slice(payload).map_err(|e| format!("Invalid JSON payload: {}", e))?;

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
    ) -> Result<String, String> {
        Ok(format!(
            "https://polar.sh/purchases?email={}",
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

    async fn extend_trial(
        &self,
        _subscription_id: &str,
        _trial_ends_at: i64,
    ) -> Result<(), String> {
        Ok(())
    }
}
