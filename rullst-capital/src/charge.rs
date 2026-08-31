use crate::error::CapitalError;
use ring::digest::{SHA256, digest};

/// Maximum amount accepted by the portable direct-charge contract.
///
/// Amounts use the currency's smallest unit. The eight-digit ceiling matches
/// the strictest reviewed built-in live adapter and prevents accidental unit
/// mistakes from becoming unbounded provider requests.
pub const MAX_CHARGE_AMOUNT_MINOR: u64 = 99_999_999;

const MAX_PROVIDER_REFERENCE_LEN: usize = 255;
const MAX_EMAIL_LEN: usize = 320;

/// A validated, provider-neutral request for one immediate off-session charge.
///
/// The payment method must already have been collected by the provider and
/// authorized for reuse. This contract has no raw card or bank-data fields.
#[derive(Clone, PartialEq, Eq)]
pub struct ChargeRequest {
    amount_minor: u64,
    currency: String,
    customer_id: String,
    customer_email: String,
    payment_method_id: String,
    idempotency_key: String,
}

impl ChargeRequest {
    /// Creates a fully specified direct-charge request.
    pub fn new(
        amount_minor: u64,
        currency: impl Into<String>,
        customer_id: impl Into<String>,
        customer_email: impl Into<String>,
        payment_method_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        if amount_minor == 0 || amount_minor > MAX_CHARGE_AMOUNT_MINOR {
            return Err(invalid_charge(
                "amount must contain 1 to 99,999,999 currency minor units",
            ));
        }

        let currency = currency.into();
        if currency.len() != 3 || !currency.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            return Err(invalid_charge(
                "currency must be exactly three ASCII letters",
            ));
        }

        let customer_id = customer_id.into();
        validate_reference("customer ID", &customer_id, MAX_PROVIDER_REFERENCE_LEN)?;
        let customer_email = customer_email.into();
        validate_email(&customer_email)?;
        let payment_method_id = payment_method_id.into();
        validate_reference(
            "payment method ID",
            &payment_method_id,
            MAX_PROVIDER_REFERENCE_LEN,
        )?;
        let idempotency_key = idempotency_key.into();
        validate_reference(
            "idempotency key",
            &idempotency_key,
            MAX_PROVIDER_REFERENCE_LEN,
        )?;
        if !idempotency_key.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(invalid_charge(
                "idempotency key must contain only visible ASCII characters",
            ));
        }

        Ok(Self {
            amount_minor,
            currency: currency.to_ascii_lowercase(),
            customer_id,
            customer_email,
            payment_method_id,
            idempotency_key,
        })
    }

    /// Amount in the currency's smallest unit, never a floating-point value.
    pub fn amount_minor(&self) -> u64 {
        self.amount_minor
    }

    /// Normalized three-letter lowercase currency code.
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// Provider-owned customer identifier.
    pub fn customer_id(&self) -> &str {
        &self.customer_id
    }

    /// Validated billing email supplied by the billable entity.
    pub fn customer_email(&self) -> &str {
        &self.customer_email
    }

    /// Provider-tokenized payment method identifier; never raw payment data.
    pub fn payment_method_id(&self) -> &str {
        &self.payment_method_id
    }

    /// Application-owned retry key forwarded to supporting providers.
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }
}

impl std::fmt::Debug for ChargeRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChargeRequest")
            .field("amount_minor", &self.amount_minor)
            .field("currency", &self.currency)
            .field("customer_id", &"[REDACTED]")
            .field("customer_email", &"[REDACTED]")
            .field("payment_method_id", &"[REDACTED]")
            .field("idempotency_key", &"[REDACTED]")
            .finish()
    }
}

/// Accepted states returned by the bounded immediate-charge operation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeStatus {
    /// The provider reports that the payment completed successfully.
    Succeeded,
    /// The provider accepted the payment and is still processing it.
    Processing,
    /// Deterministic offline fixture; no money moved and no entitlement exists.
    Mock,
}

/// Provider-bound result of a live or explicitly mocked direct charge.
#[derive(Clone, PartialEq, Eq)]
pub struct ChargeReceipt {
    provider: &'static str,
    charge_id: String,
    status: ChargeStatus,
    amount_minor: u64,
    currency: String,
}

impl ChargeReceipt {
    pub(crate) fn try_new(
        provider: &'static str,
        charge_id: impl Into<String>,
        status: ChargeStatus,
        amount_minor: u64,
        currency: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        if !reference_is_valid(provider, 64) {
            return Err(CapitalError::ProviderRequestFailed(
                "provider returned an invalid adapter name".to_string(),
            ));
        }
        let charge_id = charge_id.into();
        if !reference_is_valid(&charge_id, MAX_PROVIDER_REFERENCE_LEN) {
            return Err(CapitalError::ProviderRequestFailed(
                "provider returned an invalid charge ID".to_string(),
            ));
        }
        let currency = currency.into();
        if amount_minor == 0
            || amount_minor > MAX_CHARGE_AMOUNT_MINOR
            || currency.len() != 3
            || !currency.bytes().all(|byte| byte.is_ascii_lowercase())
        {
            return Err(CapitalError::ProviderRequestFailed(
                "provider returned invalid charge amount or currency".to_string(),
            ));
        }
        Ok(Self {
            provider,
            charge_id,
            status,
            amount_minor,
            currency,
        })
    }

    /// Name of the adapter that accepted the request.
    pub fn provider(&self) -> &'static str {
        self.provider
    }

    /// Opaque provider charge or payment-intent identifier.
    pub fn charge_id(&self) -> &str {
        &self.charge_id
    }

    /// Provider-reported accepted status.
    pub fn status(&self) -> ChargeStatus {
        self.status
    }

    /// Amount bound to the receipt in currency minor units.
    pub fn amount_minor(&self) -> u64 {
        self.amount_minor
    }

    /// Normalized currency bound to the receipt.
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// Returns whether a live provider reported final success.
    pub fn is_succeeded(&self) -> bool {
        self.status == ChargeStatus::Succeeded
    }
}

impl std::fmt::Debug for ChargeReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChargeReceipt")
            .field("provider", &self.provider)
            .field("charge_id", &"[REDACTED]")
            .field("status", &self.status)
            .field("amount_minor", &self.amount_minor)
            .field("currency", &self.currency)
            .finish()
    }
}

pub(crate) fn mock_charge_receipt(
    provider: &'static str,
    request: &ChargeRequest,
) -> Result<ChargeReceipt, CapitalError> {
    let mut material = Vec::with_capacity(512);
    for part in [
        provider,
        request.currency(),
        request.customer_id(),
        request.customer_email(),
        request.payment_method_id(),
        request.idempotency_key(),
    ] {
        material.extend_from_slice(part.as_bytes());
        material.push(0);
    }
    material.extend_from_slice(&request.amount_minor().to_be_bytes());
    let fingerprint = hex::encode(digest(&SHA256, &material));
    let fingerprint = fingerprint.get(..24).ok_or_else(|| {
        CapitalError::ProviderRequestFailed(
            "failed to construct deterministic mock charge ID".to_string(),
        )
    })?;
    ChargeReceipt::try_new(
        provider,
        format!("ch_mock_{fingerprint}"),
        ChargeStatus::Mock,
        request.amount_minor(),
        request.currency(),
    )
}

fn validate_reference(label: &str, value: &str, max_len: usize) -> Result<(), CapitalError> {
    if !reference_is_valid(value, max_len) {
        return Err(invalid_charge(format!(
            "{label} must contain 1 to {max_len} non-control characters without surrounding whitespace"
        )));
    }
    Ok(())
}

fn reference_is_valid(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn validate_email(value: &str) -> Result<(), CapitalError> {
    validate_reference("customer email", value, MAX_EMAIL_LEN)?;
    let mut parts = value.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    if local.is_empty()
        || domain.is_empty()
        || parts.next().is_some()
        || value.chars().any(char::is_whitespace)
    {
        return Err(invalid_charge(
            "customer email must contain one non-empty local and domain part",
        ));
    }
    Ok(())
}

fn invalid_charge(message: impl Into<String>) -> CapitalError {
    CapitalError::InvalidCharge(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ChargeRequest {
        ChargeRequest::new(
            1_099,
            "USD",
            "cus_123",
            "owner@example.com",
            "pm_123",
            "order_123-attempt_1",
        )
        .expect("valid request")
    }

    #[test]
    fn request_validates_money_identity_and_retry_boundaries() {
        let request = request();
        assert_eq!(request.amount_minor(), 1_099);
        assert_eq!(request.currency(), "usd");
        assert_eq!(request.customer_id(), "cus_123");
        assert_eq!(request.payment_method_id(), "pm_123");

        for invalid in [
            ChargeRequest::new(0, "USD", "cus", "a@b", "pm", "key"),
            ChargeRequest::new(
                MAX_CHARGE_AMOUNT_MINOR + 1,
                "USD",
                "cus",
                "a@b",
                "pm",
                "key",
            ),
            ChargeRequest::new(1, "US", "cus", "a@b", "pm", "key"),
            ChargeRequest::new(1, "U$D", "cus", "a@b", "pm", "key"),
            ChargeRequest::new(1, "USD", "", "a@b", "pm", "key"),
            ChargeRequest::new(1, "USD", "cus", "invalid", "pm", "key"),
            ChargeRequest::new(1, "USD", "cus", "a@b", "line\nbreak", "key"),
            ChargeRequest::new(1, "USD", "cus", "a@b", "pm", " "),
            ChargeRequest::new(1, "USD", "cus", "a@b", "pm", "non ascii ç"),
        ] {
            assert!(matches!(invalid, Err(CapitalError::InvalidCharge(_))));
        }
    }

    #[test]
    fn mock_receipt_is_deterministic_and_debug_output_is_redacted() {
        let request = request();
        let first = mock_charge_receipt("stripe", &request).expect("mock receipt");
        let second = mock_charge_receipt("stripe", &request).expect("mock receipt");
        assert_eq!(first, second);
        assert_eq!(first.status(), ChargeStatus::Mock);
        assert!(!first.is_succeeded());
        assert_eq!(first.provider(), "stripe");
        assert_eq!(first.amount_minor(), 1_099);
        assert_eq!(first.currency(), "usd");

        let request_debug = format!("{request:?}");
        let receipt_debug = format!("{first:?}");
        for secret in [
            "cus_123",
            "owner@example.com",
            "pm_123",
            "order_123-attempt_1",
        ] {
            assert!(!request_debug.contains(secret));
        }
        assert!(!receipt_debug.contains(first.charge_id()));
    }
}
