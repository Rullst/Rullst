use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// Billing provider implementation for PicPay (Brazil E-Commerce & Digital Wallet).
pub struct PicPayProvider {
    picpay_token: String,
    seller_token: String,
}

impl PicPayProvider {
    /// Creates a new `PicPayProvider` instance.
    pub fn new(picpay_token: String, seller_token: String) -> Self {
        Self {
            picpay_token,
            seller_token,
        }
    }

    /// Verifies the `x-seller-token` header.
    pub fn verify_token(&self, token_header: &str) -> Result<(), String> {
        if self.seller_token.is_empty() {
            return Ok(());
        }

        if self
            .seller_token
            .as_bytes()
            .ct_eq(token_header.as_bytes())
            .unwrap_u8()
            == 0
        {
            return Err("PicPay seller token verification failed".to_string());
        }

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for PicPayProvider {
    fn name(&self) -> &'static str {
        "picpay"
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn create_checkout_session(
        &self,
        customer_email: &str,
        plan_id: &str,
        redirect_url: &str,
    ) -> Result<String, String> {
        if self.picpay_token.is_empty() || self.picpay_token.starts_with("mock_") {
            return Ok(format!(
                "https://app.picpay.com/checkout/mock_session?email={}&plan={}&return_url={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "referenceId": format!("sub_{}", plan_id),
            "callbackUrl": redirect_url,
            "returnUrl": redirect_url,
            "value": 49.90,
            "buyer": {
                "email": customer_email,
                "firstName": "Customer",
                "lastName": "User"
            }
        });

        let res = client
            .post("https://appws.picpay.com/ecommerce/public/payments")
            .header("x-picpay-token", &self.picpay_token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("PicPay API error: HTTP {}", res.status()));
        }

        let body: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        body["paymentUrl"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing paymentUrl in PicPay response".to_string())
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, String> {
        let seller_header = headers.get("x-seller-token");

        if let Some(token) = seller_header {
            self.verify_token(token)?;
        } else if !self.seller_token.is_empty() {
            return Err("Missing x-seller-token header".to_string());
        }

        let json: Value =
            serde_json::from_slice(payload).map_err(|e| format!("Invalid JSON payload: {}", e))?;

        let subscription_id = json["referenceId"]
            .as_str()
            .or_else(|| json["authorizationId"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_id = json["buyer"]["document"]
            .as_str()
            .or_else(|| json["buyer"]["email"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_email = json["buyer"]["email"].as_str().unwrap_or("").to_string();
        let plan_id = json["referenceId"]
            .as_str()
            .unwrap_or("default")
            .to_string();
        let status_str = json["status"].as_str().unwrap_or("paid");

        Ok(WebhookEvent {
            subscription_id,
            customer_id,
            customer_email,
            plan_id,
            status: SubscriptionStatus::parse_status(status_str),
            ends_at: None,
        })
    }

    async fn create_customer_portal(
        &self,
        customer_email: &str,
        _return_url: &str,
    ) -> Result<String, String> {
        Ok(format!(
            "https://picpay.com/minha-conta?email={}",
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
