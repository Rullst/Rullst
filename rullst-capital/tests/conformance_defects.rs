//! The billing conformance kit against deliberately broken providers, each
//! failing its case, and the report it returns.
#![cfg(feature = "conformance")]

mod conformance_support;

use async_trait::async_trait;
use conformance_support::*;
use rullst_capital::conformance::{CaseOutcome, SourcePayment, check_billing_provider};
use rullst_capital::{
    BillingProvider, CapitalError, OneOffCheckout, OneOffCheckoutRequest, OneOffPayment,
    PaymentNotification, StripeProvider, SubscriptionStatus, WebhookEvent, WebhookVerificationMode,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// Ways a provider can break the contract, one case each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Defect {
    UndeclaredMode,
    AcceptsBlankPlan,
    RebindsCheckout,
    MislabelsCheckout,
    NoOneOff,
    NonDeterministic,
    TrimsSignedBody,
    AcceptsUnsigned,
    ReplayDiffers,
    TrustsNotification,
    NeverPaid,
    RebindsPayment,
    MislabelsPayment,
    AcceptsOverpayment,
    AcceptsUnderpayment,
    IgnoresCurrency,
    IgnoresReference,
    AcceptsEmptyIds,
    Panics,
}

/// Stripe's offline adapter with a ledger standing in for its API, and an
/// optional defect.
struct Probe {
    stripe: StripeProvider,
    ledger: Ledger,
    defect: Option<Defect>,
    calls: AtomicU32,
}

impl Probe {
    fn new(defect: Option<Defect>, ledger: &Ledger) -> Self {
        Self {
            stripe: StripeProvider::new("mock_sk_probe", "whsec_probe"),
            ledger: Arc::clone(ledger),
            defect,
            calls: AtomicU32::new(0),
        }
    }
    fn has(&self, defect: Defect) -> bool {
        self.defect == Some(defect)
    }
    fn tick(&self) -> u32 {
        self.calls.fetch_add(1, Ordering::Relaxed)
    }
}

#[async_trait]
impl BillingProvider for Probe {
    fn name(&self) -> &'static str {
        "probe"
    }
    fn webhook_verification_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        match self.has(Defect::UndeclaredMode) {
            true => Err(CapitalError::ConfigurationError("undeclared".into())),
            false => self.stripe.webhook_verification_mode(),
        }
    }
    async fn create_checkout_session(
        &self,
        email: &str,
        plan: &str,
        url: &str,
    ) -> Result<String, CapitalError> {
        if self.has(Defect::AcceptsBlankPlan) && plan.trim().is_empty() {
            return Ok("https://probe.invalid/checkout".into());
        }
        let session = self
            .stripe
            .create_checkout_session(email, plan, url)
            .await?;
        match self.has(Defect::NonDeterministic) {
            true => Ok(format!("{session}&n={}", self.tick())),
            false => Ok(session),
        }
    }
    async fn create_one_off_checkout(
        &self,
        request: &OneOffCheckoutRequest,
    ) -> Result<OneOffCheckout, CapitalError> {
        let url = format!("https://probe.invalid/pay/{}", request.reference());
        match self.defect {
            Some(Defect::NoOneOff) => Err(CapitalError::UnsupportedOperation("no one-off".into())),
            Some(Defect::RebindsCheckout) => {
                let cheaper = OneOffCheckoutRequest::new(
                    request.reference(),
                    1,
                    request.currency(),
                    "x",
                    request.success_url(),
                    request.cancel_url(),
                )?;
                OneOffCheckout::from_verified_provider_response("probe", &cheaper, url)
            }
            Some(Defect::MislabelsCheckout) => {
                OneOffCheckout::from_verified_provider_response("other", request, url)
            }
            _ => OneOffCheckout::from_verified_provider_response("probe", request, url)?
                .with_session_id("cs_probe"),
        }
    }
    async fn confirm_one_off_payment(
        &self,
        request: &OneOffCheckoutRequest,
        notification: &PaymentNotification,
    ) -> Result<OneOffPayment, CapitalError> {
        let source = self.ledger.lock().unwrap().clone();
        let (total, currency) = (request.total_amount_minor(), request.currency());
        if notification.reference() != request.reference() && !self.has(Defect::IgnoresReference) {
            return Err(CapitalError::InvalidCharge("another order".into()));
        }
        let lie = |holds: bool| {
            holds.then(|| OneOffPayment::paid("probe", request, total, total, currency))
        };
        let forced = match self.defect {
            Some(Defect::TrustsNotification) => lie(true),
            Some(Defect::NeverPaid) => Some(Ok(OneOffPayment::unpaid("probe", request))),
            Some(Defect::AcceptsOverpayment) => {
                lie(source.is_paid() && source.amount_minor() >= total)
            }
            Some(Defect::AcceptsUnderpayment) => {
                lie(source.is_paid() && source.amount_minor() <= total)
            }
            Some(Defect::IgnoresCurrency) => {
                lie(source.is_paid() && source.amount_minor() == total)
            }
            Some(Defect::MislabelsPayment) => Some(Ok(OneOffPayment::unpaid("other", request))),
            Some(Defect::RebindsPayment) => {
                let other = OneOffCheckoutRequest::new(
                    "order-2",
                    total,
                    currency,
                    "x",
                    request.success_url(),
                    request.cancel_url(),
                )?;
                Some(Ok(OneOffPayment::unpaid("probe", &other)))
            }
            _ => None,
        };
        match (forced, source.is_paid()) {
            (Some(answer), _) => answer,
            (None, true) => OneOffPayment::paid(
                "probe",
                request,
                source.amount_minor(),
                source.amount_minor(),
                source.currency(),
            ),
            (None, false) => Ok(OneOffPayment::unpaid("probe", request)),
        }
    }
    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        match self.defect {
            Some(Defect::Panics) => panic!("probe webhook panic"),
            Some(Defect::TrimsSignedBody) => self
                .stripe
                .handle_webhook(payload.trim_ascii_end(), headers),
            Some(Defect::AcceptsUnsigned) => Ok(WebhookEvent {
                subscription_id: "sub_forged".into(),
                customer_id: "cus_forged".into(),
                customer_email: String::new(),
                plan_id: "price_pro".into(),
                status: SubscriptionStatus::Active,
                ends_at: None,
            }),
            Some(Defect::ReplayDiffers) if self.tick() % 2 == 1 => {
                Err(CapitalError::WebhookReplay("seen".into()))
            }
            _ => self.stripe.handle_webhook(payload, headers),
        }
    }
    async fn create_customer_portal(&self, email: &str, url: &str) -> Result<String, CapitalError> {
        self.stripe.create_customer_portal(email, url).await
    }
    async fn cancel_subscription(&self, id: &str) -> Result<(), CapitalError> {
        match self.has(Defect::AcceptsEmptyIds) {
            true => Ok(()),
            false => self.stripe.cancel_subscription(id).await,
        }
    }
    async fn pause_subscription(&self, id: &str) -> Result<(), CapitalError> {
        self.stripe.pause_subscription(id).await
    }
    async fn report_usage(
        &self,
        id: &str,
        metric: &str,
        quantity: u64,
    ) -> Result<(), CapitalError> {
        self.stripe.report_usage(id, metric, quantity).await
    }
    async fn apply_coupon(&self, id: &str, coupon: &str) -> Result<(), CapitalError> {
        self.stripe.apply_coupon(id, coupon).await
    }
    async fn extend_trial(&self, id: &str, ends_at: i64) -> Result<(), CapitalError> {
        self.stripe.extend_trial(id, ends_at).await
    }
}

fn probe_harness(ledger: &Ledger) -> Harness {
    Harness {
        request: usd_request(),
        delivery: Some(Box::new(|_: Option<&OneOffCheckoutRequest>| {
            stripe_signed(SUBSCRIPTION_EVENT, "whsec_probe")
        })),
        ledger: Some(Arc::clone(ledger)),
    }
}

fn ledger() -> Ledger {
    Arc::new(Mutex::new(SourcePayment::unpaid()))
}

#[tokio::test]
async fn a_conforming_provider_exercises_every_path() {
    let ledger = ledger();
    let report = check_billing_provider(&Probe::new(None, &ledger), &probe_harness(&ledger)).await;
    report.assert_passed();
    for case in report.cases() {
        assert_eq!(
            (case.outcome(), case.detail()),
            (CaseOutcome::Passed, ""),
            "{}",
            case.id()
        );
    }
}

#[tokio::test]
async fn each_defect_fails_its_case() {
    use Defect::*;
    for (defect, id) in [
        (UndeclaredMode, "mode.declared"),
        (AcceptsBlankPlan, "checkout.inputs_validated"),
        (RebindsCheckout, "checkout.server_side_amount"),
        (MislabelsCheckout, "checkout.server_side_amount"),
        (NoOneOff, "checkout.server_side_amount"),
        (NonDeterministic, "offline.deterministic"),
        (TrimsSignedBody, "notification.forged_refused"),
        (AcceptsUnsigned, "notification.forged_refused"),
        (Panics, "notification.forged_refused"),
        (ReplayDiffers, "notification.replay_idempotent"),
        (TrustsNotification, "confirmation.required"),
        (NeverPaid, "confirmation.required"),
        (RebindsPayment, "confirmation.required"),
        (MislabelsPayment, "confirmation.required"),
        (AcceptsOverpayment, "confirmation.amount_currency_bound"),
        (AcceptsUnderpayment, "confirmation.amount_currency_bound"),
        (IgnoresCurrency, "confirmation.amount_currency_bound"),
        (IgnoresReference, "confirmation.amount_currency_bound"),
        (AcceptsEmptyIds, "errors.typed_no_panic"),
    ] {
        let ledger = ledger();
        let report =
            check_billing_provider(&Probe::new(Some(defect), &ledger), &probe_harness(&ledger))
                .await;
        assert_eq!(
            outcome(&report, id),
            CaseOutcome::Failed,
            "{defect:?}\n{report}"
        );
        assert!(!report.passed(), "{defect:?}");
    }
}

#[tokio::test]
async fn a_panic_fails_its_case_and_the_run_continues() {
    let ledger = ledger();
    let report = check_billing_provider(
        &Probe::new(Some(Defect::Panics), &ledger),
        &probe_harness(&ledger),
    )
    .await;
    let forged = report.case("notification.forged_refused").unwrap();
    assert_eq!(forged.detail(), "panicked: probe webhook panic");
    assert_eq!(report.cases().len(), 9);
    assert_eq!(
        outcome(&report, "errors.typed_no_panic"),
        CaseOutcome::Failed
    );
    assert_eq!(
        outcome(&report, "confirmation.required"),
        CaseOutcome::Passed
    );
}

#[tokio::test]
async fn missing_fixtures_skip_or_note_instead_of_passing_silently() {
    // No one-off support and no request: the probe request must be refused.
    let ledger = ledger();
    let bare = Harness {
        request: None,
        delivery: None,
        ledger: None,
    };
    let report = check_billing_provider(&Probe::new(Some(Defect::NoOneOff), &ledger), &bare).await;
    report.assert_passed();
    for id in [
        "notification.replay_idempotent",
        "confirmation.required",
        "confirmation.amount_currency_bound",
    ] {
        assert_eq!(outcome(&report, id), CaseOutcome::Skipped, "{id}");
    }
    assert_eq!(
        report.case("confirmation.required").unwrap().detail(),
        "the provider has no one-off checkout"
    );
    assert_eq!(
        report
            .case("notification.replay_idempotent")
            .unwrap()
            .detail(),
        "the harness supplies no authentic delivery"
    );
    assert_eq!(
        report.case("offline.deterministic").unwrap().detail(),
        "no one-off request, so only plan checkout was compared"
    );
    assert_eq!(
        report.case("notification.forged_refused").unwrap().detail(),
        "no authentic delivery, so only invented forgeries were sent"
    );

    // A provider with one-off support must not hide it behind a missing request.
    let report = check_billing_provider(&Probe::new(None, &ledger), &bare).await;
    assert_eq!(
        outcome(&report, "checkout.server_side_amount"),
        CaseOutcome::Failed
    );
}

#[tokio::test]
async fn the_report_lists_every_case() {
    let ledger = ledger();
    let report = check_billing_provider(
        &Probe::new(Some(Defect::AcceptsEmptyIds), &ledger),
        &probe_harness(&ledger),
    )
    .await;
    let text = report.to_string();
    assert!(
        text.starts_with("conformance of probe: 8 passed, 1 failed, 0 skipped\n"),
        "{text}"
    );
    assert!(text.contains("\n  ok mode.declared\n"), "{text}");
    assert!(
        text.ends_with(
            "  FAILED errors.typed_no_panic: cancel_subscription refuses an empty subscription ID\n"
        ),
        "{text}"
    );
    assert_eq!(
        report.failures().map(|case| case.id()).collect::<Vec<_>>(),
        ["errors.typed_no_panic"]
    );
    assert!(report.case("no.such.case").is_none());

    let bare = Harness {
        request: None,
        delivery: None,
        ledger: None,
    };
    let skipped = check_billing_provider(&Probe::new(Some(Defect::NoOneOff), &ledger), &bare)
        .await
        .to_string();
    assert!(
        skipped.starts_with("conformance of probe: 6 passed, 0 failed, 3 skipped\n"),
        "{skipped}"
    );
    assert!(
        skipped
            .contains("\n  skipped confirmation.required: the provider has no one-off checkout\n"),
        "{skipped}"
    );
}

#[tokio::test]
#[should_panic(expected = "FAILED errors.typed_no_panic")]
async fn assert_passed_fails_the_test_with_the_report() {
    let ledger = ledger();
    check_billing_provider(
        &Probe::new(Some(Defect::AcceptsEmptyIds), &ledger),
        &probe_harness(&ledger),
    )
    .await
    .assert_passed();
}
