//! InfinitePay Checkout API calls: `POST /links`, `POST /payment_check`, and
//! the unsigned webhook body.

use super::InfinitePayProvider;
use crate::providers::ProviderRequest;
use crate::{
    CapitalError, OneOffCheckout, OneOffCheckoutRequest, OneOffPayment, PaymentNotification,
};
use serde::Deserialize;
use serde_json::{Value, json};

const PROVIDER: &str = "infinitepay";
const CREATE: &str = "create payment link";
const CHECK: &str = "payment check";
/// InfinitePay's minimum payment, R$ 1.00.
const MIN_PRICE_CENTS: u64 = 100;
/// A notification body is a small JSON object; larger ones are refused.
const MAX_NOTIFICATION_BYTES: usize = 64 * 1024;

/// InfinitePay charges in BRL with a minimum of R$ 1.00 per unit.
pub(super) fn validate_request(request: &OneOffCheckoutRequest) -> Result<(), CapitalError> {
    if request.currency() != "brl" {
        return Err(CapitalError::InvalidCharge(
            "InfinitePay charges only BRL".into(),
        ));
    }
    if request.unit_amount_minor() < MIN_PRICE_CENTS {
        return Err(CapitalError::InvalidCharge(
            "InfinitePay requires at least R$ 1.00 (100 cents) per unit".into(),
        ));
    }
    Ok(())
}

pub(super) fn link_body(
    provider: &InfinitePayProvider,
    handle: &str,
    request: &OneOffCheckoutRequest,
) -> Value {
    let mut body = json!({
        "handle": handle,
        "items": [{
            "quantity": request.quantity(),
            "price": request.unit_amount_minor(),
            "description": request.description(),
        }],
        "order_nsu": request.reference(),
        "redirect_url": request.success_url(),
    });
    if let Some(url) = &provider.webhook_url {
        body["webhook_url"] = json!(url);
    }
    if let Some(email) = request.customer_email() {
        body["customer"] = json!({ "email": email });
    }
    if !provider.api_key.is_empty() {
        // Only an API key unlocks per-link options.
        body["options"] = json!({ "single_payment": true });
    }
    body
}

pub(super) async fn create_link(
    provider: &InfinitePayProvider,
    handle: &str,
    request: &OneOffCheckoutRequest,
) -> Result<OneOffCheckout, CapitalError> {
    let webhook = provider.webhook_url.as_deref();
    crate::providers::require_https_redirects(
        &[request.success_url()]
            .into_iter()
            .chain(webhook)
            .collect::<Vec<_>>(),
    )?;
    let response: Value = authorized(
        provider,
        ProviderRequest::post_json(
            PROVIDER,
            CREATE,
            format!("{}/links", provider.api_base),
            &link_body(provider, handle, request),
        )?,
    )
    .send_json()
    .await?;
    let url = checkout_url(&response)
        .ok_or_else(|| crate::ProviderFailure::contract_mismatch(PROVIDER, CREATE))?;
    OneOffCheckout::from_verified_provider_response(PROVIDER, request, url)
}

/// The documentation does not name the field holding the link; accounts
/// have returned `url` and `checkout_url`. Only an HTTPS InfinitePay host is
/// accepted.
pub(super) fn checkout_url(response: &Value) -> Option<&str> {
    ["url", "checkout_url", "link"]
        .iter()
        .find_map(|field| response.get(*field).and_then(Value::as_str))
        .filter(|url| {
            reqwest::Url::parse(url).is_ok_and(|parsed| {
                parsed.scheme() == "https"
                    && parsed.host_str().is_some_and(|host| {
                        host == "infinitepay.io" || host.ends_with(".infinitepay.io")
                    })
            })
        })
}

#[derive(Deserialize)]
pub(super) struct PaymentCheck {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    paid: bool,
    #[serde(default)]
    amount: u64,
    #[serde(default)]
    paid_amount: u64,
}

pub(super) async fn payment_check(
    provider: &InfinitePayProvider,
    handle: &str,
    request: &OneOffCheckoutRequest,
    notification: &PaymentNotification,
) -> Result<OneOffPayment, CapitalError> {
    let (Some(transaction_nsu), Some(slug)) =
        (notification.transaction_id(), notification.invoice_id())
    else {
        return Err(CapitalError::PayloadParseError(
            "InfinitePay payment_check needs the transaction_nsu and the invoice slug".into(),
        ));
    };
    let check: PaymentCheck = authorized(
        provider,
        ProviderRequest::post_json(
            PROVIDER,
            CHECK,
            format!("{}/payment_check", provider.api_base),
            &json!({
                "handle": handle,
                "order_nsu": request.reference(),
                "transaction_nsu": transaction_nsu,
                "slug": slug,
            }),
        )?,
    )
    .send_json()
    .await?;
    confirmation(request, transaction_nsu, &check)
}

/// `paid: true` only when InfinitePay matched every identifier; the amount is
/// still compared, because anyone can create a link for a public handle.
pub(super) fn confirmation(
    request: &OneOffCheckoutRequest,
    transaction_nsu: &str,
    check: &PaymentCheck,
) -> Result<OneOffPayment, CapitalError> {
    let payment = if check.success && check.paid {
        OneOffPayment::paid(PROVIDER, request, check.amount, check.paid_amount, "brl")?
    } else {
        OneOffPayment::unpaid(PROVIDER, request)
    };
    payment.with_transaction_id(transaction_nsu)
}

/// Reads only the identifiers of an unsigned webhook body; its amounts and
/// status are ignored.
pub(super) fn parse_notification(payload: &[u8]) -> Result<PaymentNotification, CapitalError> {
    if payload.len() > MAX_NOTIFICATION_BYTES {
        return Err(CapitalError::PayloadParseError(
            "InfinitePay notification exceeds 64 KiB".into(),
        ));
    }
    let body: Value = serde_json::from_slice(payload).map_err(|_| {
        CapitalError::PayloadParseError("InfinitePay notification is not a JSON object".into())
    })?;
    let field = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| body.get(*name).and_then(Value::as_str))
    };
    let reference = field(&["order_nsu"]).ok_or_else(|| {
        CapitalError::PayloadParseError("InfinitePay notification has no order_nsu".into())
    })?;
    let mut notification = PaymentNotification::new(reference)?;
    if let Some(transaction) = field(&["transaction_nsu"]) {
        notification = notification.with_transaction_id(transaction)?;
    }
    if let Some(slug) = field(&["invoice_slug", "slug"]) {
        notification = notification.with_invoice_id(slug)?;
    }
    Ok(notification)
}

fn authorized(provider: &InfinitePayProvider, request: ProviderRequest) -> ProviderRequest {
    if provider.api_key.is_empty() {
        request
    } else {
        request.header("X-Api-Key", provider.api_key.as_str())
    }
}
