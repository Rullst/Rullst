//! The cases of the billing conformance kit, one method each.

use super::report::Check;
use super::{BillingHarness, SourcePayment, WebhookDelivery};
use crate::{
    BillingProvider, CapitalError, OneOffCheckoutRequest, OneOffPayment, PaymentNotification,
    WebhookVerificationMode,
};
use std::collections::HashMap;

const EMAIL: &str = "buyer@conformance.invalid";
const PLAN: &str = "plan_conformance";
const RETURN_URL: &str = "https://conformance.invalid/return";
const CANCEL_URL: &str = "https://conformance.invalid/cancel";
const PROBE_REFERENCE: &str = "rullst-conformance-probe";
const OTHER_REFERENCE: &str = "rullst-conformance-other";
/// Larger than the 2 MiB the webhook middleware accepts.
const OVERSIZED_BODY: usize = 3 * 1024 * 1024;

/// The provider under test and the implementer's fixtures.
pub(super) struct Subject<'a, P: ?Sized, H: ?Sized> {
    pub(super) provider: &'a P,
    pub(super) harness: &'a H,
}

impl<P, H> Subject<'_, P, H>
where
    P: BillingProvider + ?Sized,
    H: BillingHarness + ?Sized,
{
    fn mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        self.provider.webhook_verification_mode()
    }

    pub(super) async fn mode_declared(&self) -> Check {
        let mut check = Check::default();
        if let Err(error) = self.mode() {
            check.expect(
                false,
                format!("webhook_verification_mode() declares a mode ({error}); the default refusal keeps the provider out of the webhook middleware"),
            );
        }
        check
    }

    pub(super) async fn checkout_inputs(&self) -> Check {
        let mut check = Check::default();
        for (email, plan) in [("", PLAN), ("  ", PLAN), (EMAIL, ""), (EMAIL, "  ")] {
            let result = self
                .provider
                .create_checkout_session(email, plan, RETURN_URL)
                .await;
            check.expect(
                result.is_err(),
                format!("create_checkout_session refuses e-mail {email:?} with plan {plan:?}"),
            );
        }
        check
    }

    pub(super) async fn server_side_amount(&self) -> Check {
        let mut check = Check::default();
        let Some(request) = self.harness.one_off_request() else {
            let result = match probe_request() {
                Ok(probe) => self.provider.create_one_off_checkout(&probe).await,
                Err(error) => Err(error),
            };
            check.expect(
                matches!(result, Err(CapitalError::UnsupportedOperation(_))),
                "without one-off support, create_one_off_checkout returns UnsupportedOperation",
            );
            return check;
        };
        match self.provider.create_one_off_checkout(&request).await {
            Ok(checkout) => {
                check.expect(
                    checkout.provider() == self.provider.name(),
                    "the checkout names the provider",
                );
                check.expect(
                    checkout.request_digest() == request.request_digest(),
                    "the checkout is bound to the server-side request: amount, quantity, currency, reference and URLs",
                );
            }
            Err(error) => check.expect(
                false,
                format!("create_one_off_checkout accepts the harness request: {error}"),
            ),
        }
        check
    }

    pub(super) async fn deterministic(&self) -> Check {
        let mut check = Check::default();
        let provider = self.provider;
        let first = provider
            .create_checkout_session(EMAIL, PLAN, RETURN_URL)
            .await;
        let second = provider
            .create_checkout_session(EMAIL, PLAN, RETURN_URL)
            .await;
        check.expect(
            first == second,
            "create_checkout_session gives the same result for the same input",
        );
        match self.harness.one_off_request() {
            Some(request) => {
                let first = provider.create_one_off_checkout(&request).await;
                let second = provider.create_one_off_checkout(&request).await;
                check.expect(
                    first == second,
                    "create_one_off_checkout gives the same checkout for the same request",
                );
            }
            None => check.note("no one-off request, so only plan checkout was compared"),
        }
        check
    }

    pub(super) async fn forged_refused(&self) -> Check {
        let mut check = Check::default();
        let mode = self.mode();
        let request = self.harness.one_off_request();
        let reference = request
            .as_ref()
            .map_or(PROBE_REFERENCE, OneOffCheckoutRequest::reference);
        let mut forgeries = vec![
            (
                "an unsigned forged body",
                WebhookDelivery::new(forged_body(reference)),
            ),
            (
                "a forged body with invented signatures",
                invented_signatures(WebhookDelivery::new(forged_body(reference))),
            ),
        ];
        match self.harness.authentic_delivery(request.as_ref()) {
            Some(authentic) => {
                forgeries.push((
                    "the authentic body without its headers",
                    WebhookDelivery::new(authentic.payload.clone()),
                ));
                forgeries.push((
                    "the authentic body with zeroed header values",
                    zeroed(&authentic),
                ));
                if mode == Ok(WebhookVerificationMode::Real) {
                    forgeries.push((
                        "the authentic headers on a tampered body",
                        tampered(&authentic),
                    ));
                }
                if mode == Ok(WebhookVerificationMode::ConfirmedAtSource) {
                    forgeries.push((
                        "an unsigned notification, which only confirmation turns into a payment",
                        authentic,
                    ));
                }
            }
            None => check.note("no authentic delivery, so only invented forgeries were sent"),
        }
        for (label, delivery) in forgeries {
            let result = self
                .provider
                .handle_webhook(&delivery.payload, &delivery.headers);
            check.expect(result.is_err(), format!("handle_webhook refuses {label}"));
        }
        check
    }

    pub(super) async fn replay_idempotent(&self) -> Check {
        let mut check = Check::default();
        let request = self.harness.one_off_request();
        let Some(delivery) = self.harness.authentic_delivery(request.as_ref()) else {
            check.skip("the harness supplies no authentic delivery");
            return check;
        };
        let (payload, headers) = (&delivery.payload, &delivery.headers);
        if self.mode() == Ok(WebhookVerificationMode::ConfirmedAtSource) {
            let first = self.provider.parse_payment_notification(payload, headers);
            let second = self.provider.parse_payment_notification(payload, headers);
            match (&first, &request) {
                (Err(error), _) => check.expect(
                    false,
                    format!("parse_payment_notification reads the authentic notification: {error}"),
                ),
                (Ok(notification), Some(request)) => check.expect(
                    notification.reference() == request.reference(),
                    "the notification names the request's reference",
                ),
                (Ok(_), None) => {}
            }
            check.expect(
                first == second,
                "reading the same notification twice gives the same result",
            );
        } else {
            let read = || {
                self.provider
                    .handle_webhook(payload, headers)
                    .map(|event| format!("{event:?}"))
            };
            let (first, second) = (read(), read());
            if let Err(error) = &first {
                check.expect(
                    false,
                    format!("handle_webhook accepts the authentic delivery: {error}"),
                );
            }
            check.expect(
                first == second,
                "handling the same delivery twice gives the same event; rejecting the replay is the webhook middleware's job",
            );
        }
        check
    }

    pub(super) async fn confirmation_required(&self) -> Check {
        let mut check = Check::default();
        let Some((request, notification)) = self.payment_under_test(&mut check).await else {
            return check;
        };
        let _ = self
            .harness
            .set_source_payment(&request, &SourcePayment::unpaid());
        let unconfirmed = self.confirm(&request, &notification).await;
        check.expect(
            !is_paid(&unconfirmed),
            "confirm_one_off_payment never reports paid while the source has no completed payment",
        );
        self.expect_bound(&mut check, &unconfirmed, &request);
        let paid = SourcePayment::paid(request.total_amount_minor(), request.currency());
        if self.harness.set_source_payment(&request, &paid) {
            let first = self.confirm(&request, &notification).await;
            let second = self.confirm(&request, &notification).await;
            check.expect(
                is_paid(&first),
                format!("a payment the source confirms with the request's amount and currency is paid, not {first:?}"),
            );
            check.expect(
                first == second,
                "confirming the same payment twice gives the same answer",
            );
            self.expect_bound(&mut check, &first, &request);
        } else {
            check.note("the harness does not simulate a confirmed payment, so the paid path was not exercised");
        }
        check
    }

    pub(super) async fn amount_currency_bound(&self) -> Check {
        let mut check = Check::default();
        let Some((request, notification)) = self.payment_under_test(&mut check).await else {
            return check;
        };
        let (total, currency) = (request.total_amount_minor(), request.currency());
        let other_currency = if currency == "usd" { "eur" } else { "usd" };
        for (label, source) in [
            ("a lower amount", SourcePayment::paid(total - 1, currency)),
            ("a higher amount", SourcePayment::paid(total + 1, currency)),
            (
                "another currency",
                SourcePayment::paid(total, other_currency),
            ),
        ] {
            if self.harness.set_source_payment(&request, &source) {
                let result = self.confirm(&request, &notification).await;
                check.expect(
                    !is_paid(&result),
                    format!("a source payment of {label} is never paid"),
                );
            } else {
                check.note(format!("a source payment of {label} was not simulated"));
            }
        }
        let _ = self
            .harness
            .set_source_payment(&request, &SourcePayment::paid(total, currency));
        match renamed(&notification) {
            Ok(other) => {
                let result = self.confirm(&request, &other).await;
                check.expect(
                    !is_paid(&result),
                    "a notification naming another order reference never confirms the request",
                );
            }
            Err(error) => check.expect(false, format!("the notification can be renamed: {error}")),
        }
        check
    }

    pub(super) async fn typed_errors(&self) -> Check {
        let mut check = Check::default();
        let provider = self.provider;
        let headers = HashMap::new();
        let oversized = vec![b'{'; OVERSIZED_BODY];
        for (label, payload) in [
            ("an empty body", &b""[..]),
            ("a non-UTF-8 body", &[0xff, 0xfe, 0x00][..]),
            ("a 3 MiB body", &oversized[..]),
        ] {
            check.expect(
                provider.handle_webhook(payload, &headers).is_err(),
                format!("handle_webhook refuses {label}"),
            );
            check.expect(
                provider
                    .parse_payment_notification(payload, &headers)
                    .is_err(),
                format!("parse_payment_notification refuses {label}"),
            );
        }
        for (holds, expectation) in [
            (
                provider
                    .create_customer_portal("", RETURN_URL)
                    .await
                    .is_err(),
                "create_customer_portal refuses an empty e-mail address",
            ),
            (
                provider.cancel_subscription("").await.is_err(),
                "cancel_subscription refuses an empty subscription ID",
            ),
            (
                provider.pause_subscription("").await.is_err(),
                "pause_subscription refuses an empty subscription ID",
            ),
            (
                provider.report_usage("", "seats", 1).await.is_err(),
                "report_usage refuses an empty subscription ID",
            ),
            (
                provider
                    .report_usage("sub_conformance", "seats", 0)
                    .await
                    .is_err(),
                "report_usage refuses a zero quantity",
            ),
            (
                provider.apply_coupon("", "").await.is_err(),
                "apply_coupon refuses an empty subscription ID and coupon",
            ),
            (
                provider.extend_trial("", 0).await.is_err(),
                "extend_trial refuses an empty subscription ID",
            ),
        ] {
            check.expect(holds, expectation);
        }
        check
    }

    /// The harness request and the notification that names its payment, or
    /// `None` after recording why the case cannot run.
    async fn payment_under_test(
        &self,
        check: &mut Check,
    ) -> Option<(OneOffCheckoutRequest, PaymentNotification)> {
        let Some(request) = self.harness.one_off_request() else {
            check.skip("the provider has no one-off checkout");
            return None;
        };
        match self.notification(&request).await {
            Ok(notification) => Some((request, notification)),
            Err(error) => {
                check.expect(false, error);
                None
            }
        }
    }

    /// What names the payment: the parsed authentic notification of a provider
    /// that confirms at the source, otherwise the reference and the checkout's
    /// session ID.
    async fn notification(
        &self,
        request: &OneOffCheckoutRequest,
    ) -> Result<PaymentNotification, String> {
        if self.mode() == Ok(WebhookVerificationMode::ConfirmedAtSource)
            && let Some(delivery) = self.harness.authentic_delivery(Some(request))
        {
            return self
                .provider
                .parse_payment_notification(&delivery.payload, &delivery.headers)
                .map_err(|error| {
                    format!("parse_payment_notification reads the authentic notification: {error}")
                });
        }
        let notification = PaymentNotification::new(request.reference())
            .map_err(|error| format!("the request's reference names a notification: {error}"))?;
        let checkout = self.provider.create_one_off_checkout(request).await;
        match checkout
            .ok()
            .and_then(|checkout| checkout.session_id().map(str::to_owned))
        {
            Some(session) => notification
                .with_transaction_id(session)
                .map_err(|error| format!("the checkout session ID names a notification: {error}")),
            None => Ok(notification),
        }
    }

    async fn confirm(
        &self,
        request: &OneOffCheckoutRequest,
        notification: &PaymentNotification,
    ) -> Result<OneOffPayment, CapitalError> {
        self.provider
            .confirm_one_off_payment(request, notification)
            .await
    }

    fn expect_bound(
        &self,
        check: &mut Check,
        result: &Result<OneOffPayment, CapitalError>,
        request: &OneOffCheckoutRequest,
    ) {
        if let Ok(payment) = result {
            check.expect(
                payment.provider() == self.provider.name(),
                "the payment names the provider",
            );
            check.expect(
                payment.request_digest() == request.request_digest(),
                "the payment is bound to the server-side request",
            );
        }
    }
}

fn is_paid(result: &Result<OneOffPayment, CapitalError>) -> bool {
    result.as_ref().is_ok_and(OneOffPayment::is_paid)
}

fn probe_request() -> Result<OneOffCheckoutRequest, CapitalError> {
    OneOffCheckoutRequest::new(
        PROBE_REFERENCE,
        1_500,
        "usd",
        "Conformance probe",
        RETURN_URL,
        CANCEL_URL,
    )
}

/// A body that claims `reference` was paid in the shapes common gateways use.
fn forged_body(reference: &str) -> Vec<u8> {
    format!(
        r#"{{"type":"checkout.session.completed","status":"paid","paid":true,"order_nsu":"{reference}","transaction_nsu":"forged-transaction","invoice_slug":"forged-invoice","amount":1,"paid_amount":1,"data":{{"object":{{"object":"subscription","id":"sub_forged","customer":"cus_forged","status":"active","client_reference_id":"{reference}","payment_status":"paid"}}}}}}"#
    )
    .into_bytes()
}

fn invented_signatures(delivery: WebhookDelivery) -> WebhookDelivery {
    let digest = "0".repeat(64);
    delivery
        .with_header("stripe-signature", format!("t=1700000000,v1={digest}"))
        .with_header("x-signature", "forged")
        .with_header("x-infinitepay-signature", "forged")
        .with_header("x-hub-signature-256", format!("sha256={digest}"))
        .with_header("webhook-id", "msg_forged")
        .with_header("webhook-timestamp", "1700000000")
        .with_header("webhook-signature", "v1,Zm9yZ2Vk")
}

fn zeroed(authentic: &WebhookDelivery) -> WebhookDelivery {
    let mut delivery = WebhookDelivery::new(authentic.payload.clone());
    for (name, value) in &authentic.headers {
        delivery = delivery.with_header(name.as_str(), "0".repeat(value.len().max(1)));
    }
    delivery
}

fn tampered(authentic: &WebhookDelivery) -> WebhookDelivery {
    let mut delivery = authentic.clone();
    delivery.payload.push(b' ');
    delivery
}

fn renamed(notification: &PaymentNotification) -> Result<PaymentNotification, CapitalError> {
    let mut other = PaymentNotification::new(OTHER_REFERENCE)?;
    if let Some(id) = notification.transaction_id() {
        other = other.with_transaction_id(id)?;
    }
    if let Some(id) = notification.invoice_id() {
        other = other.with_invoice_id(id)?;
    }
    Ok(other)
}
