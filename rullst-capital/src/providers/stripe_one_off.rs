//! Provider-neutral one-off checkout on Stripe: inline `price_data`, no
//! pre-created price or customer, confirmed by reading the session back.

use super::{execute_http, http_client, read_http_json, url_encode};
use crate::{
    CapitalError, OneOffCheckout, OneOffCheckoutRequest, OneOffPayment, PaymentNotification,
};
use serde_json::Value;

const OPERATION: &str = "one-off checkout";
const CONFIRM: &str = "one-off payment confirmation";
const API_BASE: &str = "https://api.stripe.com";

pub(super) async fn create(
    api_key: &str,
    request: &OneOffCheckoutRequest,
) -> Result<OneOffCheckout, CapitalError> {
    create_at(API_BASE, api_key, request).await
}

pub(super) async fn confirm(
    api_key: &str,
    request: &OneOffCheckoutRequest,
    notification: &PaymentNotification,
) -> Result<OneOffPayment, CapitalError> {
    confirm_at(API_BASE, api_key, request, notification).await
}

async fn create_at(
    base: &str,
    api_key: &str,
    request: &OneOffCheckoutRequest,
) -> Result<OneOffCheckout, CapitalError> {
    if is_mock(api_key) {
        return Ok(OneOffCheckout::mock("stripe", request));
    }
    super::require_https_redirects(&[request.success_url(), request.cancel_url()])?;
    let http = http_client()?
        .post(format!("{base}/v1/checkout/sessions"))
        .bearer_auth(api_key)
        .header("Stripe-Version", super::stripe_contract::API_VERSION)
        .header("Idempotency-Key", request.idempotency_key())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form(request))
        .build()
        .map_err(|_| crate::ProviderFailure::request_build("stripe", OPERATION))?;
    let response = execute_http(http, "stripe", OPERATION).await?;
    let response: Value = read_http_json(response, "stripe", OPERATION).await?;
    let id = validate_session(api_key, request, &response, OPERATION)?;
    if response["status"] != "open" || response["payment_status"] != "unpaid" {
        return Err(mismatch(OPERATION));
    }
    let url = response["url"]
        .as_str()
        .ok_or_else(|| mismatch(OPERATION))?;
    OneOffCheckout::from_verified_provider_response("stripe", request, url)?.with_session_id(id)
}

async fn confirm_at(
    base: &str,
    api_key: &str,
    request: &OneOffCheckoutRequest,
    notification: &PaymentNotification,
) -> Result<OneOffPayment, CapitalError> {
    notification.require_reference_of(request)?;
    if is_mock(api_key) {
        return Ok(OneOffPayment::mock("stripe", request));
    }
    let session_id = notification
        .transaction_id()
        .filter(|id| super::stripe_contract::valid_reference(id, "cs_", 255))
        .ok_or_else(|| {
            CapitalError::PayloadParseError(
                "Stripe confirmation needs the Checkout Session ID (cs_…) as the transaction ID"
                    .into(),
            )
        })?;
    let http = http_client()?
        .get(format!("{base}/v1/checkout/sessions/{session_id}"))
        .bearer_auth(api_key)
        .header("Stripe-Version", super::stripe_contract::API_VERSION)
        .build()
        .map_err(|_| crate::ProviderFailure::request_build("stripe", CONFIRM))?;
    let response = execute_http(http, "stripe", CONFIRM).await?;
    let response: Value = read_http_json(response, "stripe", CONFIRM).await?;
    parse_confirmation(api_key, session_id, request, &response)
}

fn form(request: &OneOffCheckoutRequest) -> String {
    let mut body = format!(
        "mode=payment&payment_method_types[0]=card&automatic_tax[enabled]=false&allow_promotion_codes=false&adaptive_pricing[enabled]=false&client_reference_id={reference}&line_items[0][quantity]={quantity}&line_items[0][price_data][currency]={currency}&line_items[0][price_data][unit_amount]={unit}&line_items[0][price_data][product_data][name]={name}&success_url={success}&cancel_url={cancel}&metadata[rullst_reference]={reference}&payment_intent_data[metadata][rullst_reference]={reference}",
        reference = url_encode(request.reference()),
        quantity = request.quantity(),
        currency = url_encode(request.currency()),
        unit = request.unit_amount_minor(),
        name = url_encode(request.description()),
        success = url_encode(request.success_url()),
        cancel = url_encode(request.cancel_url()),
    );
    if let Some(email) = request.customer_email() {
        body.push_str("&customer_email=");
        body.push_str(&url_encode(email));
    }
    body
}

/// Checks the session's identity, mode, amount and currency; returns its ID.
fn validate_session<'a>(
    api_key: &str,
    request: &OneOffCheckoutRequest,
    response: &'a Value,
    operation: &'static str,
) -> Result<&'a str, CapitalError> {
    let id = response["id"]
        .as_str()
        .filter(|id| super::stripe_contract::valid_reference(id, "cs_", 255))
        .ok_or_else(|| mismatch(operation))?;
    let mode = super::stripe_contract::credential_mode(api_key);
    if response["object"] != "checkout.session"
        || response["mode"] != "payment"
        || response["client_reference_id"].as_str() != Some(request.reference())
        || response["metadata"]["rullst_reference"].as_str() != Some(request.reference())
        || response["amount_total"].as_u64() != Some(request.total_amount_minor())
        || response["currency"].as_str() != Some(request.currency())
        || (mode.is_some() && response["livemode"].as_bool() != mode)
    {
        return Err(mismatch(operation));
    }
    Ok(id)
}

fn parse_confirmation(
    api_key: &str,
    session_id: &str,
    request: &OneOffCheckoutRequest,
    response: &Value,
) -> Result<OneOffPayment, CapitalError> {
    if validate_session(api_key, request, response, CONFIRM)? != session_id {
        return Err(mismatch(CONFIRM));
    }
    match (
        response["status"].as_str(),
        response["payment_status"].as_str(),
    ) {
        (Some("complete"), Some("paid")) => {
            let amount = response["amount_total"]
                .as_u64()
                .ok_or_else(|| mismatch(CONFIRM))?;
            let currency = response["currency"]
                .as_str()
                .ok_or_else(|| mismatch(CONFIRM))?;
            OneOffPayment::paid("stripe", request, amount, amount, currency)?
                .with_transaction_id(session_id)
        }
        (Some("open" | "complete" | "expired"), Some("unpaid")) => {
            OneOffPayment::unpaid("stripe", request).with_transaction_id(session_id)
        }
        _ => Err(mismatch(CONFIRM)),
    }
}

fn is_mock(api_key: &str) -> bool {
    api_key.is_empty() || api_key.starts_with("mock_")
}

fn mismatch(operation: &'static str) -> CapitalError {
    crate::ProviderFailure::contract_mismatch("stripe", operation).into()
}

#[cfg(test)]
#[path = "stripe_one_off_tests.rs"]
mod tests;
