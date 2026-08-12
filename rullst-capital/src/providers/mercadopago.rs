use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// Billing provider implementation for Mercado Pago (LATAM Regional Subscriptions & Checkout).
pub struct MercadoPagoProvider {
    access_token: String,
    webhook_secret: String,
}

impl MercadoPagoProvider {
    /// Creates a new `MercadoPagoProvider` instance.
    pub fn new(access_token: String, webhook_secret: String) -> Self {
        Self {
            access_token,
            webhook_secret,
        }
    }

    /// Verifies the `x-signature` header (`ts=...;v1=...`).
    pub fn verify_signature(&self, payload: &[u8], signature_header: &str) -> Result<(), String> {
        if self.webhook_secret.is_empty() {
            return Ok(());
        }

        let mut timestamp = "";
        let mut signature_hex = "";

        for part in signature_header.split(',') {
            let mut kv = part.splitn(2, '=');
            let k = kv.next().unwrap_or("").trim();
            let v = kv.next().unwrap_or("").trim();
            if k == "ts" {
                timestamp = v;
            } else if k == "v1" {
                signature_hex = v;
            }
        }

        if timestamp.is_empty() || signature_hex.is_empty() {
            return Err("Invalid x-signature header format".to_string());
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
            return Err("Mercado Pago signature verification failed".to_string());
        }

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for MercadoPagoProvider {
    fn name(&self) -> &'static str {
        "mercadopago"
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn create_checkout_session(
        &self,
        customer_email: &str,
        plan_id: &str,
        redirect_url: &str,
    ) -> Result<String, String> {
        if self.access_token.is_empty() || self.access_token.starts_with("mock_") {
            return Ok(format!(
                "https://www.mercadopago.com/checkout/mock_session?email={}&plan={}&back_url={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "items": [{
                "title": format!("Subscription Plan {}", plan_id),
                "quantity": 1,
                "unit_price": 50.0,
                "currency_id": "BRL"
            }],
            "payer": {
                "email": customer_email
            },
            "back_urls": {
                "success": redirect_url,
                "failure": redirect_url,
                "pending": redirect_url
            },
            "auto_return": "approved"
        });

        let res = client
            .post("https://api.mercadopago.com/checkout/preferences")
            .bearer_auth(&self.access_token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Mercado Pago API error: HTTP {}", res.status()));
        }

        let body: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        body["init_point"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing init_point in Mercado Pago response".to_string())
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, String> {
        let sig_header = headers.get("x-signature");

        if let Some(sig) = sig_header {
            self.verify_signature(payload, sig)?;
        } else if !self.webhook_secret.is_empty() {
            return Err("Missing x-signature header".to_string());
        }

        let json: Value =
            serde_json::from_slice(payload).map_err(|e| format!("Invalid JSON payload: {}", e))?;

        let data = &json["data"];
        let subscription_id = data["id"]
            .as_str()
            .or_else(|| json["id"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_id = data["payer_id"]
            .as_str()
            .or_else(|| json["user_id"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_email = data["email"]
            .as_str()
            .or_else(|| json["email"].as_str())
            .unwrap_or("")
            .to_string();

        let plan_id = data["plan_id"]
            .as_str()
            .or_else(|| json["action"].as_str())
            .unwrap_or("default")
            .to_string();

        let status_str = data["status"]
            .as_str()
            .or_else(|| json["type"].as_str())
            .unwrap_or("approved");

        let ends_at = data["next_payment_date"]
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

    async fn create_customer_portal(
        &self,
        customer_email: &str,
        _return_url: &str,
    ) -> Result<String, String> {
        Ok(format!(
            "https://www.mercadopago.com/subscriptions?email={}",
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
