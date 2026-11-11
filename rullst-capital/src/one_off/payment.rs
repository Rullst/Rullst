//! What names a one-off payment, and what the provider confirmed about it.

use super::OneOffCheckoutRequest;
use crate::error::CapitalError;

const MAX_IDENTIFIER_LEN: usize = 255;

/// Which one-off payment to confirm, read from an unauthenticated
/// notification, a return URL or the stored checkout. Since 13.0.0-alpha.5.
///
/// It carries **no payment status**: anyone can send a notification, so
/// nothing in it is trusted. Load the server-side
/// [`OneOffCheckoutRequest`] by [`reference`](Self::reference) and pass both
/// to [`BillingProvider::confirm_one_off_payment`](crate::BillingProvider::confirm_one_off_payment).
/// The identifiers are 1 to 255 ASCII letters, digits, `-`, `_` or `.`.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentNotification {
    reference: String,
    transaction_id: Option<String>,
    invoice_id: Option<String>,
}

impl PaymentNotification {
    /// Names the application's order reference.
    pub fn new(reference: impl Into<String>) -> Result<Self, CapitalError> {
        let reference = reference.into();
        if !super::valid_reference(&reference) {
            return Err(unreadable("order reference"));
        }
        Ok(Self {
            reference,
            transaction_id: None,
            invoice_id: None,
        })
    }

    /// The provider's payment identifier: InfinitePay's `transaction_nsu`,
    /// or the Checkout Session ID (`cs_…`) for Stripe.
    pub fn with_transaction_id(mut self, id: impl Into<String>) -> Result<Self, CapitalError> {
        let id = id.into();
        if !valid_identifier(&id) {
            return Err(unreadable("transaction ID"));
        }
        self.transaction_id = Some(id);
        Ok(self)
    }

    /// The provider's invoice or link identifier: InfinitePay's
    /// `invoice_slug` (`slug` in the return URL).
    pub fn with_invoice_id(mut self, id: impl Into<String>) -> Result<Self, CapitalError> {
        let id = id.into();
        if !valid_identifier(&id) {
            return Err(unreadable("invoice ID"));
        }
        self.invoice_id = Some(id);
        Ok(self)
    }

    /// The order reference of the [`OneOffCheckoutRequest`] to load.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// The provider's payment identifier, when the notification has one.
    pub fn transaction_id(&self) -> Option<&str> {
        self.transaction_id.as_deref()
    }

    /// The provider's invoice or link identifier, when the notification has one.
    pub fn invoice_id(&self) -> Option<&str> {
        self.invoice_id.as_deref()
    }

    /// Fails unless the notification names `request`'s reference.
    pub(crate) fn require_reference_of(
        &self,
        request: &OneOffCheckoutRequest,
    ) -> Result<(), CapitalError> {
        if self.reference == request.reference() {
            Ok(())
        } else {
            Err(CapitalError::InvalidCharge(
                "the notification names another order reference than the request".into(),
            ))
        }
    }
}

/// What the provider confirmed about a one-off payment. Since 13.0.0-alpha.5.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OneOffPaymentState {
    /// The provider confirmed a payment of at least the request's total, in
    /// its currency.
    Paid,
    /// The provider reports no completed payment (yet) for this checkout.
    Unpaid,
    /// Deterministic offline fixture of a `mock_*` adapter: no money moved and
    /// it never proves payment.
    Mock,
}

/// A provider's answer about one [`OneOffCheckoutRequest`], read at the
/// source. Since 13.0.0-alpha.5.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct OneOffPayment {
    provider: &'static str,
    reference: String,
    state: OneOffPaymentState,
    amount_minor: u64,
    paid_amount_minor: u64,
    currency: String,
    transaction_id: Option<String>,
    request_digest: [u8; 32],
}

impl OneOffPayment {
    /// A payment the provider reports as completed, bound to the server-side
    /// request.
    ///
    /// `charged_amount_minor` and `currency` are what the provider says was
    /// charged, `paid_amount_minor` what the buyer paid (more when the buyer
    /// pays the instalment fees). It fails with
    /// [`CapitalError::InvalidCharge`] unless the charged amount equals the
    /// request's total, the currency matches and the paid amount is not lower:
    /// a payment of another checkout that reuses the reference never counts.
    pub fn paid(
        provider: &'static str,
        request: &OneOffCheckoutRequest,
        charged_amount_minor: u64,
        paid_amount_minor: u64,
        currency: &str,
    ) -> Result<Self, CapitalError> {
        if charged_amount_minor != request.total_amount_minor()
            || paid_amount_minor < charged_amount_minor
            || !currency.eq_ignore_ascii_case(request.currency())
        {
            return Err(CapitalError::InvalidCharge(format!(
                "{provider} confirmed a payment that does not match the server-side amount and currency"
            )));
        }
        Ok(Self::new(
            provider,
            request,
            OneOffPaymentState::Paid,
            paid_amount_minor,
        ))
    }

    /// The provider reports no completed payment for the request.
    pub fn unpaid(provider: &'static str, request: &OneOffCheckoutRequest) -> Self {
        Self::new(provider, request, OneOffPaymentState::Unpaid, 0)
    }

    pub(crate) fn mock(provider: &'static str, request: &OneOffCheckoutRequest) -> Self {
        Self::new(provider, request, OneOffPaymentState::Mock, 0)
    }

    fn new(
        provider: &'static str,
        request: &OneOffCheckoutRequest,
        state: OneOffPaymentState,
        paid_amount_minor: u64,
    ) -> Self {
        Self {
            provider,
            reference: request.reference().to_string(),
            state,
            amount_minor: request.total_amount_minor(),
            paid_amount_minor,
            currency: request.currency().to_string(),
            transaction_id: None,
            request_digest: request.request_digest(),
        }
    }

    /// Records the provider's payment identifier.
    pub fn with_transaction_id(mut self, id: impl Into<String>) -> Result<Self, CapitalError> {
        let id = id.into();
        if !valid_identifier(&id) {
            return Err(unreadable("transaction ID"));
        }
        self.transaction_id = Some(id);
        Ok(self)
    }

    /// The adapter that answered.
    pub fn provider(&self) -> &'static str {
        self.provider
    }

    /// The request's order reference.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// What the provider confirmed.
    pub fn state(&self) -> OneOffPaymentState {
        self.state
    }

    /// Whether the provider confirmed the payment. Mock answers are never paid.
    pub fn is_paid(&self) -> bool {
        self.state == OneOffPaymentState::Paid
    }

    /// The request's total, in the currency's smallest unit.
    pub fn amount_minor(&self) -> u64 {
        self.amount_minor
    }

    /// What the buyer paid (0 unless paid); at least [`Self::amount_minor`].
    pub fn paid_amount_minor(&self) -> u64 {
        self.paid_amount_minor
    }

    /// The request's lowercase currency code.
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// The provider's payment identifier, when known.
    pub fn transaction_id(&self) -> Option<&str> {
        self.transaction_id.as_deref()
    }

    /// [`OneOffCheckoutRequest::request_digest`] of the confirmed request.
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
}

impl std::fmt::Debug for OneOffPayment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OneOffPayment")
            .field("provider", &self.provider)
            .field("state", &self.state)
            .field("amount_minor", &self.amount_minor)
            .field("paid_amount_minor", &self.paid_amount_minor)
            .field("currency", &self.currency)
            .finish_non_exhaustive()
    }
}

pub(crate) fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn unreadable(field: &str) -> CapitalError {
    CapitalError::PayloadParseError(format!(
        "payment notification {field} must be 1 to 255 ASCII letters, digits, '-', '_' or '.'"
    ))
}
