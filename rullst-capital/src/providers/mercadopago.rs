use super::{
    BillingProvider, SubscriptionStatus, WebhookEvent, WebhookVerificationMode, url_encode,
    verify_explicit_mock_signature, webhook_mode_from_secret,
};
use crate::error::CapitalError;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Billing provider implementation for Mercado Pago (LATAM Regional Subscriptions & Checkout).
pub struct MercadoPagoProvider {
    access_token: String,
    webhook_secret: String,
}

impl MercadoPagoProvider {
    /// Creates a new `MercadoPagoProvider` instance.
    pub fn new(access_token: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            webhook_secret: webhook_secret.into(),
        }
    }

    /// Retained for source compatibility; live webhook verification is unavailable.
    pub fn with_webhook_tolerance(self, _tolerance: Duration) -> Self {
        self
    }

    /// Verifies the `x-signature` header (`ts=...;v1=...`).
    pub fn verify_signature(
        &self,
        payload: &[u8],
        signature_header: &str,
    ) -> Result<(), CapitalError> {
        self.verify_signature_at(payload, signature_header, chrono::Utc::now().timestamp())
    }

    /// Retained for compatibility. A live Mercado Pago notification requires
    /// the original query data ID, request ID, and authoritative resource lookup;
    /// this body-only API cannot establish those provider-specific guarantees.
    pub fn verify_signature_at(
        &self,
        _payload: &[u8],
        signature_header: &str,
        _now_unix_seconds: i64,
    ) -> Result<(), CapitalError> {
        match self.webhook_verification_mode()? {
            WebhookVerificationMode::Mock => verify_explicit_mock_signature(
                self.name(), &self.webhook_secret, signature_header,
            ),
            WebhookVerificationMode::Real => Err(CapitalError::UnsupportedOperation(
                "Mercado Pago live webhooks require a reviewed query/request-ID signature and resource lookup contract".into(),
            )),
        }
    }
}

#[async_trait]
impl BillingProvider for MercadoPagoProvider {
    fn name(&self) -> &'static str {
        "mercadopago"
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

        if self.access_token.is_empty() || self.access_token.starts_with("mock_") {
            return Ok(format!(
                "https://www.mercadopago.com/checkout/preferences/mock_session?email={}&plan={}&back_url={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        Err(CapitalError::UnsupportedOperation(
            "mercadopago plan-only checkout has no reviewed authoritative pricing contract".into(),
        ))
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let _ = self.webhook_verification_mode()?;
        let sig_header = headers.get("x-signature").ok_or_else(|| {
            CapitalError::InvalidSignature("Missing x-signature header".to_string())
        })?;
        self.verify_signature(payload, sig_header)?;

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

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
            .filter(|status| !status.trim().is_empty())
            .ok_or_else(|| {
                CapitalError::PayloadParseError("Webhook status is missing or invalid".into())
            })?;

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
    ) -> Result<String, CapitalError> {
        if customer_email.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Customer email cannot be empty".to_string(),
            ));
        }

        super::require_mock_operation(&self.access_token, self.name(), "create customer portal")?;

        Ok(format!(
            "https://www.mercadopago.com/subscriptions?email={}",
            url_encode(customer_email)
        ))
    }

    async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        if !self.access_token.is_empty() && !self.access_token.starts_with("mock_") {
            crate::subscription::validate_provider_subscription_id(subscription_id)?;
            let client = crate::providers::http_client()?;
            crate::providers::send_http(
                client
                    .put(format!(
                        "https://api.mercadopago.com/preapproval/{}",
                        subscription_id
                    ))
                    .bearer_auth(&self.access_token)
                    .json(&serde_json::json!({ "status": "cancelled" })),
                "mercadopago",
                "cancel subscription",
            )
            .await?;
        }
        Ok(())
    }

    async fn pause_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        if !self.access_token.is_empty() && !self.access_token.starts_with("mock_") {
            crate::subscription::validate_provider_subscription_id(subscription_id)?;
            let client = crate::providers::http_client()?;
            crate::providers::send_http(
                client
                    .put(format!(
                        "https://api.mercadopago.com/preapproval/{}",
                        subscription_id
                    ))
                    .bearer_auth(&self.access_token)
                    .json(&serde_json::json!({ "status": "paused" })),
                "mercadopago",
                "pause subscription",
            )
            .await?;
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
        super::require_mock_operation(&self.access_token, self.name(), "report usage")?;

        Ok(())
    }

    async fn apply_coupon(
        &self,
        subscription_id: &str,
        coupon_code: &str,
    ) -> Result<(), CapitalError> {
        crate::subscription::validate_provider_subscription_id(subscription_id)?;
        let _coupon = crate::subscription::validate_coupon_code(coupon_code)?;
        if !self.access_token.is_empty() && !self.access_token.starts_with("mock_") {
            return Err(CapitalError::UnsupportedOperation(
                "Mercado Pago coupon application has no reviewed live contract".to_string(),
            ));
        }
        Ok(())
    }

    async fn extend_trial(
        &self,
        subscription_id: &str,
        trial_ends_at: i64,
    ) -> Result<(), CapitalError> {
        crate::subscription::validate_provider_subscription_id(subscription_id)?;
        crate::subscription::validate_trial_end(trial_ends_at)?;
        if !self.access_token.is_empty() && !self.access_token.starts_with("mock_") {
            return Err(CapitalError::UnsupportedOperation(
                "Mercado Pago trial extension has no reviewed live contract".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::hmac;

    #[tokio::test]
    async fn test_mercadopago_provider_methods() {
        let provider = MercadoPagoProvider::new("mock_access_token", "mock_mp_secret");
        assert_eq!(provider.name(), "mercadopago");

        // 1. Checkout session
        let url = provider
            .create_checkout_session("user@mp.com", "plan_mp", "https://app.com/success")
            .await
            .unwrap();
        assert!(url.contains("mercadopago.com/checkout/preferences"));
        assert!(url.contains("plan_mp"));

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
            .create_customer_portal("user@mp.com", "https://app.com")
            .await
            .unwrap();
        assert!(portal.contains("mercadopago.com/subscriptions"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // 4. Subscription actions
        assert!(provider.cancel_subscription("sub_mp").await.is_ok());
        assert!(provider.cancel_subscription("").await.is_err());

        assert!(provider.pause_subscription("sub_mp").await.is_ok());
        assert!(provider.pause_subscription("").await.is_err());

        assert!(provider.report_usage("sub_mp", "usage", 5).await.is_ok());
        assert!(provider.report_usage("", "usage", 5).await.is_err());

        assert!(provider.apply_coupon("sub_mp", "CUPOM10").await.is_ok());
        assert!(provider.apply_coupon("", "CUPOM10").await.is_err());
        assert!(provider.apply_coupon("sub_mp", "").await.is_err());

        assert!(provider.extend_trial("sub_mp", 1800000000).await.is_ok());
        assert!(provider.extend_trial("", 1800000000).await.is_err());
        assert!(provider.extend_trial("sub_mp", -1).await.is_err());

        // 5. Signature verification
        let secret = "sec_mp123";
        let real_provider = MercadoPagoProvider::new("live-fixture-key", secret);
        let now = chrono::Utc::now().timestamp();
        let timestamp = now.to_string();
        let payload = br#"{"type":"payment","data":{"id":"pay_mp_999","email":"user@mp.com","plan_id":"plan_mp_pro","status":"approved"}}"#;

        let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
        let mut ctx = hmac::Context::with_key(&key);
        ctx.update(timestamp.as_bytes());
        ctx.update(b":");
        ctx.update(payload);
        let sig_hex = hex::encode(ctx.sign().as_ref());
        let valid_header = format!("ts={},v1={}", timestamp, sig_hex);

        assert!(matches!(
            real_provider.verify_signature_at(payload, &valid_header, now),
            Err(CapitalError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            real_provider.verify_signature_at(payload, &valid_header, now + 301),
            Err(CapitalError::UnsupportedOperation(_))
        ));

        // Signature error paths
        let no_secret_provider = MercadoPagoProvider::new("token", "");
        assert!(matches!(
            no_secret_provider.verify_signature(payload, ""),
            Err(CapitalError::ConfigurationError(_))
        ));
        assert!(
            provider
                .verify_signature(payload, "invalid_header")
                .is_err()
        );
        assert!(
            provider
                .verify_signature(payload, "ts=123,v1=invalid_hex_!")
                .is_err()
        );
        assert!(
            provider
                .verify_signature(
                    payload,
                    "ts=123,v1=00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
                )
                .is_err()
        );

        // 6. Handle webhook
        let mut headers = HashMap::new();
        headers.insert("x-signature".to_string(), "mock_mp_secret".into());

        let event = provider.handle_webhook(payload, &headers).unwrap();
        assert_eq!(event.subscription_id, "pay_mp_999");
        assert_eq!(event.customer_email, "user@mp.com");
        assert_eq!(event.status, SubscriptionStatus::Active);

        // Webhook error paths
        let empty_headers = HashMap::new();
        assert!(provider.handle_webhook(payload, &empty_headers).is_err());
        assert!(provider.handle_webhook(b"invalid json", &headers).is_err());
    }
}
