use super::{execute_http, http_client, read_http_json, url_encode, validate_checkout_url};
use crate::{CapitalError, StripeCheckoutRequest, StripeCheckoutSession, StripeCheckoutStatus};
use serde_json::Value;

const OPERATION: &str = "customer-bound subscription checkout";

impl super::StripeProvider {
    /// Creates hosted subscription checkout for an already bound customer.
    ///
    /// Persist the authorized customer/tenant binding and immutable attempt
    /// before calling; a timeout can mean Stripe created the session. Reuse the
    /// original key within Stripe's retention window and reconcile older or
    /// unknown outcomes. Session creation never proves payment or grants access.
    pub async fn create_subscription_checkout(
        &self,
        request: &StripeCheckoutRequest,
    ) -> Result<StripeCheckoutSession, CapitalError> {
        let api_key = self.usage_api_key();
        if api_key.is_empty() || api_key.starts_with("mock_") {
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
        let response = execute_http(
            build_request(http_client()?, api_key, request)?,
            "stripe",
            OPERATION,
        )
        .await?;
        let response: Value = read_http_json(response, "stripe", OPERATION).await?;
        let result = parse_response(request, &response)?;
        let credential_mode = super::stripe_contract::credential_mode(api_key);
        if credential_mode.is_some() && credential_mode != result.livemode() {
            return Err(mismatch());
        }
        Ok(result)
    }
}

fn build_request(
    client: &reqwest::Client,
    api_key: &str,
    request: &StripeCheckoutRequest,
) -> Result<reqwest::Request, CapitalError> {
    let body = format!(
        "mode=subscription&customer={}&client_reference_id={}&subscription_data[metadata][rullst_owner_reference]={}&line_items[0][price]={}&line_items[0][quantity]=1&success_url={}&cancel_url={}&expand[0]=line_items",
        url_encode(request.customer_id()),
        url_encode(request.owner_reference()),
        url_encode(request.owner_reference()),
        url_encode(request.price_id()),
        url_encode(request.success_url()),
        url_encode(request.cancel_url()),
    );
    client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .bearer_auth(api_key)
        .header("Stripe-Version", super::stripe_contract::API_VERSION)
        .header("Idempotency-Key", request.idempotency_key())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .build()
        .map_err(|_| crate::ProviderFailure::request_build("stripe", OPERATION).into())
}

fn mismatch() -> CapitalError {
    crate::ProviderFailure::contract_mismatch("stripe", OPERATION).into()
}

fn parse_response(
    request: &StripeCheckoutRequest,
    response: &Value,
) -> Result<StripeCheckoutSession, CapitalError> {
    let id = response["id"]
        .as_str()
        .filter(|id| super::stripe_contract::valid_reference(id, "cs_", 255))
        .ok_or_else(mismatch)?;
    let livemode = response["livemode"].as_bool().ok_or_else(mismatch)?;
    let expires_at = response["expires_at"]
        .as_i64()
        .filter(|time| *time > 0)
        .ok_or_else(mismatch)?;
    if response["object"].as_str() != Some("checkout.session")
        || response["mode"].as_str() != Some("subscription")
        || response["status"].as_str() != Some("open")
        || response["customer"].as_str() != Some(request.customer_id())
        || response["client_reference_id"].as_str() != Some(request.owner_reference())
        || response["success_url"].as_str() != Some(request.success_url())
        || response["cancel_url"].as_str() != Some(request.cancel_url())
        || response["line_items"]["has_more"].as_bool() != Some(false)
    {
        return Err(mismatch());
    }
    let items = response["line_items"]["data"]
        .as_array()
        .ok_or_else(mismatch)?;
    if items.len() != 1
        || items[0]["quantity"].as_u64() != Some(1)
        || items[0]["price"]["id"].as_str() != Some(request.price_id())
        || items[0]["price"]["type"].as_str() != Some("recurring")
    {
        return Err(mismatch());
    }
    let url = validate_checkout_url("stripe", response["url"].as_str().ok_or_else(mismatch)?)?;
    Ok(StripeCheckoutSession {
        id: id.to_owned(),
        url,
        status: StripeCheckoutStatus::Created,
        livemode: Some(livemode),
        expires_at: Some(expires_at),
        request_digest: request.request_digest(),
    })
}

#[cfg(test)]
#[path = "stripe_checkout_tests.rs"]
mod tests;
