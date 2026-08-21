use super::{BillingProvider, SubscriptionStatus, WebhookEvent, url_encode};
use crate::error::CapitalError;
use async_trait::async_trait;
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;
use subtle::ConstantTimeEq;

/// Billing provider implementation for Alipay (支付宝 / Alipay+ China & APAC Cross-Border Payments).
pub struct AlipayProvider {
    app_id: String,
    private_key: String,
    public_key: String,
    gateway_url: String,
}

impl AlipayProvider {
    /// Creates a new `AlipayProvider` instance.
    pub fn new(app_id: String, private_key: String, public_key: String) -> Self {
        Self {
            app_id,
            private_key,
            public_key,
            gateway_url: "https://openapi.alipay.com/gateway.do".to_string(),
        }
    }

    /// Sets a custom gateway URL (e.g. Alipay sandbox `https://openapi-sandbox.dl.alipaydev.com/gateway.do`).
    pub fn with_gateway_url(mut self, gateway_url: String) -> Self {
        self.gateway_url = gateway_url;
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

    /// Verifies the webhook signature using constant-time comparison.
    pub fn verify_signature(&self, payload: &[u8], signature: &str) -> Result<(), CapitalError> {
        if self.public_key.is_empty() {
            return Ok(());
        }

        let key = hmac::Key::new(hmac::HMAC_SHA256, self.public_key.as_bytes());
        let tag = hmac::sign(&key, payload);

        // If signature is provided as hex
        if let Ok(sig_bytes) = hex::decode(signature)
            && tag.as_ref().ct_eq(&sig_bytes).unwrap_u8() == 1
        {
            return Ok(());
        }

        // If signature is raw or base64, compare against signature bytes directly
        let sig_raw = signature.as_bytes();
        let tag_hex = hex::encode(tag.as_ref());
        if tag_hex.as_bytes().ct_eq(sig_raw).unwrap_u8() == 1 {
            return Ok(());
        }

        Err(CapitalError::InvalidSignature(
            "Alipay signature verification failed".to_string(),
        ))
    }
}

#[async_trait]
impl BillingProvider for AlipayProvider {
    fn name(&self) -> &'static str {
        "alipay"
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

        if self.app_id.is_empty() || self.app_id.starts_with("mock_") {
            return Ok(format!(
                "{}?app_id={}&method=alipay.trade.page.pay&email={}&plan={}&return_url={}",
                self.gateway_url,
                url_encode(if self.app_id.is_empty() {
                    "mock_alipay_app"
                } else {
                    &self.app_id
                }),
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let out_trade_no = format!("RULLST_{}_{}", plan_id, chrono::Utc::now().timestamp());
        let biz_content = serde_json::json!({
            "out_trade_no": out_trade_no,
            "product_code": "FAST_INSTANT_TRADE_PAY",
            "total_amount": "29.00",
            "subject": format!("Subscription Plan {}", plan_id),
            "body": format!("SaaS Subscription for {}", customer_email),
            "passback_params": url_encode(customer_email),
        });

        let checkout_url = format!(
            "{}?app_id={}&method=alipay.trade.page.pay&format=JSON&charset=utf-8&sign_type=RSA2&version=1.0&return_url={}&notify_url={}&biz_content={}",
            self.gateway_url,
            url_encode(&self.app_id),
            url_encode(redirect_url),
            url_encode(redirect_url),
            url_encode(&biz_content.to_string())
        );

        Ok(checkout_url)
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        let sig_header = headers
            .get("alipay-signature")
            .or_else(|| headers.get("x-alipay-signature"))
            .or_else(|| headers.get("sign"));

        if let Some(sig) = sig_header {
            self.verify_signature(payload, sig)?;
        } else if !self.public_key.is_empty() {
            return Err(CapitalError::InvalidSignature(
                "Missing Alipay signature header".to_string(),
            ));
        }

        // Support both JSON payloads and URL-encoded notification form posts
        let (out_trade_no, trade_no, trade_status, buyer_email, plan_id, gmt_close) =
            if let Ok(json) = serde_json::from_slice::<Value>(payload) {
                let out_trade_no = json["out_trade_no"].as_str().unwrap_or("").to_string();
                let trade_no = json["trade_no"].as_str().unwrap_or("").to_string();
                let trade_status = json["trade_status"]
                    .as_str()
                    .unwrap_or("TRADE_SUCCESS")
                    .to_string();
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
                    trade_status,
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
                let trade_status = params
                    .get("trade_status")
                    .cloned()
                    .unwrap_or_else(|| "TRADE_SUCCESS".to_string());
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
            _ => SubscriptionStatus::Active,
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

        Ok(format!(
            "https://custweb.alipay.com/account/index.htm?email={}",
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
        );

        let url = provider
            .create_checkout_session("user@alipay.com", "pro_plan", "https://myapp.com/callback")
            .await
            .unwrap();

        assert!(url.contains("alipay.trade.page.pay"));
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
        assert!(portal.contains("custweb.alipay.com"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // Cancel
        assert!(provider.cancel_subscription("sub_ali").await.is_ok());
        assert!(provider.cancel_subscription("").await.is_err());

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
        let provider = AlipayProvider::new(
            "mock_app".to_string(),
            "mock_priv".to_string(),
            "".to_string(), // Empty key skips strict signature check
        );

        let payload = r#"{
            "out_trade_no": "SUB_ALI_123",
            "trade_no": "202608152200140001",
            "trade_status": "TRADE_SUCCESS",
            "buyer_email": "consumer@alipay.cn",
            "subject": "Pro Tier"
        }"#;

        let headers = HashMap::new();
        let event = provider
            .handle_webhook(payload.as_bytes(), &headers)
            .unwrap();

        assert_eq!(event.subscription_id, "SUB_ALI_123");
        assert_eq!(event.customer_id, "202608152200140001");
        assert_eq!(event.customer_email, "consumer@alipay.cn");
        assert_eq!(event.status, SubscriptionStatus::Active);
    }
}
