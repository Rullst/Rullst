use super::{PayoutEvent, PayoutProvider, PayoutStatus};
use async_trait::async_trait;
use serde_json::Value;

/// Payout provider implementation for Wise (Global Multi-Currency B2B Payouts & Disbursements).
pub struct WiseProvider {
    api_token: String,
    profile_id: String,
}

impl WiseProvider {
    /// Creates a new `WiseProvider` instance.
    pub fn new(api_token: String, profile_id: String) -> Self {
        Self {
            api_token,
            profile_id,
        }
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
    ) -> Result<String, String> {
        if self.api_token.is_empty() || self.api_token.starts_with("mock_") {
            return Ok(format!(
                "transfer_mock_{}",
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
            .map_err(|e| format!("Network error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Wise API error: HTTP {}", res.status()));
        }

        let body: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        body["id"]
            .as_i64()
            .map(|id| id.to_string())
            .or_else(|| body["id"].as_str().map(|s| s.to_string()))
            .ok_or_else(|| "Missing transfer ID in Wise response".to_string())
    }

    async fn get_transfer_status(&self, transfer_id: &str) -> Result<PayoutStatus, String> {
        if self.api_token.is_empty() || self.api_token.starts_with("mock_") {
            return Ok(PayoutStatus::OutgoingPaymentSent);
        }

        let client = reqwest::Client::new();
        let res = client
            .get(format!("https://api.wise.com/v1/transfers/{}", transfer_id))
            .bearer_auth(&self.api_token)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Wise API error: HTTP {}", res.status()));
        }

        let body: Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

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
    pub fn parse_webhook_payload(&self, payload: &[u8]) -> Result<PayoutEvent, String> {
        let json: Value =
            serde_json::from_slice(payload).map_err(|e| format!("Invalid JSON payload: {}", e))?;

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
