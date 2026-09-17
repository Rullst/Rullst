use super::{execute_http, http_client, read_http_json, stripe_contract, url_encode};
use crate::{CapitalError, StripeCustomerReceipt, StripeCustomerRequest, StripeCustomerStatus};
use serde_json::Value;

const OPERATION: &str = "create bound customer";

impl super::StripeProvider {
    /// Creates a provider customer with explicit local reference and retry identity.
    ///
    /// Persist provisioning intent before dispatch and the returned ID before
    /// checkout. A timeout can hide successful creation: reuse the original
    /// request within provider retention and reconcile older unknown outcomes.
    /// This method neither searches by email nor persists application ownership.
    pub async fn create_customer(
        &self,
        request: &StripeCustomerRequest,
    ) -> Result<StripeCustomerReceipt, CapitalError> {
        let api_key = self.usage_api_key();
        if api_key.is_empty() || api_key.starts_with("mock_") {
            let digest = request.request_digest();
            return Ok(StripeCustomerReceipt {
                id: format!("cus_mock_{}", hex::encode(digest)),
                status: StripeCustomerStatus::Mock,
                livemode: None,
                created_at: None,
                request_digest: digest,
            });
        }
        let response = execute_http(
            build_request(http_client()?, api_key, request)?,
            "stripe",
            OPERATION,
        )
        .await?;
        let body: Value = read_http_json(response, "stripe", OPERATION).await?;
        parse_response(request, &body, stripe_contract::credential_mode(api_key))
    }
}

fn build_request(
    client: &reqwest::Client,
    api_key: &str,
    request: &StripeCustomerRequest,
) -> Result<reqwest::Request, CapitalError> {
    let mut body = format!(
        "metadata[rullst_owner_reference]={}",
        url_encode(request.owner_reference())
    );
    if let Some(email) = request.email() {
        body.push_str("&email=");
        body.push_str(&url_encode(email));
    }
    client
        .post("https://api.stripe.com/v1/customers")
        .bearer_auth(api_key)
        .header("Stripe-Version", stripe_contract::API_VERSION)
        .header("Idempotency-Key", request.idempotency_key())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .build()
        .map_err(|_| crate::ProviderFailure::request_build("stripe", OPERATION).into())
}

fn parse_response(
    request: &StripeCustomerRequest,
    body: &Value,
    expected_mode: Option<bool>,
) -> Result<StripeCustomerReceipt, CapitalError> {
    let id = body["id"]
        .as_str()
        .filter(|id| stripe_contract::valid_reference(id, "cus_", 200))
        .ok_or_else(mismatch)?;
    let mode = body["livemode"].as_bool().ok_or_else(mismatch)?;
    let created_at = body["created"]
        .as_i64()
        .filter(|time| *time > 0)
        .ok_or_else(mismatch)?;
    if body["object"].as_str() != Some("customer")
        || body["metadata"]["rullst_owner_reference"].as_str() != Some(request.owner_reference())
        || (!body["deleted"].is_null() && body["deleted"].as_bool() != Some(false))
        || expected_mode.is_some_and(|expected| expected != mode)
    {
        return Err(mismatch());
    }
    Ok(StripeCustomerReceipt {
        id: id.to_owned(),
        status: StripeCustomerStatus::Created,
        livemode: Some(mode),
        created_at: Some(created_at),
        request_digest: request.request_digest(),
    })
}

fn mismatch() -> CapitalError {
    crate::ProviderFailure::contract_mismatch("stripe", OPERATION).into()
}

#[cfg(test)]
#[path = "stripe_customer_tests.rs"]
mod tests;
