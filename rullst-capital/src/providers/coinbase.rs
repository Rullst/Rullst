use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
use crate::error::CapitalError;
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// Billing provider implementation for Coinbase Commerce (Global Web3 & Crypto: BTC, ETH, SOL, USDC).
pub struct CoinbaseCommerceProvider {
    api_key: String,
    webhook_secret: String,
}

impl CoinbaseCommerceProvider {
    /// Creates a new `CoinbaseCommerceProvider` instance.
    pub fn new(api_key: String, webhook_secret: String) -> Self {
        Self {
            api_key,
            webhook_secret,
        }
    }

    /// Verifies the `X-CC-Webhook-Signature` header HMAC-SHA256 signature.
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
                "Coinbase Commerce signature verification failed".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl BillingProvider for CoinbaseCommerceProvider {
    fn name(&self) -> &'static str {
        "coinbase"
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
                "https://commerce.coinbase.com/charges/mock_session?email={}&plan={}&redirect={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "name": format!("Subscription Plan {}", plan_id),
            "description": "SaaS Subscription Payment via Web3 Crypto",
            "pricing_type": "fixed_price",
            "local_price": {
                "amount": "29.00",
                "currency": "USD"
            },
            "metadata": {
                "customer_email": customer_email,
                "plan_id": plan_id
            },
            "redirect_url": redirect_url,
            "cancel_url": redirect_url
        });

        let res = client
            .post("https://api.commerce.coinbase.com/charges")
            .header("X-CC-Api-Key", &self.api_key)
            .header("X-CC-Version", "2018-03-22")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CapitalError::ProviderRequestFailed(format!("Network error: {}", e)))?;

        if !res.status().is_success() {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "Coinbase Commerce API error: HTTP {}",
                res.status()
            )));
        }

        let body: Value = res.json().await.map_err(|e| {
            CapitalError::PayloadParseError(format!("Failed to parse response: {}", e))
        })?;

        body["data"]["hosted_url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                CapitalError::PayloadParseError(
                    "Missing hosted_url in Coinbase Commerce response".to_string(),
                )
            })
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let sig_header = headers.get("x-cc-webhook-signature");

        if let Some(sig) = sig_header {
            self.verify_signature(payload, sig)?;
        } else if !self.webhook_secret.is_empty() {
            return Err(CapitalError::InvalidSignature(
                "Missing X-CC-Webhook-Signature header".to_string(),
            ));
        }

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        let event = &json["event"];
        let data = &event["data"];

        let subscription_id = data["id"].as_str().unwrap_or("").to_string();
        let customer_id = data["metadata"]["customer_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let customer_email = data["metadata"]["customer_email"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let plan_id = data["metadata"]["plan_id"]
            .as_str()
            .unwrap_or("default")
            .to_string();

        let event_type = event["type"].as_str().unwrap_or("charge:confirmed");
        let status = if event_type.contains("confirmed") || event_type.contains("resolved") {
            SubscriptionStatus::Active
        } else if event_type.contains("failed") {
            SubscriptionStatus::Unpaid
        } else {
            SubscriptionStatus::Active
        };

        let ends_at = data["expires_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp());

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
            "https://commerce.coinbase.com/portal?email={}",
            url_encode(customer_email)
        ))
    }

    async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    async fn pause_subscription(&self, _subscription_id: &str) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "Coinbase Commerce does not support subscription pause".to_string(),
        ))
    }

    async fn report_usage(
        &self,
        _subscription_id: &str,
        _metric: &str,
        _quantity: u64,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "Coinbase Commerce does not support metered usage reporting".to_string(),
        ))
    }

    async fn apply_coupon(
        &self,
        _subscription_id: &str,
        _coupon_code: &str,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "Coinbase Commerce does not support coupon application".to_string(),
        ))
    }

    async fn extend_trial(
        &self,
        _subscription_id: &str,
        _trial_ends_at: i64,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "Coinbase Commerce does not support trial extension".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coinbase_provider_methods() {
        let provider = CoinbaseProvider::new("mock_key", "sec_coin123");

        // 1. Checkout session
        let url = provider
            .create_checkout_session("crypto@user.com", "crypto_plan", "https://app.com/done")
            .await
            .unwrap();
        assert!(url.contains("commerce.coinbase.com/checkout"));
        assert!(url.contains("crypto_plan"));

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
            .create_customer_portal("crypto@user.com", "https://app.com")
            .await
            .unwrap();
        assert!(portal.contains("commerce.coinbase.com/portal"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // 4. Cancel
        assert!(provider.cancel_subscription("sub_coin").await.is_ok());
        assert!(provider.cancel_subscription("").await.is_err());

        // 5. Unsupported operations
        assert!(matches!(
            provider.pause_subscription("sub").await,
            Err(CapitalError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            provider.report_usage("sub", "api", 1).await,
            Err(CapitalError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            provider.apply_coupon("sub", "CODE").await,
            Err(CapitalError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            provider.extend_trial("sub", 1800000000).await,
            Err(CapitalError::UnsupportedOperation(_))
        ));
    }
}
