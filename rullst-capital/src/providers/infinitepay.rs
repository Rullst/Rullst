use super::{
    BillingProvider, SubscriptionStatus, WebhookEvent, WebhookVerificationMode, url_encode,
    verify_explicit_mock_signature, webhook_mode_from_secret,
};
use crate::error::CapitalError;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Billing provider adapter for InfinitePay (CloudWalk Brazil).
///
/// Fees, settlement schedules, installment terms, geography, and merchant
/// eligibility belong to the current provider contract and account.
pub struct InfinitePayProvider {
    api_key: String,
    webhook_secret: String,
}

impl InfinitePayProvider {
    /// Creates a new `InfinitePayProvider` instance.
    pub fn new(api_key: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            webhook_secret: webhook_secret.into(),
        }
    }

    /// Retained for source compatibility; live callback authentication is unavailable.
    ///
    /// The documented checkout callback and payment lookup need a separately
    /// reviewed merchant/order/amount contract. A locally generated HMAC does
    /// not establish that InfinitePay authenticates this body-only protocol.
    pub fn verify_signature(
        &self,
        _payload: &[u8],
        signature_hex: &str,
    ) -> Result<(), CapitalError> {
        if self.webhook_verification_mode()? == WebhookVerificationMode::Mock {
            return verify_explicit_mock_signature(
                self.name(),
                &self.webhook_secret,
                signature_hex,
            );
        }

        Err(live_webhook_unavailable())
    }
}

#[async_trait]
impl BillingProvider for InfinitePayProvider {
    fn name(&self) -> &'static str {
        "infinitepay"
    }

    fn webhook_verification_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        webhook_mode_from_secret(self.name(), &self.webhook_secret)
    }

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
                "https://checkout.infinitepay.io/pay/mock_session?email={}&plan={}&redirect={}&handle={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url),
                url_encode(&self.api_key)
            ));
        }

        Err(CapitalError::UnsupportedOperation(
            "infinitepay plan-only checkout has no reviewed authoritative pricing contract".into(),
        ))
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        if self.webhook_verification_mode()? == WebhookVerificationMode::Real {
            return Err(live_webhook_unavailable());
        }
        let sig_header = headers
            .get("x-signature")
            .or_else(|| headers.get("x-infinitepay-signature"))
            .ok_or_else(|| {
                CapitalError::InvalidSignature("Missing X-Signature header".to_string())
            })?;
        self.verify_signature(payload, sig_header)?;

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        let subscription_id = json["id"]
            .as_str()
            .or_else(|| json["transaction_id"].as_str())
            .or_else(|| json["data"]["id"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_id = json["customer"]["id"]
            .as_str()
            .or_else(|| json["customer_id"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_email = json["customer"]["email"]
            .as_str()
            .or_else(|| json["customer_email"].as_str())
            .unwrap_or("")
            .to_string();

        let plan_id = json["plan_id"]
            .as_str()
            .or_else(|| json["order_id"].as_str())
            .unwrap_or("default")
            .to_string();

        let status_str = json["status"]
            .as_str()
            .or_else(|| json["event"].as_str())
            .filter(|status| !status.trim().is_empty())
            .ok_or_else(|| {
                CapitalError::PayloadParseError("Webhook status is missing or invalid".into())
            })?;

        let ends_at = json["ends_at"].as_i64();

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
    ) -> Result<String, CapitalError> {
        if customer_email.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Customer email cannot be empty".to_string(),
            ));
        }

        super::require_mock_operation(&self.api_key, self.name(), "create customer portal")?;

        Ok(format!(
            "https://app.infinitepay.io/client-portal?email={}",
            url_encode(customer_email)
        ))
    }

    async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        super::require_mock_operation(&self.api_key, self.name(), "cancel subscription")?;

        Ok(())
    }

    async fn pause_subscription(&self, _subscription_id: &str) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "InfinitePay does not support subscription pause".to_string(),
        ))
    }

    async fn report_usage(
        &self,
        _subscription_id: &str,
        _metric: &str,
        _quantity: u64,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "InfinitePay does not support metered usage reporting".to_string(),
        ))
    }

    async fn apply_coupon(
        &self,
        _subscription_id: &str,
        _coupon_code: &str,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "InfinitePay does not support coupon application".to_string(),
        ))
    }

    async fn extend_trial(
        &self,
        _subscription_id: &str,
        _trial_ends_at: i64,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "InfinitePay does not support trial extension".to_string(),
        ))
    }
}

fn live_webhook_unavailable() -> CapitalError {
    CapitalError::UnsupportedOperation(
        "InfinitePay live callbacks require reviewed authentication and authoritative payment lookup".into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::hmac;

    #[tokio::test]
    async fn test_infinitepay_provider_methods() {
        let provider = InfinitePayProvider::new("mock_key", "sec_inf123");
        assert_eq!(provider.name(), "infinitepay");

        // 1. Checkout session
        let url = provider
            .create_checkout_session(
                "user@infinite.com",
                "plan_starter",
                "https://app.com/callback",
            )
            .await
            .unwrap();
        assert!(url.contains("infinitepay.io/pay"));

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
            .create_customer_portal("user@infinite.com", "https://app.com")
            .await
            .unwrap();
        assert!(portal.contains("client-portal"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // 4. Cancel
        assert!(provider.cancel_subscription("sub_inf1").await.is_ok());
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

        // 6. Signature verification
        let secret = "sec_inf123";
        let payload = br#"{"id":"tx_inf_100","customer":{"id":"c1","email":"user@infinite.com"},"plan_id":"plan_starter","status":"paid"}"#;

        let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
        let sig = hmac::sign(&key, payload);
        let sig_hex = hex::encode(sig.as_ref());

        assert!(matches!(
            provider.verify_signature(payload, &sig_hex),
            Err(CapitalError::UnsupportedOperation(_))
        ));

        // Signature error paths
        let no_sec = InfinitePayProvider::new("k", "");
        assert!(matches!(
            no_sec.verify_signature(payload, ""),
            Err(CapitalError::ConfigurationError(_))
        ));
        assert!(provider.verify_signature(payload, "invalid_hex!").is_err());
        assert!(
            provider
                .verify_signature(
                    payload,
                    "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
                )
                .is_err()
        );

        // 7. Handle webhook
        let provider = InfinitePayProvider::new("mock_key", "mock_webhook");
        let mut headers = HashMap::new();
        headers.insert("x-signature".to_string(), "mock_webhook".into());

        let event = provider.handle_webhook(payload, &headers).unwrap();
        assert_eq!(event.subscription_id, "tx_inf_100");
        assert_eq!(event.customer_email, "user@infinite.com");
        assert_eq!(event.status, SubscriptionStatus::Active);

        // Webhook error paths
        let empty_headers = HashMap::new();
        assert!(provider.handle_webhook(payload, &empty_headers).is_err());
        assert!(provider.handle_webhook(b"invalid json", &headers).is_err());
    }
}
