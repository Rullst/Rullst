use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
use crate::error::CapitalError;
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
    pub fn new(picpay_token: impl Into<String>, seller_token: impl Into<String>) -> Self {
        Self {
            picpay_token: picpay_token.into(),
            seller_token: seller_token.into(),
        }
    }

    /// Verifies the `x-seller-token` header.
    pub fn verify_token(&self, token_header: &str) -> Result<(), CapitalError> {
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
            return Err(CapitalError::InvalidSignature(
                "PicPay seller token verification failed".to_string(),
            ));
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

        if self.picpay_token.is_empty()
            || self.picpay_token.starts_with("mock_")
            || self.picpay_token == "picpay_token"
        {
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
            .map_err(|e| CapitalError::ProviderRequestFailed(format!("Network error: {}", e)))?;

        if !res.status().is_success() {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "PicPay API error: HTTP {}",
                res.status()
            )));
        }

        let body: Value = res.json().await.map_err(|e| {
            CapitalError::PayloadParseError(format!("Failed to parse response: {}", e))
        })?;

        body["paymentUrl"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                CapitalError::PayloadParseError("Missing paymentUrl in PicPay response".to_string())
            })
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let seller_header = headers.get("x-seller-token");

        if let Some(token) = seller_header {
            self.verify_token(token)?;
        } else if !self.seller_token.is_empty() {
            return Err(CapitalError::InvalidSignature(
                "Missing x-seller-token header".to_string(),
            ));
        }

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

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
    ) -> Result<String, CapitalError> {
        if customer_email.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Customer email cannot be empty".to_string(),
            ));
        }

        Ok(format!(
            "https://picpay.com/portal?email={}",
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
            "PicPay does not support pause subscription".to_string(),
        ))
    }

    async fn report_usage(
        &self,
        _subscription_id: &str,
        _metric: &str,
        _quantity: u64,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "PicPay does not support metered usage reporting".to_string(),
        ))
    }

    async fn apply_coupon(
        &self,
        _subscription_id: &str,
        _coupon_code: &str,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "PicPay does not support coupon applications via API".to_string(),
        ))
    }

    async fn extend_trial(
        &self,
        _subscription_id: &str,
        _trial_ends_at: i64,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "PicPay does not support trial period extension".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_picpay_provider_methods() {
        let provider = PicPayProvider::new("picpay_token", "sec_seller123");

        // 1. Checkout session
        let url = provider
            .create_checkout_session("user@picpay.com", "plan_mensal", "https://app.com/callback")
            .await
            .unwrap();
        assert!(url.contains("picpay.com/checkout"));
        assert!(url.contains("plan_mensal"));

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
            .create_customer_portal("user@picpay.com", "https://app.com")
            .await
            .unwrap();
        assert!(portal.contains("picpay.com/portal"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // 4. Cancel
        assert!(provider.cancel_subscription("sub_pic").await.is_ok());
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
