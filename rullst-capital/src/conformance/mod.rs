//! Conformance kit for [`BillingProvider`] implementations (Cargo feature
//! `conformance`). Since 13.0.0-alpha.5.
//!
//! An application-owned payment adapter proves it behaves like Capital's
//! built-in Stripe and InfinitePay adapters by passing
//! [`check_billing_provider`] in its tests:
//!
//! ```ignore
//! let report = check_billing_provider(&provider, &harness).await;
//! report.assert_passed();
//! ```
//!
//! The provider is configured for its deterministic offline mode (empty or
//! `mock_*` credentials) or pointed at a local fixture; the kit never needs a
//! network. The [`BillingHarness`] supplies what only the implementer knows:
//! the server-side request the provider accepts, an authentic notification,
//! and, optionally, how to make the provider's source of truth report a
//! payment.
//!
//! | Case | What a conforming provider does |
//! | --- | --- |
//! | `mode.declared` | `webhook_verification_mode` returns a mode |
//! | `checkout.inputs_validated` | plan checkout refuses blank e-mail addresses and plans |
//! | `checkout.server_side_amount` | the one-off checkout is bound to the server-side request, or `UnsupportedOperation` without one-off support |
//! | `offline.deterministic` | the same checkout input gives the same result |
//! | `notification.forged_refused` | `handle_webhook` refuses unsigned, re-signed and tampered bodies, and every body of a provider that confirms at the source |
//! | `notification.replay_idempotent` | the same authentic notification read twice gives the same result |
//! | `confirmation.required` | nothing is paid until the source confirms it; a confirmed payment is paid and bound to the request |
//! | `confirmation.amount_currency_bound` | a source payment of another amount, currency or order is never paid |
//! | `errors.typed_no_panic` | hostile input returns a [`CapitalError`], never a panic |
//!
//! Every case runs under `catch_unwind`: a panic fails that case, never the
//! run. Rejecting a replayed delivery is the webhook middleware's job, not the
//! provider's, so the kit checks that a replay gives the same answer.
//!
//! [`CapitalError`]: crate::CapitalError

mod cases;
mod report;

pub use report::{CaseOutcome, ConformanceCase, ConformanceReport};

use crate::{BillingProvider, OneOffCheckoutRequest};
use report::run_case;
use std::collections::HashMap;

/// A raw webhook delivery: body and headers, with lower-case header names as
/// the webhook middleware passes them. Since 13.0.0-alpha.5.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebhookDelivery {
    payload: Vec<u8>,
    headers: HashMap<String, String>,
}

impl WebhookDelivery {
    /// A delivery of `payload` without headers.
    pub fn new(payload: impl Into<Vec<u8>>) -> Self {
        Self {
            payload: payload.into(),
            headers: HashMap::new(),
        }
    }

    /// Adds a header; the name is stored in lower case.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .insert(name.into().to_ascii_lowercase(), value.into());
        self
    }

    /// The raw body.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// The headers, by lower-case name.
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

/// What the provider's source of truth reports about a one-off payment.
/// Since 13.0.0-alpha.5.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePayment {
    paid: bool,
    amount_minor: u64,
    currency: String,
}

impl SourcePayment {
    /// No completed payment.
    pub fn unpaid() -> Self {
        Self {
            paid: false,
            amount_minor: 0,
            currency: String::new(),
        }
    }

    /// A completed payment that charged `amount_minor` in `currency`.
    pub fn paid(amount_minor: u64, currency: impl Into<String>) -> Self {
        Self {
            paid: true,
            amount_minor,
            currency: currency.into(),
        }
    }

    /// Whether the source reports a completed payment.
    pub fn is_paid(&self) -> bool {
        self.paid
    }

    /// The charged amount in the currency's smallest unit; 0 when unpaid.
    pub fn amount_minor(&self) -> u64 {
        self.amount_minor
    }

    /// The charged currency as the kit passes it (lower case); empty when
    /// unpaid.
    pub fn currency(&self) -> &str {
        &self.currency
    }
}

/// The fixtures [`check_billing_provider`] needs from the implementer.
/// Since 13.0.0-alpha.5.
///
/// Only [`one_off_request`](Self::one_off_request) is required. Without an
/// authentic delivery or a source simulation, the cases that need them are
/// skipped or noted in the report, never passed silently.
pub trait BillingHarness: Send + Sync {
    /// A server-side one-off request the provider accepts (its currency and
    /// minimum amount), or `None` when the provider has no one-off checkout.
    fn one_off_request(&self) -> Option<OneOffCheckoutRequest>;

    /// An authentic notification about `request` (or about any event when
    /// `None`), exactly as the provider would deliver it: correctly signed for
    /// [`WebhookVerificationMode::Real`] and
    /// [`WebhookVerificationMode::Mock`], the unsigned body naming the request
    /// for [`WebhookVerificationMode::ConfirmedAtSource`]. Build it fresh on
    /// each call when the signature carries a timestamp.
    ///
    /// [`WebhookVerificationMode::Real`]: crate::WebhookVerificationMode::Real
    /// [`WebhookVerificationMode::Mock`]: crate::WebhookVerificationMode::Mock
    /// [`WebhookVerificationMode::ConfirmedAtSource`]: crate::WebhookVerificationMode::ConfirmedAtSource
    // Its only mutant, `None`, is the default itself.
    #[cfg_attr(mutants, mutants::skip)]
    fn authentic_delivery(
        &self,
        request: Option<&OneOffCheckoutRequest>,
    ) -> Option<WebhookDelivery> {
        let _ = request;
        None
    }

    /// Makes the provider's source of truth (its API, or the fixture standing
    /// in for it) report `payment` for `request` from now on. Returns `false`
    /// when the harness cannot simulate that answer; the kit then notes the
    /// check it could not run. The default simulates nothing.
    fn set_source_payment(&self, request: &OneOffCheckoutRequest, payment: &SourcePayment) -> bool {
        let _ = (request, payment);
        false
    }
}

/// Runs every case of the billing conformance kit against `provider`.
/// Since 13.0.0-alpha.5.
///
/// It never panics: a panic inside the provider or the harness fails the
/// case it happened in. Run it against the offline or fixture configuration:
/// it creates checkouts and confirms payments.
pub async fn check_billing_provider<P, H>(provider: &P, harness: &H) -> ConformanceReport
where
    P: BillingProvider + ?Sized,
    H: BillingHarness + ?Sized,
{
    let subject = cases::Subject { provider, harness };
    let cases = vec![
        run_case("mode.declared", subject.mode_declared()).await,
        run_case("checkout.inputs_validated", subject.checkout_inputs()).await,
        run_case("checkout.server_side_amount", subject.server_side_amount()).await,
        run_case("offline.deterministic", subject.deterministic()).await,
        run_case("notification.forged_refused", subject.forged_refused()).await,
        run_case(
            "notification.replay_idempotent",
            subject.replay_idempotent(),
        )
        .await,
        run_case("confirmation.required", subject.confirmation_required()).await,
        run_case(
            "confirmation.amount_currency_bound",
            subject.amount_currency_bound(),
        )
        .await,
        run_case("errors.typed_no_panic", subject.typed_errors()).await,
    ];
    ConformanceReport::new(provider.name(), cases)
}

#[cfg(test)]
mod tests;
