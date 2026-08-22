use super::{PayoutEvent, PayoutProvider, PayoutStatus};
use crate::error::CapitalError;
use async_trait::async_trait;
use serde_json::Value;

/// Payout provider implementation for Wise (Global Multi-Currency B2B Payouts & Disbursements).
pub struct WiseProvider {
    api_token: String,
    profile_id: String,
}

impl WiseProvider {
    /// Creates a new `WiseProvider` instance.
    pub fn new(api_token: impl Into<String>, profile_id: impl Into<String>) -> Self {
        Self {
            api_token: api_token.into(),
            profile_id: profile_id.into(),
        }
    }

    /// Sends a payout to an international recipient.
    pub async fn send_payout(
        &self,
        recipient_email: &str,
        amount_cents: u64,
        currency: &str,
        _reference: &str,
    ) -> Result<String, CapitalError> {
        if recipient_email.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Recipient email cannot be empty".to_string(),
            ));
        }
        if amount_cents == 0 {
            return Err(CapitalError::ConfigurationError(
                "Transfer amount must be greater than 0".to_string(),
            ));
        }
        if currency.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Currency cannot be empty".to_string(),
            ));
        }
        self.create_transfer(recipient_email, amount_cents, currency)
            .await
    }

    /// Retrieves payout status.
    pub async fn get_payout_status(&self, transfer_id: &str) -> Result<PayoutStatus, CapitalError> {
        self.get_transfer_status(transfer_id).await
    }
}

#[async_trait]
impl PayoutProvider for WiseProvider {
    fn name(&self) -> &'static str {
        "wise"
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn create_transfer(
        &self,
        recipient_email: &str,
        amount_cents: u64,
        currency: &str,
    ) -> Result<String, CapitalError> {
        if recipient_email.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Recipient email cannot be empty".to_string(),
            ));
        }
        if amount_cents == 0 {
            return Err(CapitalError::ConfigurationError(
                "Transfer amount must be greater than 0".to_string(),
            ));
        }
        if currency.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Currency cannot be empty".to_string(),
            ));
        }

        if self.api_token.is_empty() || self.api_token.starts_with("mock_") {
            return Ok(format!(
                "wise_tr_mock_{}",
                recipient_email.replace('@', "_")
            ));
        }

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "targetAccount": recipient_email,
            "quoteUuid": format!("profile_{}", self.profile_id),
            "customerTransactionId": format!("payout_{}_{}", recipient_email, amount_cents),
            "details": {
                "reference": "SaaS Creator Payout",
                "transferPurpose": "verification.transfers.purpose.pay.bills",
                "sourceOfFunds": "verification.source.of.funds.other"
            },
            "amount": amount_cents as f64 / 100.0,
            "currency": currency
        });

        let res = client
            .post("https://api.wise.com/v1/transfers")
            .bearer_auth(&self.api_token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CapitalError::ProviderRequestFailed(format!("Network error: {}", e)))?;

        if !res.status().is_success() {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "Wise API error: HTTP {}",
                res.status()
            )));
        }

        let body: Value = res.json().await.map_err(|e| {
            CapitalError::PayloadParseError(format!("Failed to parse response: {}", e))
        })?;

        body["id"]
            .as_i64()
            .map(|id| id.to_string())
            .or_else(|| body["id"].as_str().map(|s| s.to_string()))
            .ok_or_else(|| {
                CapitalError::PayloadParseError("Missing transfer ID in Wise response".to_string())
            })
    }

    async fn get_transfer_status(&self, transfer_id: &str) -> Result<PayoutStatus, CapitalError> {
        if transfer_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Transfer ID cannot be empty".to_string(),
            ));
        }

        if self.api_token.is_empty() || self.api_token.starts_with("mock_") {
            return Ok(PayoutStatus::OutgoingPaymentSent);
        }

        let client = reqwest::Client::new();
        let res = client
            .get(format!("https://api.wise.com/v1/transfers/{}", transfer_id))
            .bearer_auth(&self.api_token)
            .send()
            .await
            .map_err(|e| CapitalError::ProviderRequestFailed(format!("Network error: {}", e)))?;

        if !res.status().is_success() {
            return Err(CapitalError::ProviderRequestFailed(format!(
                "Wise API error: HTTP {}",
                res.status()
            )));
        }

        let body: Value = res.json().await.map_err(|e| {
            CapitalError::PayloadParseError(format!("Failed to parse response: {}", e))
        })?;

        let status_str = body["status"].as_str().unwrap_or("processing");
        match status_str {
            "outgoing_payment_sent" => Ok(PayoutStatus::OutgoingPaymentSent),
            "funds_refunded" => Ok(PayoutStatus::FundsRefunded),
            "cancelled" => Ok(PayoutStatus::Cancelled),
            _ => Ok(PayoutStatus::Processing),
        }
    }
}

impl WiseProvider {
    /// Normalizes a webhook payload from Wise into a `PayoutEvent`.
    pub fn parse_webhook_payload(&self, payload: &[u8]) -> Result<PayoutEvent, CapitalError> {
        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        let data = &json["data"];
        let transfer_id = data["resource"]["id"]
            .as_i64()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "transfer_unknown".to_string());

        let recipient_email = data["resource"]["recipient_email"]
            .as_str()
            .unwrap_or("unknown@example.com")
            .to_string();

        let amount_cents = (data["resource"]["amount"].as_f64().unwrap_or(0.0) * 100.0) as u64;
        let currency = data["resource"]["currency"]
            .as_str()
            .unwrap_or("USD")
            .to_string();

        let status_str = data["current_state"].as_str().unwrap_or("processing");
        let status = match status_str {
            "outgoing_payment_sent" => PayoutStatus::OutgoingPaymentSent,
            "funds_refunded" => PayoutStatus::FundsRefunded,
            "cancelled" => PayoutStatus::Cancelled,
            _ => PayoutStatus::Processing,
        };

        Ok(PayoutEvent {
            transfer_id,
            recipient_email,
            amount_cents,
            currency,
            status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wise_provider_payout_lifecycle() {
        let provider = WiseProvider::new("mock_wise_token", "sec_wise123");
        assert_eq!(provider.name(), "wise");

        // 1. Send payout
        let transfer_id = provider
            .send_payout("beneficiary@wise.com", 15000, "USD", "Invoice 1234")
            .await
            .unwrap();
        assert!(transfer_id.starts_with("wise_tr_"));

        // 2. Validation errors
        assert!(provider.send_payout("", 1000, "USD", "desc").await.is_err());
        assert!(
            provider
                .send_payout("a@b.com", 0, "USD", "desc")
                .await
                .is_err()
        );
        assert!(
            provider
                .send_payout("a@b.com", 1000, "", "desc")
                .await
                .is_err()
        );

        // 3. Status
        let status = provider.get_payout_status("tr_123").await.unwrap();
        assert_eq!(status, PayoutStatus::OutgoingPaymentSent);
        assert!(provider.get_payout_status("").await.is_err());

        // 4. Webhook payload parsing
        let payload = r#"{
            "data": {
                "resource": {
                    "id": 987654321,
                    "recipient_email": "payee@wise.com",
                    "amount": 250.75,
                    "currency": "EUR"
                },
                "current_state": "outgoing_payment_sent"
            }
        }"#;
        let event = provider.parse_webhook_payload(payload.as_bytes()).unwrap();
        assert_eq!(event.transfer_id, "987654321");
        assert_eq!(event.recipient_email, "payee@wise.com");
        assert_eq!(event.amount_cents, 25075);
        assert_eq!(event.currency, "EUR");
        assert_eq!(event.status, PayoutStatus::OutgoingPaymentSent);

        // Other states
        let refunded_payload = r#"{"data":{"resource":{"id":1},"current_state":"funds_refunded"}}"#;
        let refunded_event = provider
            .parse_webhook_payload(refunded_payload.as_bytes())
            .unwrap();
        assert_eq!(refunded_event.status, PayoutStatus::FundsRefunded);

        let cancelled_payload = r#"{"data":{"resource":{"id":2},"current_state":"cancelled"}}"#;
        let cancelled_event = provider
            .parse_webhook_payload(cancelled_payload.as_bytes())
            .unwrap();
        assert_eq!(cancelled_event.status, PayoutStatus::Cancelled);

        let unknown_payload = r#"{"data":{"resource":{"id":3},"current_state":"other"}}"#;
        let unknown_event = provider
            .parse_webhook_payload(unknown_payload.as_bytes())
            .unwrap();
        assert_eq!(unknown_event.status, PayoutStatus::Processing);

        // Webhook error paths
        assert!(provider.parse_webhook_payload(b"invalid json").is_err());
    }
}
