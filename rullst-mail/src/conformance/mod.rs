//! Conformance kit for [`MailDriver`] implementations (Cargo feature
//! `conformance`). Since 13.0.0-alpha.5.
//!
//! An application-owned mail transport proves it behaves like the built-in
//! Resend, SES, SendPulse and SMTP drivers by passing [`check_mail_driver`] in
//! its tests:
//!
//! ```ignore
//! let report = check_mail_driver("acmemail", &driver, &capture).await;
//! report.assert_passed();
//! ```
//!
//! The driver is configured for its deterministic offline mode (empty or
//! `mock_*` credentials), so the kit never needs a network. The
//! [`DeliveryCapture`] returns what the offline mode captured instead of
//! sending; [`OfflineMockCapture`] reads the built-in drivers' shared
//! [`OfflineMailMock`](crate::OfflineMailMock).
//!
//! | Case | What a conforming driver does |
//! | --- | --- |
//! | `delivery.accepted` | a valid message is delivered once through `send`, `send_with_delivery_id` and `send_for_tenant` |
//! | `headers.crlf_refused` | CR or LF in the subject, recipient, sender or Reply-To is a `ValidationError` and nothing is delivered |
//! | `attachments.limits_enforced` | too many, too large or badly named attachments are a `ValidationError` and nothing is delivered |
//! | `suppression.respected` | behind a [`SuppressionGuard`](crate::SuppressionGuard), a suppressed recipient gets `SuppressedRecipient` on every path and nothing is delivered |
//! | `offline.deterministic` | the same message sent twice is captured twice, identically |
//! | `errors.typed_no_panic` | invalid recipients, delivery IDs, tenants and subjects are a `ValidationError`, never a panic, and nothing is delivered |
//!
//! Running the shared [`DeliveryPipeline`](crate::DeliveryPipeline) first in
//! every `send` method is what makes a driver pass the refusal cases. Every
//! case runs under `catch_unwind`: a panic fails that case, never the run.

mod cases;
mod report;

pub use report::{CaseOutcome, ConformanceCase, ConformanceReport};

use crate::{MailDriver, MailError, Message, OfflineMailMock};
use report::run_case;

/// What a driver's offline mode captured instead of sending.
/// Since 13.0.0-alpha.5.
pub trait DeliveryCapture: Send + Sync {
    /// Every message captured so far, as the driver would have sent it. The
    /// kit tells its own messages apart by subject, so a capture shared with
    /// other tests works.
    fn captured(&self) -> Result<Vec<Message>, MailError>;
}

/// The [`OfflineMailMock`] deliveries of one built-in provider label
/// (`resend`, `aws_ses`, `sendpulse` or `smtp`). Since 13.0.0-alpha.5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfflineMockCapture {
    provider: String,
}

impl OfflineMockCapture {
    /// Reads the deliveries the offline mock recorded for `provider`.
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
        }
    }
}

impl DeliveryCapture for OfflineMockCapture {
    fn captured(&self) -> Result<Vec<Message>, MailError> {
        Ok(OfflineMailMock::deliveries()?
            .into_iter()
            .filter(|delivery| delivery.provider == self.provider)
            .map(|delivery| delivery.message)
            .collect())
    }
}

/// Runs every case of the mail conformance kit against `driver`.
/// Since 13.0.0-alpha.5.
///
/// It never panics: a panic inside the driver or the capture fails the case
/// it happened in. `name` labels the report. Run it against the offline
/// configuration: it sends several messages.
pub async fn check_mail_driver<D, C>(
    name: impl Into<String>,
    driver: &D,
    capture: &C,
) -> ConformanceReport
where
    D: MailDriver + ?Sized,
    C: DeliveryCapture + ?Sized,
{
    let run = cases::Run::new(driver, capture);
    let cases = vec![
        run_case("delivery.accepted", run.accepted()).await,
        run_case("headers.crlf_refused", run.crlf_refused()).await,
        run_case("attachments.limits_enforced", run.attachment_limits()).await,
        run_case("suppression.respected", run.suppression()).await,
        run_case("offline.deterministic", run.deterministic()).await,
        run_case("errors.typed_no_panic", run.typed_errors()).await,
    ];
    ConformanceReport::new(name, cases)
}
