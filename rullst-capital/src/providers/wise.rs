use super::{PayoutEvent, PayoutProvider, PayoutStatus};
use crate::error::CapitalError;
use async_trait::async_trait;
use serde_json::Value;

/// Payout provider implementation for Wise (Global Multi-Currency B2B Payouts & Disbursements).
pub struct WiseProvider {
    api_token: String,
    _profile_id: String,
}

impl WiseProvider {
    /// Creates a new `WiseProvider` instance.
    pub fn new(api_token: impl Into<String>, profile_id: impl Into<String>) -> Self {
        Self {
            api_token: api_token.into(),
            _profile_id: profile_id.into(),
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

        Err(CapitalError::UnsupportedOperation(
            "Wise transfers require a real recipient account, authenticated quote UUID and durable UUID idempotency identity; the email-based method cannot establish them".into(),
        ))
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

        crate::subscription::validate_provider_subscription_id(transfer_id)?;
        let client = crate::providers::http_client()?;
        let body: Value = crate::providers::send_http_json(
            client
                .get(format!("https://api.wise.com/v1/transfers/{}", transfer_id))
                .bearer_auth(&self.api_token),
            "wise",
            "get transfer status",
        )
        .await?;

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
    /// Normalizes an **unauthenticated** Wise webhook fixture into a `PayoutEvent`.
    ///
    /// This parser performs no signature verification, so it cannot distinguish
    /// a Wise delivery from a forged request. It is restricted to deterministic
    /// offline fixtures selected by an explicit `mock_*` API token. An empty
    /// token returns `ConfigurationError` and any other token returns
    /// `UnsupportedOperation` before the body is read. Never re-issue, release
    /// or reconcile money from its result.
    pub fn parse_webhook_payload(&self, payload: &[u8]) -> Result<PayoutEvent, CapitalError> {
        self.require_webhook_fixture_mode()?;
        let json: Value = serde_json::from_slice(payload)
            .map_err(|e| CapitalError::PayloadParseError(format!("Invalid JSON payload: {}", e)))?;

        // Fixture fields are required: nothing missing is replaced by a default.
        let data = &json["data"];
        let resource = &data["resource"];
        let transfer_id = resource["id"]
            .as_u64()
            .filter(|id| *id > 0)
            .map(|id| id.to_string())
            .ok_or_else(|| fixture_error("missing or invalid transfer ID"))?;
        let recipient_email = resource["recipient_email"]
            .as_str()
            .filter(|email| {
                !email.trim().is_empty()
                    && email.len() <= 254
                    && !email.chars().any(char::is_control)
            })
            .ok_or_else(|| fixture_error("missing or invalid recipient email"))?
            .to_string();
        let currency = resource["currency"]
            .as_str()
            .filter(|code| code.len() == 3 && code.bytes().all(|byte| byte.is_ascii_uppercase()))
            .ok_or_else(|| fixture_error("missing or invalid ISO 4217 currency"))?
            .to_string();
        let amount_cents = fixture_minor_units(&resource["amount"], &currency)?;
        let status = match data["current_state"].as_str() {
            Some(
                "incoming_payment_waiting"
                | "incoming_payment_initiated"
                | "processing"
                | "funds_converted",
            ) => PayoutStatus::Processing,
            Some("outgoing_payment_sent") => PayoutStatus::OutgoingPaymentSent,
            Some("funds_refunded") => PayoutStatus::FundsRefunded,
            Some("cancelled") => PayoutStatus::Cancelled,
            _ => return Err(fixture_error("missing or unsupported current_state")),
        };

        Ok(PayoutEvent {
            transfer_id,
            recipient_email,
            amount_cents,
            currency,
            status,
        })
    }

    // Unlike payout fixtures, an unset token must not enable an unauthenticated
    // webhook parser: only an explicitly named `mock_*` token selects it.
    fn require_webhook_fixture_mode(&self) -> Result<(), CapitalError> {
        if self.api_token.starts_with("mock_") {
            return Ok(());
        }
        if self.api_token.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "Wise webhook fixture parsing requires an explicit mock_* API token".into(),
            ));
        }
        Err(CapitalError::UnsupportedOperation(
            "Wise webhook payload parsing is unauthenticated; live deliveries require X-Signature-SHA256 verification".into(),
        ))
    }
}

fn fixture_error(reason: &str) -> CapitalError {
    CapitalError::PayloadParseError(format!("Wise fixture: {reason}"))
}

// ISO 4217 minor-unit exponent; currencies not listed use two decimals.
fn currency_exponent(currency: &str) -> usize {
    match currency {
        "BIF" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW" | "PYG" | "RWF" | "UGX"
        | "UYI" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
        "CLF" | "UYW" => 4,
        _ => 2,
    }
}

// Scales an exact decimal amount to minor units without floating-point
// arithmetic. JSON numbers are read from their shortest round-trip decimal
// text; negative, exponent, over-precise, zero and overflowing values fail.
fn fixture_minor_units(value: &Value, currency: &str) -> Result<u64, CapitalError> {
    let text = match value {
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        _ => return Err(fixture_error("missing or invalid amount")),
    };
    let exponent = currency_exponent(currency);
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) if !fraction.is_empty() => (whole, fraction),
        Some(_) => return Err(fixture_error("missing or invalid amount")),
        None => (text.as_str(), ""),
    };
    let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if whole.is_empty() || !digits(whole) || !digits(fraction) {
        return Err(fixture_error("amount is not an exact non-negative decimal"));
    }
    // Trailing zeros carry no value, so `1000.0 JPY` is still exact.
    let fraction = fraction.trim_end_matches('0');
    if fraction.len() > exponent {
        return Err(fixture_error("amount has more decimals than its currency"));
    }
    let fraction_units = if fraction.is_empty() {
        Some(0)
    } else {
        format!("{fraction:0<exponent$}").parse::<u64>().ok()
    };
    whole
        .parse::<u64>()
        .ok()
        .and_then(|whole| whole.checked_mul(10_u64.pow(exponent as u32)))
        .zip(fraction_units)
        .and_then(|(units, fraction_units)| units.checked_add(fraction_units))
        .filter(|units| *units > 0)
        .ok_or_else(|| fixture_error("amount is zero or overflows minor units"))
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
        let fixture = |state: &str| {
            format!(
                r#"{{"data":{{"resource":{{"id":1,"recipient_email":"payee@wise.com","amount":"10","currency":"EUR"}},"current_state":"{state}"}}}}"#
            )
        };
        for (state, expected) in [
            ("funds_refunded", PayoutStatus::FundsRefunded),
            ("cancelled", PayoutStatus::Cancelled),
            ("incoming_payment_waiting", PayoutStatus::Processing),
            ("funds_converted", PayoutStatus::Processing),
        ] {
            let event = provider
                .parse_webhook_payload(fixture(state).as_bytes())
                .unwrap();
            assert_eq!(event.status, expected);
        }
        for state in ["other", "charged_back", "bounced_back"] {
            assert!(
                provider
                    .parse_webhook_payload(fixture(state).as_bytes())
                    .is_err()
            );
        }

        // Webhook error paths
        assert!(provider.parse_webhook_payload(b"invalid json").is_err());
    }

    fn fixture_event(resource: Value, state: Option<&str>) -> Result<PayoutEvent, CapitalError> {
        let mut body = serde_json::json!({"data": {"resource": resource}});
        if let Some(state) = state {
            body["data"]["current_state"] = Value::from(state);
        }
        WiseProvider::new("mock_wise_token", "profile")
            .parse_webhook_payload(&serde_json::to_vec(&body).unwrap())
    }

    #[test]
    fn fixture_amounts_are_exact_minor_units_without_float_truncation() {
        let amount = |amount: Value, currency: &str| {
            fixture_event(
                serde_json::json!({"id": 7, "recipient_email": "payee@wise.com", "amount": amount, "currency": currency}),
                Some("outgoing_payment_sent"),
            )
            .map(|event| event.amount_cents)
        };
        for (value, currency, expected) in [
            (serde_json::json!(19.99), "EUR", 1999),
            (serde_json::json!(0.29), "USD", 29),
            (serde_json::json!(1.15), "GBP", 115),
            (serde_json::json!(100.0), "USD", 10000),
            (serde_json::json!("19.90"), "EUR", 1990),
            (serde_json::json!(1000), "JPY", 1000),
            (serde_json::json!(1000.0), "JPY", 1000),
            (serde_json::json!("1.234"), "KWD", 1234),
        ] {
            assert_eq!(
                amount(value.clone(), currency).unwrap(),
                expected,
                "{value} {currency}"
            );
        }
        for (value, currency) in [
            (serde_json::json!(-1), "USD"),
            (serde_json::json!(-0.5), "USD"),
            (serde_json::json!(0), "USD"),
            (serde_json::json!("19.999"), "USD"),
            (serde_json::json!(1.5), "JPY"),
            (serde_json::json!(1e21), "USD"),
            (serde_json::json!("1."), "USD"),
            (serde_json::json!(".5"), "USD"),
            (serde_json::json!("18446744073709551615"), "USD"),
            (serde_json::json!("ten"), "USD"),
            (serde_json::json!(true), "USD"),
        ] {
            assert!(
                amount(value.clone(), currency).is_err(),
                "{value} {currency}"
            );
        }
    }

    #[test]
    fn fixture_fields_are_required_instead_of_invented() {
        let complete = serde_json::json!({"id": 7, "recipient_email": "payee@wise.com", "amount": 5, "currency": "EUR"});
        assert!(fixture_event(complete.clone(), Some("processing")).is_ok());
        assert!(fixture_event(complete.clone(), None).is_err());
        for field in ["id", "recipient_email", "amount", "currency"] {
            let mut resource = complete.clone();
            resource.as_object_mut().unwrap().remove(field);
            assert!(
                fixture_event(resource, Some("processing")).is_err(),
                "{field}"
            );
        }
        for (field, wrong) in [
            ("id", serde_json::json!(0)),
            ("id", serde_json::json!(-3)),
            ("id", serde_json::json!("7")),
            ("recipient_email", serde_json::json!("  ")),
            ("currency", serde_json::json!("usd")),
            ("currency", serde_json::json!("EURO")),
        ] {
            let mut resource = complete.clone();
            resource[field] = wrong;
            assert!(
                fixture_event(resource, Some("processing")).is_err(),
                "{field}"
            );
        }
    }
}
