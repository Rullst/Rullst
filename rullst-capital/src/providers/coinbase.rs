use super::{
    BillingProvider, SubscriptionStatus, WebhookEvent, WebhookVerificationMode, url_encode,
    verify_explicit_mock_signature, webhook_mode_from_secret,
};
use crate::error::CapitalError;
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// Type alias for `CoinbaseCommerceProvider`.
pub type CoinbaseProvider = CoinbaseCommerceProvider;

/// Billing provider implementation for Coinbase Commerce (Global Web3 & Crypto: BTC, ETH, SOL, USDC).
pub struct CoinbaseCommerceProvider {
    api_key: String,
    webhook_secret: String,
}

impl CoinbaseCommerceProvider {
    /// Creates a new `CoinbaseCommerceProvider` instance.
    pub fn new(api_key: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            webhook_secret: webhook_secret.into(),
        }
    }

    /// Verifies the `X-CC-Webhook-Signature` header HMAC-SHA256 signature.
    pub fn verify_signature(
        &self,
        payload: &[u8],
        signature_hex: &str,
    ) -> Result<(), CapitalError> {
        if self.webhook_verification_mode()? == WebhookVerificationMode::Mock {
            return verify_explicit_mock_signature(
                self.name(),
                &self.webhook_secret,
                signature_hex,
            );
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
                "https://commerce.coinbase.com/checkout/mock_session?email={}&plan={}&redirect={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        Err(CapitalError::UnsupportedOperation(
            "coinbase plan-only checkout has no reviewed authoritative pricing contract".into(),
        ))
    }

    /// Verifies and normalizes a signed one-off charge notification.
    ///
    /// A Coinbase charge is not a subscription: `subscription_id` carries the
    /// charge ID and `ends_at` is always `None`, because the charge's
    /// `expires_at` is its payment window, not an entitlement period. A
    /// confirmed or resolved charge requires the application's own
    /// `metadata.customer_id` and `metadata.plan_id`; nothing is defaulted. The
    /// settled amount and currency are not bound here, so the host must check
    /// `pricing`/`payments` against its own order before granting access.
    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let _ = self.webhook_verification_mode()?;
        let sig_header = headers.get("x-cc-webhook-signature").ok_or_else(|| {
            CapitalError::InvalidSignature("Missing X-CC-Webhook-Signature header".to_string())
        })?;
        self.verify_signature(payload, sig_header)?;

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        let event = &json["event"];
        let data = &event["data"];

        let status = match event["type"].as_str() {
            Some("charge:confirmed" | "charge:resolved") => SubscriptionStatus::Active,
            Some("charge:failed") => SubscriptionStatus::Unpaid,
            _ => {
                return Err(CapitalError::PayloadParseError(
                    "Unsupported Coinbase event".into(),
                ));
            }
        };
        let metadata = &data["metadata"];
        let subscription_id = charge_identity(&data["id"], "charge ID")?;
        let (customer_id, plan_id) = if status == SubscriptionStatus::Active {
            (
                charge_identity(&metadata["customer_id"], "metadata.customer_id")?,
                charge_identity(&metadata["plan_id"], "metadata.plan_id")?,
            )
        } else {
            (
                optional_charge_identity(&metadata["customer_id"], "metadata.customer_id")?,
                optional_charge_identity(&metadata["plan_id"], "metadata.plan_id")?,
            )
        };
        let customer_email =
            optional_charge_identity(&metadata["customer_email"], "metadata.customer_email")?;

        Ok(WebhookEvent {
            subscription_id,
            customer_id,
            customer_email,
            plan_id,
            status,
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

        super::require_mock_operation(&self.api_key, self.name(), "create customer portal")?;

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
        super::require_mock_operation(&self.api_key, self.name(), "cancel subscription")?;

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

const MAX_CHARGE_IDENTITY_BYTES: usize = 255;

fn charge_identity(value: &Value, field: &str) -> Result<String, CapitalError> {
    let text = optional_charge_identity(value, field)?;
    if text.is_empty() {
        return Err(CapitalError::PayloadParseError(format!(
            "Coinbase charge {field} is required"
        )));
    }
    Ok(text)
}

fn optional_charge_identity(value: &Value, field: &str) -> Result<String, CapitalError> {
    match value {
        Value::Null => Ok(String::new()),
        Value::String(text)
            if !text.trim().is_empty()
                && text.len() <= MAX_CHARGE_IDENTITY_BYTES
                && !text.chars().any(char::is_control) =>
        {
            Ok(text.clone())
        }
        _ => Err(CapitalError::PayloadParseError(format!(
            "Coinbase charge {field} must be a bounded non-empty string"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coinbase_provider_methods() {
        let provider = CoinbaseCommerceProvider::new("mock_key", "sec_coin123");
        assert_eq!(provider.name(), "coinbase");

        // 1. Checkout session
        let url = provider
            .create_checkout_session("crypto@user.com", "crypto_plan", "https://app.com/success")
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

        // 6. Signature verification
        let secret = "sec_coin123";
        let payload = br#"{"event":{"type":"charge:confirmed","data":{"id":"ch_123","expires_at":"2026-01-01T01:00:00Z","metadata":{"customer_id":"cust_9","customer_email":"crypto@user.com","plan_id":"crypto_plan"}}}}"#;

        let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
        let sig = hmac::sign(&key, payload);
        let sig_hex = hex::encode(sig.as_ref());

        assert!(provider.verify_signature(payload, &sig_hex).is_ok());

        // Signature error paths
        let no_sec = CoinbaseCommerceProvider::new("k", "");
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
        let mut headers = HashMap::new();
        headers.insert("x-cc-webhook-signature".to_string(), sig_hex);

        let event = provider.handle_webhook(payload, &headers).unwrap();
        assert_eq!(event.subscription_id, "ch_123");
        assert_eq!(event.customer_id, "cust_9");
        assert_eq!(event.customer_email, "crypto@user.com");
        assert_eq!(event.plan_id, "crypto_plan");
        assert_eq!(event.status, SubscriptionStatus::Active);
        // The charge's payment window is not an entitlement period.
        assert_eq!(event.ends_at, None);

        // Charge failed event
        let failed_payload = br#"{"event":{"type":"charge:failed","data":{"id":"ch_fail"}}}"#;
        let failed_sig = hex::encode(hmac::sign(&key, failed_payload).as_ref());
        let mut failed_headers = HashMap::new();
        failed_headers.insert("x-cc-webhook-signature".to_string(), failed_sig);
        let failed_event = provider
            .handle_webhook(failed_payload, &failed_headers)
            .unwrap();
        assert_eq!(failed_event.status, SubscriptionStatus::Unpaid);
        assert_eq!(failed_event.plan_id, "");

        // Confirmed charges never activate an invented plan or identity.
        for confirmed in [
            &br#"{"event":{"type":"charge:confirmed","data":{"id":"ch_1","metadata":{"customer_id":"c"}}}}"#[..],
            &br#"{"event":{"type":"charge:confirmed","data":{"id":"ch_1","metadata":{"plan_id":"p"}}}}"#[..],
            &br#"{"event":{"type":"charge:resolved","data":{"metadata":{"customer_id":"c","plan_id":"p"}}}}"#[..],
            &br#"{"event":{"type":"charge:confirmed","data":{"id":"","metadata":{"customer_id":"c","plan_id":"p"}}}}"#[..],
            &br#"{"event":{"type":"charge:confirmed","data":{"id":7,"metadata":{"customer_id":"c","plan_id":"p"}}}}"#[..],
        ] {
            let signature = hex::encode(hmac::sign(&key, confirmed).as_ref());
            let headers = HashMap::from([("x-cc-webhook-signature".to_string(), signature)]);
            assert!(matches!(
                provider.handle_webhook(confirmed, &headers),
                Err(CapitalError::PayloadParseError(_))
            ));
        }

        // Webhook error paths
        let empty_headers = HashMap::new();
        assert!(provider.handle_webhook(payload, &empty_headers).is_err());
        assert!(provider.handle_webhook(b"invalid json", &headers).is_err());
    }
}
