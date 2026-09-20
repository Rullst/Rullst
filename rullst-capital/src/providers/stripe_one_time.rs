use super::{execute_http, http_client, read_http_json, url_encode, validate_checkout_url};
use crate::{
    CapitalError, StripeCheckoutSession, StripeCheckoutStatus, StripeOneTimePaymentState,
    StripeOneTimeReceipt, StripePaymentCheckoutRequest,
};
use serde_json::Value;

const OPERATION: &str = "customer-bound one-time checkout";

impl super::StripeProvider {
    /// Validates the current active fixed price against the server's displayed
    /// amount/currency, then creates a card-only `mode=payment` hosted checkout.
    /// No access is granted by session creation or a success-page redirect.
    pub async fn create_one_time_checkout(
        &self,
        request: &StripePaymentCheckoutRequest,
    ) -> Result<StripeCheckoutSession, CapitalError> {
        let key = self.usage_api_key();
        if is_mock(key) {
            let digest = request.request_digest();
            let id = format!("cs_mock_{}", hex::encode(digest));
            return Ok(StripeCheckoutSession {
                url: format!("https://mock.stripe.invalid/checkout/{id}"),
                id,
                status: StripeCheckoutStatus::Mock,
                livemode: None,
                expires_at: None,
                request_digest: digest,
            });
        }
        let price = self
            .read_one_time_object(&format!("prices/{}", request.price().id()))
            .await?;
        validate_price(request, &price, true)?;
        verify_mode(key, &price)?;
        let response = execute_http(
            build_request(http_client()?, key, request)?,
            "stripe",
            OPERATION,
        )
        .await?;
        let response = read_http_json(response, "stripe", OPERATION).await?;
        let (id, live) = validate_session(request, &response)?;
        verify_mode(key, &response)?;
        if response["status"] != "open" || response["payment_status"] != "unpaid" {
            return Err(mismatch());
        }
        Ok(StripeCheckoutSession {
            id: id.into(),
            url: validate_checkout_url("stripe", response["url"].as_str().ok_or_else(mismatch)?)?,
            status: StripeCheckoutStatus::Created,
            livemode: Some(live),
            expires_at: Some(
                response["expires_at"]
                    .as_i64()
                    .filter(|time| *time > 0)
                    .ok_or_else(mismatch)?,
            ),
            request_digest: request.request_digest(),
        })
    }

    /// Re-reads the session, line item, PaymentIntent and charge before an
    /// entitlement transition. Any refund or dispute prevents a Paid snapshot.
    /// Persist the event claim and entitlement mutation atomically in the app;
    /// reconcile again after later refund/dispute notifications.
    pub async fn read_one_time_receipt(
        &self,
        session_id: impl Into<String>,
        request: &StripePaymentCheckoutRequest,
    ) -> Result<StripeOneTimeReceipt, CapitalError> {
        let session_id = session_id.into();
        if !super::stripe_contract::valid_reference(&session_id, "cs_", 255) {
            return Err(mismatch());
        }
        if is_mock(self.usage_api_key()) {
            return Ok(StripeOneTimeReceipt {
                session_id,
                payment_intent_id: None,
                state: StripeOneTimePaymentState::Mock,
                livemode: None,
                request_digest: request.request_digest(),
            });
        }
        let response = self.read_one_time_object(&format!("checkout/sessions/{session_id}?expand[]=line_items&expand[]=payment_intent.latest_charge")).await?;
        verify_mode(self.usage_api_key(), &response)?;
        parse_receipt(&session_id, request, &response)
    }

    async fn read_one_time_object(&self, path: &str) -> Result<Value, CapitalError> {
        let request = http_client()?
            .get(format!("https://api.stripe.com/v1/{path}"))
            .bearer_auth(self.usage_api_key())
            .header("Stripe-Version", super::stripe_contract::API_VERSION)
            .build()
            .map_err(|_| crate::ProviderFailure::request_build("stripe", OPERATION))?;
        let response = execute_http(request, "stripe", OPERATION).await?;
        read_http_json(response, "stripe", OPERATION).await
    }
}

fn is_mock(key: &str) -> bool {
    key.is_empty() || key.starts_with("mock_")
}
fn mismatch() -> CapitalError {
    crate::ProviderFailure::contract_mismatch("stripe", OPERATION).into()
}

fn verify_mode(key: &str, object: &Value) -> Result<(), CapitalError> {
    let mode = super::stripe_contract::credential_mode(key).ok_or_else(mismatch)?;
    if object["livemode"].as_bool() != Some(mode) {
        return Err(mismatch());
    }
    Ok(())
}

fn build_request(
    client: &reqwest::Client,
    key: &str,
    request: &StripePaymentCheckoutRequest,
) -> Result<reqwest::Request, CapitalError> {
    let body = format!(
        "mode=payment&payment_method_types[0]=card&automatic_tax[enabled]=false&allow_promotion_codes=false&adaptive_pricing[enabled]=false&customer={}&client_reference_id={}&line_items[0][price]={}&line_items[0][quantity]=1&success_url={}&cancel_url={}&expand[0]=line_items&metadata[rullst_owner_reference]={}&metadata[rullst_attempt_reference]={}&payment_intent_data[metadata][rullst_owner_reference]={}&payment_intent_data[metadata][rullst_attempt_reference]={}",
        url_encode(request.customer_id()),
        url_encode(request.owner_reference()),
        url_encode(request.price().id()),
        url_encode(request.success_url()),
        url_encode(request.cancel_url()),
        url_encode(request.owner_reference()),
        url_encode(request.idempotency_key()),
        url_encode(request.owner_reference()),
        url_encode(request.idempotency_key()),
    );
    client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .bearer_auth(key)
        .header("Stripe-Version", super::stripe_contract::API_VERSION)
        .header("Idempotency-Key", request.idempotency_key())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .build()
        .map_err(|_| crate::ProviderFailure::request_build("stripe", OPERATION).into())
}

fn validate_price(
    request: &StripePaymentCheckoutRequest,
    price: &Value,
    active: bool,
) -> Result<(), CapitalError> {
    if price["object"] != "price"
        || price["id"].as_str() != Some(request.price().id())
        || price["type"] != "one_time"
        || !price["recurring"].is_null()
        || price["unit_amount"].as_u64() != Some(request.price().amount_minor())
        || price["currency"].as_str() != Some(request.price().currency())
        || (active && price["active"].as_bool() != Some(true))
    {
        return Err(mismatch());
    }
    Ok(())
}

fn validate_session<'a>(
    request: &StripePaymentCheckoutRequest,
    response: &'a Value,
) -> Result<(&'a str, bool), CapitalError> {
    let id = response["id"]
        .as_str()
        .filter(|id| super::stripe_contract::valid_reference(id, "cs_", 255))
        .ok_or_else(mismatch)?;
    let live = response["livemode"].as_bool().ok_or_else(mismatch)?;
    if response["object"] != "checkout.session"
        || response["mode"] != "payment"
        || !response["subscription"].is_null()
        || response["customer"].as_str() != Some(request.customer_id())
        || response["client_reference_id"].as_str() != Some(request.owner_reference())
        || response["metadata"]["rullst_owner_reference"].as_str()
            != Some(request.owner_reference())
        || response["metadata"]["rullst_attempt_reference"].as_str()
            != Some(request.idempotency_key())
        || response["amount_total"].as_u64() != Some(request.price().amount_minor())
        || response["currency"].as_str() != Some(request.price().currency())
        || response["success_url"].as_str() != Some(request.success_url())
        || response["cancel_url"].as_str() != Some(request.cancel_url())
        || response["line_items"]["has_more"].as_bool() != Some(false)
    {
        return Err(mismatch());
    }
    let items = response["line_items"]["data"]
        .as_array()
        .ok_or_else(mismatch)?;
    if items.len() != 1 || items[0]["quantity"].as_u64() != Some(1) {
        return Err(mismatch());
    }
    validate_price(request, &items[0]["price"], false)?;
    Ok((id, live))
}

fn parse_receipt(
    session_id: &str,
    request: &StripePaymentCheckoutRequest,
    response: &Value,
) -> Result<StripeOneTimeReceipt, CapitalError> {
    let (id, live) = validate_session(request, response)?;
    if id != session_id {
        return Err(mismatch());
    }
    let mut receipt = StripeOneTimeReceipt {
        session_id: id.into(),
        payment_intent_id: None,
        state: StripeOneTimePaymentState::Unpaid,
        livemode: Some(live),
        request_digest: request.request_digest(),
    };
    match (
        response["status"].as_str(),
        response["payment_status"].as_str(),
    ) {
        (Some("expired"), Some("unpaid")) => {
            receipt.state = StripeOneTimePaymentState::Expired;
            return Ok(receipt);
        }
        (Some("open" | "complete"), Some("unpaid")) => return Ok(receipt),
        (Some("complete"), Some("paid")) => {}
        _ => return Err(mismatch()),
    }
    let intent = &response["payment_intent"];
    let intent_id = intent["id"]
        .as_str()
        .filter(|id| super::stripe_contract::valid_reference(id, "pi_", 255))
        .ok_or_else(mismatch)?;
    if intent["object"] != "payment_intent"
        || intent["status"] != "succeeded"
        || intent["livemode"].as_bool() != Some(live)
        || intent["customer"].as_str() != Some(request.customer_id())
        || intent["amount_received"].as_u64() != Some(request.price().amount_minor())
        || intent["currency"].as_str() != Some(request.price().currency())
        || intent["metadata"]["rullst_owner_reference"].as_str() != Some(request.owner_reference())
        || intent["metadata"]["rullst_attempt_reference"].as_str()
            != Some(request.idempotency_key())
    {
        return Err(mismatch());
    }
    let charge = &intent["latest_charge"];
    if charge["id"]
        .as_str()
        .is_none_or(|id| !super::stripe_contract::valid_reference(id, "ch_", 255))
        || charge["object"] != "charge"
        || charge["payment_intent"].as_str() != Some(intent_id)
        || charge["customer"].as_str() != Some(request.customer_id())
        || charge["status"] != "succeeded"
        || charge["paid"].as_bool() != Some(true)
        || charge["captured"].as_bool() != Some(true)
        || charge["livemode"].as_bool() != Some(live)
        || charge["amount"].as_u64() != Some(request.price().amount_minor())
        || charge["amount_captured"].as_u64() != Some(request.price().amount_minor())
        || charge["currency"].as_str() != Some(request.price().currency())
    {
        return Err(mismatch());
    }
    let refunded = charge["amount_refunded"]
        .as_u64()
        .filter(|value| *value <= request.price().amount_minor())
        .ok_or_else(mismatch)?;
    receipt.state = if charge["disputed"].as_bool().ok_or_else(mismatch)? {
        StripeOneTimePaymentState::Disputed
    } else if refunded > 0 {
        StripeOneTimePaymentState::Refunded
    } else {
        StripeOneTimePaymentState::Paid
    };
    receipt.payment_intent_id = Some(intent_id.into());
    Ok(receipt)
}

#[cfg(test)]
#[path = "stripe_one_time_tests.rs"]
mod tests;
