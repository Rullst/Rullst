use super::{
    BillingProvider, SubscriptionStatus, WebhookEvent, WebhookVerificationMode, url_encode,
    verify_explicit_mock_signature, webhook_mode_from_secret,
};
use crate::error::CapitalError;
use async_trait::async_trait;
#[cfg(test)]
use ring::hmac;
use serde_json::Value;
use std::collections::HashMap;

/// Billing provider implementation for Polar.sh (Developer-First MoR & Open Source).
pub struct PolarProvider {
    api_key: String,
    webhook_secret: String,
}

impl PolarProvider {
    /// Creates a new `PolarProvider` instance.
    pub fn new(api_key: impl Into<String>, webhook_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            webhook_secret: webhook_secret.into(),
        }
    }

    /// Offline compatibility helper. Live verification requires all Standard
    /// Webhooks headers and must use `BillingProvider::handle_webhook`.
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

        Err(CapitalError::UnsupportedOperation(
            "Polar live verification requires Standard Webhooks ID, timestamp and signature headers".into(),
        ))
    }
}

#[async_trait]
impl BillingProvider for PolarProvider {
    fn name(&self) -> &'static str {
        "polar"
    }

    fn webhook_verification_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        webhook_mode_from_secret(self.name(), &self.webhook_secret)
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
                "https://polar.sh/checkout/mock_session?email={}&product={}&success_url={}",
                url_encode(customer_email),
                url_encode(plan_id),
                url_encode(redirect_url)
            ));
        }

        let client = crate::providers::http_client()?;
        let payload = serde_json::json!({
            "product_price_id": plan_id,
            "customer_email": customer_email,
            "success_url": redirect_url
        });

        let body: Value = crate::providers::send_http_json(
            client
                .post("https://api.polar.sh/v1/checkouts/custom/")
                .bearer_auth(&self.api_key)
                .header("Content-Type", "application/json")
                .json(&payload),
            "polar",
            "create checkout",
        )
        .await?;

        let url = body["url"].as_str().ok_or_else(|| {
            CapitalError::from(crate::ProviderFailure::contract_mismatch(
                "polar",
                "create checkout",
            ))
        })?;
        crate::providers::validate_checkout_url("polar", url)
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        if self.webhook_verification_mode()? == WebhookVerificationMode::Mock {
            let signature = headers
                .get("polar-signature")
                .or_else(|| headers.get("webhook-signature"))
                .ok_or_else(|| {
                    CapitalError::InvalidSignature("Missing Polar-Signature header".into())
                })?;
            self.verify_signature(payload, signature)?;
        } else {
            super::polar_webhook::verify(
                &self.webhook_secret,
                payload,
                headers,
                chrono::Utc::now().timestamp(),
            )?;
        }

        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        let data = &json["data"];
        let subscription_id = data["id"].as_str().unwrap_or("").to_string();
        let customer_id = data["user_id"]
            .as_str()
            .or_else(|| data["customer_id"].as_str())
            .unwrap_or("")
            .to_string();

        let customer_email = data["user"]["email"]
            .as_str()
            .or_else(|| data["email"].as_str())
            .unwrap_or("")
            .to_string();

        let plan_id = data["product_id"]
            .as_str()
            .or_else(|| data["price_id"].as_str())
            .unwrap_or("")
            .to_string();

        let status_str = data["status"]
            .as_str()
            .filter(|status| !status.trim().is_empty())
            .ok_or_else(|| {
                CapitalError::PayloadParseError("Webhook status is missing or invalid".into())
            })?;
        let ends_at = data["current_period_end"].as_i64();

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
            "https://polar.sh/purchases?email={}",
            url_encode(customer_email)
        ))
    }

    async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), CapitalError> {
        if subscription_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Subscription ID cannot be empty".to_string(),
            ));
        }
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            crate::subscription::validate_provider_subscription_id(subscription_id)?;
            let client = crate::providers::http_client()?;
            crate::providers::send_http(
                client
                    .delete(format!(
                        "https://api.polar.sh/v1/subscriptions/{}",
                        subscription_id
                    ))
                    .bearer_auth(&self.api_key),
                "polar",
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
        super::require_mock_operation(&self.api_key, self.name(), "pause subscription")?;

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
        super::require_mock_operation(&self.api_key, self.name(), "report usage")?;

        Ok(())
    }

    async fn apply_coupon(
        &self,
        subscription_id: &str,
        coupon_code: &str,
    ) -> Result<(), CapitalError> {
        crate::subscription::validate_provider_subscription_id(subscription_id)?;
        let _coupon = crate::subscription::validate_coupon_code(coupon_code)?;
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            return Err(CapitalError::UnsupportedOperation(
                "Polar coupon application has no reviewed live contract".to_string(),
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
        if !self.api_key.is_empty() && !self.api_key.starts_with("mock_") {
            return Err(CapitalError::UnsupportedOperation(
                "Polar trial extension has no reviewed live contract".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_polar_provider_methods() {
        let provider = PolarProvider::new("mock_token", "sec_polar123");
        assert_eq!(provider.name(), "polar");

        // 1. Checkout session
        let url = provider
            .create_checkout_session(
                "user@polar.sh",
                "prod_polar_plan",
                "https://app.com/success",
            )
            .await
            .unwrap();
        assert!(url.contains("polar.sh/checkout"));
        assert!(url.contains("prod_polar_plan"));

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
            .create_customer_portal("user@polar.sh", "https://app.com")
            .await
            .unwrap();
        assert!(portal.contains("polar.sh/purchases"));
        assert!(provider.create_customer_portal("", "url").await.is_err());

        // 4. Subscription actions
        assert!(provider.cancel_subscription("sub_pol").await.is_ok());
        assert!(provider.cancel_subscription("").await.is_err());

        assert!(provider.pause_subscription("sub_pol").await.is_ok());
        assert!(provider.pause_subscription("").await.is_err());

        assert!(provider.report_usage("sub_pol", "api", 10).await.is_ok());
        assert!(provider.report_usage("", "api", 10).await.is_err());

        assert!(provider.apply_coupon("sub_pol", "POLAR10").await.is_ok());
        assert!(provider.apply_coupon("", "POLAR10").await.is_err());
        assert!(provider.apply_coupon("sub_pol", "").await.is_err());

        assert!(provider.extend_trial("sub_pol", 1800000000).await.is_ok());
        assert!(provider.extend_trial("", 1800000000).await.is_err());
        assert!(provider.extend_trial("sub_pol", -1).await.is_err());

        // 5. Signature verification
        let secret = "sec_polar123";
        let payload = br#"{"data":{"id":"sub_polar_100","user_id":"u_1","user":{"email":"user@polar.sh"},"product_id":"prod_polar_plan","status":"active"}}"#;

        let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
        let sig = hmac::sign(&key, payload);
        let sig_hex = hex::encode(sig.as_ref());

        assert!(matches!(
            provider.verify_signature(payload, &sig_hex),
            Err(CapitalError::UnsupportedOperation(_))
        ));

        // Signature error paths
        let no_sec = PolarProvider::new("t", "");
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

        // 6. Handle webhook
        use base64::Engine;
        let timestamp = chrono::Utc::now().timestamp();
        let mut signer = hmac::Context::with_key(&key);
        signer.update(format!("evt_polar.{timestamp}.").as_bytes());
        signer.update(payload);
        let mut headers = HashMap::new();
        headers.insert("webhook-id".into(), "evt_polar".into());
        headers.insert("webhook-timestamp".into(), timestamp.to_string());
        headers.insert(
            "webhook-signature".into(),
            format!(
                "v1,{}",
                base64::engine::general_purpose::STANDARD.encode(signer.sign().as_ref())
            ),
        );

        let event = provider.handle_webhook(payload, &headers).unwrap();
        assert_eq!(event.subscription_id, "sub_polar_100");
        assert_eq!(event.customer_email, "user@polar.sh");
        assert_eq!(event.status, SubscriptionStatus::Active);

        // Webhook error paths
        let empty_headers = HashMap::new();
        assert!(provider.handle_webhook(payload, &empty_headers).is_err());
        assert!(provider.handle_webhook(b"invalid json", &headers).is_err());
    }
}
