use super::{execute_http, http_client, read_http_json, stripe_contract, stripe_webhook};
use crate::{
    CapitalError, StripeSubscriptionLookup, StripeSubscriptionSnapshot, StripeSubscriptionSource,
    SubscriptionStatus, WebhookEvent,
};
use serde_json::Value;

const OPERATION: &str = "retrieve bound subscription";

impl super::StripeProvider {
    /// Reconciles a subscription whose price may have changed through the
    /// provider portal. Every other ownership binding remains mandatory; the
    /// current single recurring price must belong to the server-owned catalog.
    pub async fn retrieve_subscription_with_allowed_prices(
        &self,
        request: &StripeSubscriptionLookup,
        allowed_prices: &[&str],
    ) -> Result<StripeSubscriptionSnapshot, CapitalError> {
        if allowed_prices.is_empty()
            || allowed_prices.len() > 64
            || allowed_prices
                .iter()
                .any(|price| !stripe_contract::valid_reference(price, "price_", 200))
        {
            return Err(mismatch());
        }
        let api_key = self.usage_api_key();
        if api_key.is_empty() || api_key.starts_with("mock_") {
            return self.retrieve_subscription(request).await;
        }
        let response = execute_http(
            build_request(http_client()?, api_key, request)?,
            "stripe",
            OPERATION,
        )
        .await?;
        let body: Value = read_http_json(response, "stripe", OPERATION).await?;
        let price = body["items"]["data"][0]["price"]["id"]
            .as_str()
            .ok_or_else(mismatch)?;
        if !allowed_prices.contains(&price) {
            return Err(mismatch());
        }
        let current = StripeSubscriptionLookup::new(
            request.subscription_id(),
            request.customer_id(),
            request.owner_reference(),
            price,
            request.livemode(),
        )?;
        parse_response(&current, &body)
    }

    /// Reads bounded provider state for an already authorized persisted binding.
    ///
    /// Serialize this read with the subsequent domain update; two reads made
    /// outside that boundary can commit in reverse order. This read performs no
    /// mutation, grants no entitlement and does not claim an event or retry HTTP.
    pub async fn retrieve_subscription(
        &self,
        request: &StripeSubscriptionLookup,
    ) -> Result<StripeSubscriptionSnapshot, CapitalError> {
        let api_key = self.usage_api_key();
        if api_key.is_empty() || api_key.starts_with("mock_") {
            return Ok(StripeSubscriptionSnapshot {
                subscription: WebhookEvent {
                    subscription_id: request.subscription_id().to_owned(),
                    customer_id: request.customer_id().to_owned(),
                    customer_email: String::new(),
                    plan_id: request.price_id().to_owned(),
                    status: SubscriptionStatus::Unpaid,
                    ends_at: None,
                },
                provider_status: "incomplete".into(),
                owner_reference: request.owner_reference().to_owned(),
                source: StripeSubscriptionSource::Mock,
                livemode: None,
            });
        }
        let response = execute_http(
            build_request(http_client()?, api_key, request)?,
            "stripe",
            OPERATION,
        )
        .await?;
        let body: Value = read_http_json(response, "stripe", OPERATION).await?;
        parse_response(request, &body)
    }
}

fn build_request(
    client: &reqwest::Client,
    api_key: &str,
    request: &StripeSubscriptionLookup,
) -> Result<reqwest::Request, CapitalError> {
    if stripe_contract::credential_mode(api_key).is_some_and(|mode| mode != request.livemode()) {
        return Err(CapitalError::ConfigurationError(
            "subscription lookup mode does not match the credential mode".into(),
        ));
    }
    // The constructor permits only bounded ASCII identifier bytes, never URL delimiters.
    client
        .get(format!(
            "https://api.stripe.com/v1/subscriptions/{}",
            request.subscription_id()
        ))
        .bearer_auth(api_key)
        .header("Stripe-Version", stripe_contract::API_VERSION)
        .build()
        .map_err(|_| crate::ProviderFailure::request_build("stripe", OPERATION).into())
}

fn parse_response(
    request: &StripeSubscriptionLookup,
    body: &Value,
) -> Result<StripeSubscriptionSnapshot, CapitalError> {
    let subscription = stripe_webhook::parse_subscription(body).map_err(|_| mismatch())?;
    if subscription.subscription_id != request.subscription_id()
        || subscription.customer_id != request.customer_id()
        || subscription.plan_id != request.price_id()
        || body["items"]["data"][0]["price"]["type"].as_str() != Some("recurring")
        || body["livemode"].as_bool() != Some(request.livemode())
        || body["metadata"]["rullst_owner_reference"].as_str() != Some(request.owner_reference())
        || (!body["deleted"].is_null() && body["deleted"].as_bool() != Some(false))
    {
        return Err(mismatch());
    }
    let provider_status = body["status"].as_str().ok_or_else(mismatch)?.to_owned();
    Ok(StripeSubscriptionSnapshot {
        subscription,
        provider_status,
        owner_reference: request.owner_reference().to_owned(),
        source: StripeSubscriptionSource::Retrieved,
        livemode: Some(request.livemode()),
    })
}

fn mismatch() -> CapitalError {
    crate::ProviderFailure::contract_mismatch("stripe", OPERATION).into()
}

#[cfg(test)]
#[path = "stripe_snapshot_tests.rs"]
mod tests;
