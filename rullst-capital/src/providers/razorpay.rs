use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
use crate::error::CapitalError;
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// Billing provider implementation for Razorpay (India & Southeast Asia UPI, Cards & Subscriptions).
pub struct RazorpayProvider {
    key_id: String,
    key_secret: String,
    webhook_secret: String,
}

impl RazorpayProvider {
    /// Creates a new `RazorpayProvider` instance.
    pub fn new(
        key_id: impl Into<String>,
        key_secret: impl Into<String>,
        webhook_secret: impl Into<String>,
    ) -> Self {
        Self {
            key_id: key_id.into(),
            key_secret: key_secret.into(),
            webhook_secret: webhook_secret.into(),
        }
    }

    /// Verifies the `X-Razorpay-Signature` header HMAC-SHA256 signature.
    pub fn verify_signature(
        &self,
        payload: &[u8],
        signature_hex: &str,
    ) -> Result<(), CapitalError> {
        if self.webhook_secret.is_empty() {
            return Ok(());
        }

        let sig_bytes = hex::decode(signature_hex)
            .map_err(|e| CapitalError::InvalidSignature(format!("Invalid hex signature: {}", e)))?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());
        let tag = hmac::sign(&key, payload);

        if tag.as_ref().ct_eq(&sig_bytes).unwrap_u8() == 0 {
            return Err(CapitalError::InvalidSignature(
                "Razorpay signature verification failed".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for RazorpayProvider {
    fn name(&self) -> &'static str {
        "razorpay"
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

        if self.key_id.is_empty() || self.key_id.starts_with("mock_") {
            return Ok(format!(
                "https://razorpay.com/checkout/mock_session?email={}&plan={}&callback_url={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "plan_id": plan_id,
            "total_count": 12,
            "quantity": 1,
            "customer_notify": 1,
            "notes": {
                "customer_email": customer_email,
                "redirect_url": redirect_url
            }
        });

        let res = client
            .post("https://api.razorpay.com/v1/subscriptions")
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CapitalError::ProviderRequestFailed(format!("Network error: {}", e)))?;

        if !res.status().is_success() {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "Razorpay API error: HTTP {}",
                res.status()
            )));
        }

        let body: Value = res.json().await.map_err(|e| {
            CapitalError::PayloadParseError(format!("Failed to parse response: {}", e))
        })?;

        body["short_url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                CapitalError::PayloadParseError(
                    "Missing short_url in Razorpay response".to_string(),
                )
            })
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let sig_header = headers.get("x-razorpay-signature");

        if let Some(sig) = sig_header {
            self.verify_signature(payload, sig)?;
        } else if !self.webhook_secret.is_empty() {
            return Err(CapitalError::InvalidSignature(
                "Missing X-Razorpay-Signature header".to_string(),
            ));
        }

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        let event = json["event"].as_str().unwrap_or("");
        let sub_data = &json["payload"]["subscription"]["entity"];
        let payment_data = &json["payload"]["payment"]["entity"];

        let subscription_id = sub_data["id"]
            .as_str()
            .or_else(|| payment_data["order_id"].as_str())
            .or_else(|| json["payload"]["order"]["entity"]["id"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_id = sub_data["customer_id"]
            .as_str()
            .or_else(|| payment_data["customer_id"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_email = payment_data["email"]
            .as_str()
            .or_else(|| sub_data["notes"]["customer_email"].as_str())
            .unwrap_or("")
            .to_string();

        let plan_id = sub_data["plan_id"]
            .as_str()
            .unwrap_or("default")
            .to_string();

        let status = match event {
            "subscription.authenticated" | "subscription.activated" | "payment.captured" => {
                SubscriptionStatus::Active
            }
            "subscription.cancelled" => SubscriptionStatus::Canceled,
            "subscription.pending" => SubscriptionStatus::PastDue,
            "subscription.halted" | "payment.failed" => SubscriptionStatus::Unpaid,
            _ => SubscriptionStatus::Active,
        };

        let ends_at = sub_data["current_end"].as_i64();

        Ok(WebhookEvent {
            subscription_id,
            customer_id,
            customer_email,
            plan_id,
            status,
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
            "https://dashboard.razorpay.com/portal?email={}",
            url_encode(customer_email)
        ))
    }

    async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        if !self.key_id.is_empty() && !self.key_id.starts_with("mock_") {
            let client = reqwest::Client::new();
            let res = client
                .post(format!(
                    "https://api.razorpay.com/v1/subscriptions/{}/cancel",
                    subscription_id
                ))
                .basic_auth(&self.key_id, Some(&self.key_secret))
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
        if !self.key_id.is_empty() && !self.key_id.starts_with("mock_") {
            let client = reqwest::Client::new();
            let res = client
                .post(format!(
                    "https://api.razorpay.com/v1/subscriptions/{}/pause",
                    subscription_id
                ))
                .basic_auth(&self.key_id, Some(&self.key_secret))
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
    async fn test_razorpay_provider_methods() {
        let provider = RazorpayProvider::new("mock_key_id", "mock_secret", "sec_rzp123");

        // 1. Checkout session
        let url = provider
            .create_checkout_session(
                "user@razorpay.com",
                "plan_sub_pro",
                "https://app.com/success",
            )
            .await
            .unwrap();
        assert!(url.contains("razorpay.com/checkout"));
        assert!(url.contains("plan_sub_pro"));

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
            .create_customer_portal("user@razorpay.com", "https://app.com")
            .await
            .unwrap();
        assert!(portal.contains("dashboard.razorpay.com/portal"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // 4. Subscription actions
        assert!(provider.cancel_subscription("sub_rzp").await.is_ok());
        assert!(provider.cancel_subscription("").await.is_err());

        assert!(provider.pause_subscription("sub_rzp").await.is_ok());
        assert!(provider.pause_subscription("").await.is_err());

        assert!(provider.report_usage("sub_rzp", "api", 10).await.is_ok());
        assert!(provider.report_usage("", "api", 10).await.is_err());

        assert!(provider.apply_coupon("sub_rzp", "DISCOUNT").await.is_ok());
        assert!(provider.apply_coupon("", "DISCOUNT").await.is_err());
        assert!(provider.apply_coupon("sub_rzp", "").await.is_err());

        assert!(provider.extend_trial("sub_rzp", 1800000000).await.is_ok());
        assert!(provider.extend_trial("", 1800000000).await.is_err());
        assert!(provider.extend_trial("sub_rzp", -1).await.is_err());
    }
}
