use super::{StripeProvider, http_client, send_http_json, url_encode};
use crate::{
    CapitalError, MeteredBillingProvider, StripeMeterEvent, UsageDeduplication, UsageReceipt,
    UsageStatus, mock_usage_receipt,
};
use async_trait::async_trait;
use serde_json::Value;

const STRIPE_METER_EVENTS_ENDPOINT: &str = "https://api.stripe.com/v1/billing/meter_events";

#[async_trait]
impl MeteredBillingProvider for StripeProvider {
    type UsageRequest = StripeMeterEvent;

    async fn report_metered_usage(
        &self,
        request: &Self::UsageRequest,
    ) -> Result<UsageReceipt, CapitalError> {
        execute_at(
            self.usage_api_key(),
            STRIPE_METER_EVENTS_ENDPOINT,
            request,
            chrono::Utc::now().timestamp(),
        )
        .await
    }
}

async fn execute_at(
    api_key: &str,
    endpoint: &str,
    event: &StripeMeterEvent,
    now: i64,
) -> Result<UsageReceipt, CapitalError> {
    event.validate_at(now)?;
    if api_key.is_empty() || api_key.starts_with("mock_") {
        return mock_usage_receipt(
            "stripe",
            event.identifier(),
            event.value(),
            &[event.customer_id(), event.event_name(), event.identifier()],
        );
    }

    let body: Value = send_http_json(
        http_client()?
            .post(endpoint)
            .bearer_auth(api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Idempotency-Key", event.identifier())
            .body(stripe_form_body(event)),
        "stripe",
        "report metered usage",
    )
    .await?;
    bind_response(event, &body)
}

fn stripe_form_body(event: &StripeMeterEvent) -> String {
    format!(
        "event_name={}&payload%5Bvalue%5D={}&payload%5Bstripe_customer_id%5D={}&identifier={}&timestamp={}",
        url_encode(event.event_name()),
        event.value(),
        url_encode(event.customer_id()),
        url_encode(event.identifier()),
        event.occurred_at()
    )
}

fn bind_response(event: &StripeMeterEvent, body: &Value) -> Result<UsageReceipt, CapitalError> {
    let payload = body.get("payload").and_then(Value::as_object);
    let value_matches = payload
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
        .is_some_and(|value| value == event.value().to_string());
    let customer_matches = payload
        .and_then(|value| value.get("stripe_customer_id"))
        .and_then(Value::as_str)
        .is_some_and(|value| value == event.customer_id());
    let response_matches = body.get("object").and_then(Value::as_str)
        == Some("billing.meter_event")
        && body.get("event_name").and_then(Value::as_str) == Some(event.event_name())
        && body.get("identifier").and_then(Value::as_str) == Some(event.identifier())
        && body.get("timestamp").and_then(Value::as_i64) == Some(event.occurred_at())
        && value_matches
        && customer_matches;
    if !response_matches {
        return Err(
            crate::ProviderFailure::contract_mismatch("stripe", "report metered usage").into(),
        );
    }

    UsageReceipt::from_verified_provider_response(
        "stripe",
        event.identifier(),
        event.identifier(),
        event.value(),
        UsageStatus::Accepted,
        UsageDeduplication::ProviderRollingWindow,
    )
    .map_err(|_| crate::ProviderFailure::contract_mismatch("stripe", "report metered usage").into())
}

#[cfg(all(test, feature = "axum"))]
#[path = "stripe_usage_tests.rs"]
mod tests;
