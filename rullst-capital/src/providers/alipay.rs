use super::{
    BillingProvider, SubscriptionStatus, WebhookEvent, WebhookVerificationMode, url_encode,
    verify_explicit_mock_signature,
};
use crate::error::CapitalError;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Billing provider implementation for Alipay (支付宝 / Alipay+ China & APAC Cross-Border Payments).
pub struct AlipayProvider {
    app_id: String,
    private_key: String,
    public_key: String,
    gateway_url: String,
}

impl AlipayProvider {
    /// Creates a new `AlipayProvider` instance.
    pub fn new(
        app_id: impl Into<String>,
        private_key: impl Into<String>,
        public_key: impl Into<String>,
    ) -> Self {
        Self {
            app_id: app_id.into(),
            private_key: private_key.into(),
            public_key: public_key.into(),
            gateway_url: "https://openapi.alipay.com/gateway.do".to_string(),
        }
    }

    /// Sets a custom gateway URL (e.g. Alipay sandbox `https://openapi-sandbox.dl.alipaydev.com/gateway.do`).
    pub fn with_gateway_url(mut self, gateway_url: impl Into<String>) -> Self {
        self.gateway_url = gateway_url.into();
        self
    }

    /// Returns the Alipay Application ID.
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Returns the Alipay Private Key.
    pub fn private_key(&self) -> &str {
        &self.private_key
    }

    /// Returns the Alipay Public Key.
    pub fn public_key(&self) -> &str {
        &self.public_key
    }

    /// Returns the Alipay Gateway URL.
    pub fn gateway_url(&self) -> &str {
        &self.gateway_url
    }

    fn credential_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        let credentials = [
            ("application ID", self.app_id.trim()),
            ("private key", self.private_key.trim()),
            ("public key", self.public_key.trim()),
        ];
        if let Some((name, _)) = credentials.iter().find(|(_, value)| value.is_empty()) {
            return Err(CapitalError::ConfigurationError(format!(
                "Alipay {name} cannot be empty"
            )));
        }

        let mock_count = credentials
            .iter()
            .filter(|(_, value)| value.starts_with("mock_"))
            .count();
        match mock_count {
            0 => Ok(WebhookVerificationMode::Real),
            3 => Ok(WebhookVerificationMode::Mock),
            _ => Err(CapitalError::ConfigurationError(
                "Alipay credentials cannot mix real and mock values".to_string(),
            )),
        }
    }

    /// Verifies an explicit local mock signature.
    ///
    /// Live Alipay RSA2 verification is intentionally disabled until the provider is backed by
    /// an interoperable RSA-SHA256 implementation and official contract tests.
    pub fn verify_signature(&self, _payload: &[u8], signature: &str) -> Result<(), CapitalError> {
        match self.credential_mode()? {
            WebhookVerificationMode::Mock => {
                verify_explicit_mock_signature(self.name(), &self.public_key, signature)
            }
            WebhookVerificationMode::Real => Err(CapitalError::UnsupportedOperation(
                "Alipay RSA2 verification is not implemented; live webhooks are disabled"
                    .to_string(),
            )),
        }
    }
}

#[async_trait]
impl BillingProvider for AlipayProvider {
    fn name(&self) -> &'static str {
        "alipay"
    }

    fn webhook_verification_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        self.credential_mode()
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

        match self.credential_mode()? {
            WebhookVerificationMode::Mock => Ok(format!(
                "https://mock.alipay.invalid/checkout?app_id={}&email={}&plan={}&return_url={}",
                url_encode(&self.app_id),
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            )),
            WebhookVerificationMode::Real => Err(CapitalError::UnsupportedOperation(
                "Alipay RSA2 checkout signing is not implemented; live checkout is disabled"
                    .to_string(),
            )),
        }
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        if self.webhook_verification_mode()? == WebhookVerificationMode::Real {
            return Err(CapitalError::UnsupportedOperation(
                "Alipay RSA2 verification is not implemented; live webhooks are disabled"
                    .to_string(),
            ));
        }
        let sig_header = headers
            .get("alipay-signature")
            .or_else(|| headers.get("x-alipay-signature"))
            .or_else(|| headers.get("sign"))
            .ok_or_else(|| {
                CapitalError::InvalidSignature("Missing Alipay signature header".to_string())
            })?;
        self.verify_signature(payload, sig_header)?;

        // Support both JSON payloads and URL-encoded notification form posts
        let (out_trade_no, trade_no, trade_status, buyer_email, plan_id, gmt_close) =
            if let Ok(json) = serde_json::from_slice::<Value>(payload) {
                let out_trade_no = json["out_trade_no"].as_str().unwrap_or("").to_string();
                let trade_no = json["trade_no"].as_str().unwrap_or("").to_string();
                let trade_status = json["trade_status"].as_str().ok_or_else(|| {
                    CapitalError::PayloadParseError("Missing Alipay trade_status field".to_string())
                })?;
                let buyer_email = json["buyer_email"]
                    .as_str()
                    .or_else(|| json["buyer_id"].as_str())
                    .unwrap_or("")
                    .to_string();
                let plan_id = json["plan_id"]
                    .as_str()
                    .or_else(|| json["subject"].as_str())
                    .unwrap_or("default")
                    .to_string();
                let gmt_close = json["gmt_close"].as_i64();
                (
                    out_trade_no,
                    trade_no,
                    trade_status.to_string(),
                    buyer_email,
                    plan_id,
                    gmt_close,
                )
            } else {
                let body_str = String::from_utf8_lossy(payload);
                let mut params = HashMap::new();
                for pair in body_str.split('&') {
                    let mut parts = pair.splitn(2, '=');
                    if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                        params.insert(k.to_string(), v.to_string());
                    }
                }

                let out_trade_no = params.get("out_trade_no").cloned().unwrap_or_default();
                let trade_no = params.get("trade_no").cloned().unwrap_or_default();
                let trade_status = params.get("trade_status").cloned().ok_or_else(|| {
                    CapitalError::PayloadParseError("Missing Alipay trade_status field".to_string())
                })?;
                let buyer_email = params
                    .get("buyer_email")
                    .or_else(|| params.get("buyer_id"))
                    .or_else(|| params.get("passback_params"))
                    .cloned()
                    .unwrap_or_default();
                let plan_id = params
                    .get("subject")
                    .cloned()
                    .unwrap_or_else(|| "default".to_string());
                (
                    out_trade_no,
                    trade_no,
                    trade_status,
                    buyer_email,
                    plan_id,
                    None,
                )
            };

        let status = match trade_status.as_str() {
            "TRADE_SUCCESS" | "TRADE_FINISHED" => SubscriptionStatus::Active,
            "TRADE_CLOSED" => SubscriptionStatus::Canceled,
            "WAIT_BUYER_PAY" => SubscriptionStatus::PastDue,
            _ => SubscriptionStatus::Unpaid,
        };

        Ok(WebhookEvent {
            subscription_id: out_trade_no,
            customer_id: trade_no,
            customer_email: buyer_email,
            plan_id,
            status,
            ends_at: gmt_close,
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

        match self.credential_mode()? {
            WebhookVerificationMode::Mock => Ok(format!(
                "https://mock.alipay.invalid/account?email={}",
                url_encode(customer_email)
            )),
            WebhookVerificationMode::Real => Err(CapitalError::UnsupportedOperation(
                "Live Alipay customer portal integration is not implemented".to_string(),
            )),
        }
    }

    async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        match self.credential_mode()? {
            WebhookVerificationMode::Mock => Ok(()),
            WebhookVerificationMode::Real => Err(CapitalError::UnsupportedOperation(
                "Live Alipay cancellation is not implemented".to_string(),
            )),
        }
    }

    async fn pause_subscription(&self, _subscription_id: &str) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "Alipay does not support subscription pause".to_string(),
        ))
    }

    async fn report_usage(
        &self,
        _subscription_id: &str,
        _metric: &str,
        _quantity: u64,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "Alipay does not support metered usage reporting".to_string(),
        ))
    }

    async fn apply_coupon(
        &self,
        _subscription_id: &str,
        _coupon_code: &str,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "Alipay does not support coupon application".to_string(),
        ))
    }

    async fn extend_trial(
        &self,
        _subscription_id: &str,
        _trial_ends_at: i64,
    ) -> Result<(), CapitalError> {
        Err(CapitalError::UnsupportedOperation(
            "Alipay does not support trial extension".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alipay_mock_checkout() {
        let provider = AlipayProvider::new(
            "mock_app_id".to_string(),
            "mock_private_key".to_string(),
            "mock_public_key".to_string(),
        )
        .with_gateway_url("https://openapi-sandbox.dl.alipaydev.com/gateway.do");

        assert_eq!(provider.name(), "alipay");
        assert_eq!(provider.app_id(), "mock_app_id");
        assert_eq!(provider.private_key(), "mock_private_key");
        assert_eq!(provider.public_key(), "mock_public_key");
        assert_eq!(
            provider.gateway_url(),
            "https://openapi-sandbox.dl.alipaydev.com/gateway.do"
        );

        let url = provider
            .create_checkout_session("user@alipay.com", "pro_plan", "https://myapp.com/callback")
            .await
            .unwrap();

        assert!(url.starts_with("https://mock.alipay.invalid/checkout?"));
        assert!(url.contains("user%40alipay.com"));
        assert!(url.contains("pro_plan"));

        // Validation
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

        // Customer portal
        let portal = provider
            .create_customer_portal("user@alipay.com", "https://myapp.com")
            .await
            .unwrap();
        assert!(portal.contains("mock.alipay.invalid/account"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // Cancel
        assert!(provider.cancel_subscription("sub_ali").await.is_ok());
        assert!(provider.cancel_subscription("").await.is_err());

        let live = AlipayProvider::new("live_app", "live_private", "live_public");
        assert!(matches!(
            live.create_checkout_session("user@example.com", "plan", "https://example.com")
                .await,
            Err(CapitalError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            live.cancel_subscription("sub_live").await,
            Err(CapitalError::UnsupportedOperation(_))
        ));

        // Unsupported operations
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

    #[test]
    fn test_alipay_webhook_handling() {
        let provider = AlipayProvider::new("mock_app_id", "mock_private_key", "mock_public_key");

        let payload = r#"{
            "out_trade_no": "SUB_ALI_123",
            "trade_no": "202608152200140001",
            "trade_status": "TRADE_SUCCESS",
            "buyer_email": "consumer@alipay.cn",
            "subject": "Pro Tier"
        }"#;

        // 1. Explicit mock signature
        assert!(
            provider
                .verify_signature(payload.as_bytes(), "mock_public_key")
                .is_ok()
        );

        // 2. Invalid mock signature
        assert!(
            provider
                .verify_signature(payload.as_bytes(), "bad_sig")
                .is_err()
        );

        // 3. Handle mock webhook with mandatory header
        let mut headers = HashMap::new();
        headers.insert("sign".to_string(), "mock_public_key".to_string());

        let event = provider
            .handle_webhook(payload.as_bytes(), &headers)
            .unwrap();

        assert_eq!(event.subscription_id, "SUB_ALI_123");
        assert_eq!(event.customer_id, "202608152200140001");
        assert_eq!(event.customer_email, "consumer@alipay.cn");
        assert_eq!(event.status, SubscriptionStatus::Active);

        // 4. TRADE_CLOSED event -> Canceled
        let closed_payload = r#"{"out_trade_no":"SUB_CLOSED","trade_status":"TRADE_CLOSED"}"#;
        let mut closed_headers = HashMap::new();
        closed_headers.insert("sign".to_string(), "mock_public_key".to_string());
        let closed_event = provider
            .handle_webhook(closed_payload.as_bytes(), &closed_headers)
            .unwrap();
        assert_eq!(closed_event.status, SubscriptionStatus::Canceled);

        // 5. WAIT_BUYER_PAY event -> PastDue
        let wait_payload = r#"{"out_trade_no":"SUB_WAIT","trade_status":"WAIT_BUYER_PAY"}"#;
        let mut wait_headers = HashMap::new();
        wait_headers.insert("sign".to_string(), "mock_public_key".to_string());
        let wait_event = provider
            .handle_webhook(wait_payload.as_bytes(), &wait_headers)
            .unwrap();
        assert_eq!(wait_event.status, SubscriptionStatus::PastDue);

        // 6. Error handling
        let empty_headers = HashMap::new();
        assert!(
            provider
                .handle_webhook(payload.as_bytes(), &empty_headers)
                .is_err()
        );
        assert!(provider.handle_webhook(b"invalid json", &headers).is_err());

        let real = AlipayProvider::new("real_app", "real_private", "real_public");
        assert!(matches!(
            real.verify_signature(payload.as_bytes(), "any"),
            Err(CapitalError::UnsupportedOperation(_))
        ));
        assert!(matches!(
            real.handle_webhook(payload.as_bytes(), &HashMap::new()),
            Err(CapitalError::UnsupportedOperation(_))
        ));

        let empty = AlipayProvider::new("", "", "");
        assert!(matches!(
            empty.verify_signature(payload.as_bytes(), ""),
            Err(CapitalError::ConfigurationError(_))
        ));
    }
}
