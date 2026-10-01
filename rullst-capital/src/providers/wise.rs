use super::{PayoutEvent, PayoutProvider, PayoutStatus, WiseTransferState};
use crate::error::{CapitalError, ProviderFailure};
use async_trait::async_trait;
use serde_json::Value;

const MOCK_TRANSFER_PREFIX: &str = "wise_tr_mock_";
const WISE_API_BASE: &str = "https://api.wise.com";
const WISE_SANDBOX_API_BASE: &str = "https://api.sandbox.transferwise.tech";

// Deterministic offline transfer ID that does not embed the recipient's email.
fn mock_transfer_id(recipient_email: &str, amount_cents: u64, currency: &str) -> String {
    let mut digest = ring::digest::Context::new(&ring::digest::SHA256);
    for field in [
        b"rullst-wise-mock-transfer-v1".as_slice(),
        recipient_email.as_bytes(),
        &amount_cents.to_be_bytes(),
        currency.as_bytes(),
    ] {
        digest.update(&(field.len() as u64).to_be_bytes());
        digest.update(field);
    }
    let digest = digest.finish();
    format!(
        "{MOCK_TRANSFER_PREFIX}{}",
        hex::encode(&digest.as_ref()[..8])
    )
}

/// Payout provider implementation for Wise (Global Multi-Currency B2B Payouts & Disbursements).
pub struct WiseProvider {
    api_token: String,
    _profile_id: String,
    api_base: &'static str,
    // RSAPublicKey DER values accepted for signed webhooks.
    pub(super) webhook_keys: Vec<Vec<u8>>,
}

impl WiseProvider {
    /// Creates a new `WiseProvider` instance.
    pub fn new(api_token: impl Into<String>, profile_id: impl Into<String>) -> Self {
        Self {
            api_token: api_token.into(),
            _profile_id: profile_id.into(),
            api_base: WISE_API_BASE,
            webhook_keys: Vec::new(),
        }
    }

    /// Sends authenticated reads to Wise's sandbox API
    /// (`https://api.sandbox.transferwise.tech`) instead of production. Use it
    /// with a sandbox API token and the sandbox webhook key. New in 13.0.
    pub fn with_sandbox_api(mut self) -> Self {
        self.api_base = WISE_SANDBOX_API_BASE;
        self
    }

    /// Reads the typed Wise transfer state, including `bounced_back` and
    /// `charged_back`, which [`PayoutStatus`] cannot represent. The response
    /// must name the requested positive decimal transfer ID. The offline mock
    /// reports `OutgoingPaymentSent` only for transfers it issued. New in 13.0.
    pub async fn get_transfer_state(
        &self,
        transfer_id: &str,
    ) -> Result<WiseTransferState, CapitalError> {
        self.transfer_state(transfer_id).await
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
            return Ok(mock_transfer_id(recipient_email, amount_cents, currency));
        }

        Err(CapitalError::UnsupportedOperation(
            "Wise transfers require a real recipient account, authenticated quote UUID and durable UUID idempotency identity; the email-based method cannot establish them".into(),
        ))
    }

    async fn get_transfer_status(&self, transfer_id: &str) -> Result<PayoutStatus, CapitalError> {
        legacy_payout_status(self.transfer_state(transfer_id).await?)
    }
}

impl WiseProvider {
    async fn transfer_state(&self, transfer_id: &str) -> Result<WiseTransferState, CapitalError> {
        if transfer_id.trim().is_empty() {
            return Err(CapitalError::SubscriptionError(
                "Transfer ID cannot be empty".to_string(),
            ));
        }

        if self.api_token.is_empty() || self.api_token.starts_with("mock_") {
            // The offline mock reports only on transfers it issued itself; it
            // never claims that a real Wise transfer was sent.
            if transfer_id.starts_with(MOCK_TRANSFER_PREFIX) {
                return Ok(WiseTransferState::OutgoingPaymentSent);
            }
            return Err(CapitalError::UnsupportedOperation(
                "the offline Wise mock cannot report the status of a transfer it did not issue"
                    .into(),
            ));
        }

        transfer_state_at(&self.api_token, self.api_base, transfer_id).await
    }
}

/// Reads one transfer and binds the response to the requested numeric ID.
async fn transfer_state_at(
    api_token: &str,
    api_base: &str,
    transfer_id: &str,
) -> Result<WiseTransferState, CapitalError> {
    crate::subscription::validate_provider_subscription_id(transfer_id)?;
    let transfer_id = transfer_id
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0 && id.to_string() == transfer_id)
        .ok_or_else(|| {
            CapitalError::SubscriptionError(
                "Wise transfer ID must be a positive decimal number".to_string(),
            )
        })?;
    let client = crate::providers::http_client()?;
    let body: Value = crate::providers::send_http_json(
        client
            .get(format!("{api_base}/v1/transfers/{transfer_id}"))
            .bearer_auth(api_token),
        "wise",
        "get transfer status",
    )
    .await?;
    bind_transfer_state(transfer_id, &body)
}

/// A response for another transfer, or a missing or undocumented state
/// (including Wise's `unknown`), is a contract failure, never "processing".
fn bind_transfer_state(transfer_id: u64, body: &Value) -> Result<WiseTransferState, CapitalError> {
    let mismatch = || {
        CapitalError::from(ProviderFailure::contract_mismatch(
            "wise",
            "get transfer status",
        ))
    };
    if body["id"].as_u64() != Some(transfer_id) {
        return Err(mismatch());
    }
    body["status"]
        .as_str()
        .and_then(WiseTransferState::parse)
        .ok_or_else(mismatch)
}

/// Bounced-back and charged-back transfers failed; the coarse legacy status
/// cannot express that, so they are reported as an error instead of in flight.
fn legacy_payout_status(state: WiseTransferState) -> Result<PayoutStatus, CapitalError> {
    state.payout_status().ok_or_else(|| {
        CapitalError::UnsupportedOperation(format!(
            "Wise transfer is {}; PayoutStatus cannot represent it and it is not in flight (use WiseProvider::get_transfer_state)",
            state.as_str()
        ))
    })
}

impl WiseProvider {
    /// Normalizes an **unauthenticated** Wise webhook fixture into a `PayoutEvent`.
    ///
    /// This parser performs no signature verification, so it cannot distinguish
    /// a Wise delivery from a forged request. It is restricted to deterministic
    /// offline fixtures selected by an explicit `mock_*` API token. An empty
    /// token returns `ConfigurationError` and any other token returns
    /// `UnsupportedOperation` before the body is read. Never re-issue, release
    /// or reconcile money from its result. Live deliveries use
    /// [`WiseProvider::verify_transfer_state_change`].
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
            "Wise webhook payload parsing is unauthenticated; use verify_transfer_state_change for live deliveries".into(),
        ))
    }
}

fn fixture_error(reason: &str) -> CapitalError {
    CapitalError::PayloadParseError(format!("Wise fixture: {reason}"))
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
    let exponent = crate::currency::minor_unit_exponent(currency) as usize;
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
#[path = "wise_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "wise_status_tests.rs"]
mod status_tests;
