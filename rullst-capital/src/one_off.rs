//! Provider-neutral one-off checkout: one purchase priced on the server,
//! confirmed afterwards by querying the provider.
//!
//! [`OneOffCheckoutRequest`] is what the application authorizes and persists
//! before it calls [`BillingProvider::create_one_off_checkout`]. The provider
//! answers with a hosted [`OneOffCheckout`] URL. When the provider later names
//! the payment (a webhook body, a return query or the stored session ID), the
//! application loads its request by [`PaymentNotification::reference`] and
//! calls [`BillingProvider::confirm_one_off_payment`], whose
//! [`OneOffPayment`] is the only value that reports the purchase as paid.
//!
//! [`BillingProvider::create_one_off_checkout`]: crate::BillingProvider::create_one_off_checkout
//! [`BillingProvider::confirm_one_off_payment`]: crate::BillingProvider::confirm_one_off_payment

use crate::error::CapitalError;
use ring::digest::{Context, SHA256};

mod payment;
pub use payment::{OneOffPayment, OneOffPaymentState, PaymentNotification};

const MAX_REFERENCE_LEN: usize = 200;
const MAX_DESCRIPTION_CHARS: usize = 250;
const MAX_EMAIL_LEN: usize = 320;
/// Largest quantity of one checkout line.
pub const MAX_ONE_OFF_QUANTITY: u32 = 10_000;

/// One purchase priced on the server: what the buyer pays and where the
/// browser returns. Since 13.0.0-alpha.5.
///
/// Build it from server-owned data (a product, an order, a stored charge),
/// never from fields the browser sent, and persist it under its
/// [`reference`](Self::reference) before calling the provider: the same
/// values confirm the payment later. The total is `unit_amount_minor ×
/// quantity` in the currency's smallest unit and must not exceed
/// [`MAX_CHARGE_AMOUNT_MINOR`](crate::MAX_CHARGE_AMOUNT_MINOR).
///
/// `success_url` and `cancel_url` must be bounded credential-free URLs without
/// fragments. HTTPS is required when a live key sends the request; `http://`
/// on `localhost`, `127.0.0.1` or `[::1]` is accepted for a `mock_*` key, so a
/// local development server works offline.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct OneOffCheckoutRequest {
    reference: String,
    unit_amount_minor: u64,
    quantity: u32,
    currency: String,
    description: String,
    success_url: String,
    cancel_url: String,
    customer_email: Option<String>,
}

impl OneOffCheckoutRequest {
    /// Creates a request for one unit.
    ///
    /// `reference` is an opaque application key (1 to 200 ASCII letters,
    /// digits, `-` or `_`), not an e-mail address or a secret; providers
    /// receive it as the order reference. `currency` is a three-letter ISO
    /// 4217 code, stored in lowercase. `description` is the line shown to the
    /// buyer (1 to 250 characters without control characters).
    pub fn new(
        reference: impl Into<String>,
        unit_amount_minor: u64,
        currency: impl Into<String>,
        description: impl Into<String>,
        success_url: impl Into<String>,
        cancel_url: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let reference = reference.into();
        if !valid_reference(&reference) {
            return Err(invalid(
                "reference must contain 1 to 200 ASCII letters, digits, '-' or '_'",
            ));
        }
        if unit_amount_minor == 0 || unit_amount_minor > crate::MAX_CHARGE_AMOUNT_MINOR {
            return Err(invalid(
                "unit amount must contain 1 to 99,999,999 currency minor units",
            ));
        }
        let currency = currency.into();
        if currency.len() != 3 || !currency.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            return Err(invalid("currency must be exactly three ASCII letters"));
        }
        let description = description.into();
        let characters = description.chars().count();
        if characters == 0
            || characters > MAX_DESCRIPTION_CHARS
            || description.trim() != description
            || description.chars().any(char::is_control)
        {
            return Err(invalid(
                "description must contain 1 to 250 characters without control characters or surrounding whitespace",
            ));
        }
        let (success_url, cancel_url) = (success_url.into(), cancel_url.into());
        for url in [&success_url, &cancel_url] {
            crate::providers::validate_redirect_url(url)?;
        }
        Ok(Self {
            reference,
            unit_amount_minor,
            quantity: 1,
            currency: currency.to_ascii_lowercase(),
            description,
            success_url,
            cancel_url,
            customer_email: None,
        })
    }

    /// Sets how many units the buyer pays for (1 to [`MAX_ONE_OFF_QUANTITY`]),
    /// for example one seat per participant of a group class.
    pub fn with_quantity(mut self, quantity: u32) -> Result<Self, CapitalError> {
        if quantity == 0 || quantity > MAX_ONE_OFF_QUANTITY {
            return Err(invalid("quantity must be 1 to 10,000"));
        }
        if self
            .unit_amount_minor
            .checked_mul(u64::from(quantity))
            .is_none_or(|total| total > crate::MAX_CHARGE_AMOUNT_MINOR)
        {
            return Err(invalid(
                "total amount must not exceed 99,999,999 currency minor units",
            ));
        }
        self.quantity = quantity;
        Ok(self)
    }

    /// Pre-fills the buyer's e-mail address on the hosted checkout.
    pub fn with_customer_email(mut self, email: impl Into<String>) -> Result<Self, CapitalError> {
        let email = email.into();
        let mut parts = email.split('@');
        let valid = (3..=MAX_EMAIL_LEN).contains(&email.len())
            && !email
                .chars()
                .any(|character| character.is_whitespace() || character.is_control())
            && parts.next().is_some_and(|local| !local.is_empty())
            && parts.next().is_some_and(|domain| !domain.is_empty())
            && parts.next().is_none();
        if !valid {
            return Err(invalid("customer email must be one address"));
        }
        self.customer_email = Some(email);
        Ok(self)
    }

    /// The application's opaque order reference.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Price of one unit in the currency's smallest unit.
    pub fn unit_amount_minor(&self) -> u64 {
        self.unit_amount_minor
    }

    /// Number of units.
    pub fn quantity(&self) -> u32 {
        self.quantity
    }

    /// `unit_amount_minor × quantity`, the amount the buyer must pay.
    pub fn total_amount_minor(&self) -> u64 {
        // Bounded by `with_quantity`, so the product never overflows.
        self.unit_amount_minor
            .saturating_mul(u64::from(self.quantity))
    }

    /// Lowercase three-letter currency code.
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// The line shown to the buyer.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Where the browser returns after paying; never a payment confirmation.
    pub fn success_url(&self) -> &str {
        &self.success_url
    }

    /// Where the browser returns when the buyer gives up.
    pub fn cancel_url(&self) -> &str {
        &self.cancel_url
    }

    /// The pre-filled buyer e-mail address, when set.
    pub fn customer_email(&self) -> Option<&str> {
        self.customer_email.as_deref()
    }

    /// Versioned SHA-256 digest of every field, to compare a persisted
    /// attempt with the request that confirms it.
    pub fn request_digest(&self) -> [u8; 32] {
        let mut hash = Context::new(&SHA256);
        hash.update(b"rullst.one-off-checkout.v1\0");
        hash.update(&self.unit_amount_minor.to_be_bytes());
        hash.update(&self.quantity.to_be_bytes());
        for field in [
            self.reference.as_str(),
            self.currency.as_str(),
            self.description.as_str(),
            self.success_url.as_str(),
            self.cancel_url.as_str(),
            self.customer_email.as_deref().unwrap_or(""),
        ] {
            hash.update(&(field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
        let mut digest = [0; 32];
        digest.copy_from_slice(hash.finish().as_ref());
        digest
    }

    /// Provider idempotency key: a retry of the same request reuses it.
    pub(crate) fn idempotency_key(&self) -> String {
        format!("rullst-one-off-{}", hex::encode(self.request_digest()))
    }
}

impl std::fmt::Debug for OneOffCheckoutRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OneOffCheckoutRequest")
            .field("unit_amount_minor", &self.unit_amount_minor)
            .field("quantity", &self.quantity)
            .field("currency", &self.currency)
            .field(
                "customer_email",
                &self.customer_email.as_ref().map(|_| "[REDACTED]"),
            )
            .finish_non_exhaustive()
    }
}

/// A hosted checkout created for a [`OneOffCheckoutRequest`]; it never proves
/// payment. Since 13.0.0-alpha.5.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct OneOffCheckout {
    provider: &'static str,
    url: String,
    session_id: Option<String>,
    mock: bool,
    reference: String,
    request_digest: [u8; 32],
}

impl OneOffCheckout {
    /// Binds the hosted checkout URL a provider returned to the request.
    ///
    /// Custom adapters call it after checking the provider's response; the URL
    /// must be a bounded credential-free HTTPS URL without a fragment.
    pub fn from_verified_provider_response(
        provider: &'static str,
        request: &OneOffCheckoutRequest,
        url: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let url = crate::providers::validate_checkout_url(provider, &url.into())?;
        Ok(Self {
            provider,
            url,
            session_id: None,
            mock: false,
            reference: request.reference.clone(),
            request_digest: request.request_digest(),
        })
    }

    /// Records the provider's session or link identifier, to persist with the
    /// request and confirm the payment with later.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Result<Self, CapitalError> {
        let session_id = session_id.into();
        if !payment::valid_identifier(&session_id) {
            return Err(crate::ProviderFailure::contract_mismatch(
                self.provider,
                "one-off checkout",
            )
            .into());
        }
        self.session_id = Some(session_id);
        Ok(self)
    }

    pub(crate) fn mock(provider: &'static str, request: &OneOffCheckoutRequest) -> Self {
        let digest = hex::encode(request.request_digest());
        Self {
            provider,
            url: format!("https://mock.{provider}.invalid/checkout/one_off_{digest}"),
            session_id: Some(format!("mock_{digest}")),
            mock: true,
            reference: request.reference.clone(),
            request_digest: request.request_digest(),
        }
    }

    /// The adapter that created the checkout.
    pub fn provider(&self) -> &'static str {
        self.provider
    }

    /// The hosted checkout URL to redirect the buyer to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The provider's session identifier (Stripe's `cs_…`), when it has one.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Whether this is a deterministic offline fixture (`mock_*` credentials).
    pub fn is_mock(&self) -> bool {
        self.mock
    }

    /// The request's order reference.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// [`OneOffCheckoutRequest::request_digest`] of the request.
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
}

impl std::fmt::Debug for OneOffCheckout {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OneOffCheckout")
            .field("provider", &self.provider)
            .field("mock", &self.mock)
            .finish_non_exhaustive()
    }
}

fn valid_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_REFERENCE_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn invalid(message: &str) -> CapitalError {
    CapitalError::InvalidCharge(message.to_string())
}

#[cfg(test)]
#[path = "one_off/tests.rs"]
mod tests;
