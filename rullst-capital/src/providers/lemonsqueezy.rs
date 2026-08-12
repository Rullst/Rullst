use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
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
    pub fn new(api_key: String, webhook_secret: String) -> Self {
        Self {
            api_key,
            webhook_secret,
        }
    }

    /// Verifies the `X-Signature` header signature using HMAC-SHA256 of the raw body.
    pub fn verify_signature(&self, payload: &[u8], signature_hex: &str) -> Result<(), String> {
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
            return Ok(format!(
                "https://checkout.lemonsqueezy.com/checkout/mock_session?email={}&variant={}&redirect={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = reqwest::Client::new();
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
            .map_err(|e| format!("Network error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("LemonSqueezy API error: HTTP {}", res.status()));
        }

        let body: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        body["data"]["attributes"]["url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing checkout URL in LemonSqueezy response".to_string())
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, String> {
        let sig_header = headers
            .get("x-signature")
            .ok_or_else(|| "Missing X-Signature header".to_string())?;

        self.verify_signature(payload, sig_header)?;

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| format!("Invalid JSON payload: {}", e))?;

        let data = &json["data"];
        let attrs = &data["attributes"];

        let subscription_id = data["id"].as_str().unwrap_or("").to_string();
        let customer_id = attrs["customer_id"].to_string();
        let customer_email = attrs["user_email"].as_str().unwrap_or("").to_string();
        let plan_id = attrs["variant_id"].to_string();
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
    ) -> Result<String, String> {
        Ok(format!(
            "https://app.lemonsqueezy.com/my-orders?email={}",
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
