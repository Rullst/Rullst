use super::{LemonSqueezyProvider, http_client};
use crate::{
    CapitalError, LemonSqueezyUsageRecord, MeteredBillingProvider, UsageDeduplication,
    UsageReceipt, UsageStatus, mock_usage_receipt, read_bounded_usage_json,
};
use async_trait::async_trait;
use serde_json::{Value, json};

const LEMON_USAGE_RECORDS_ENDPOINT: &str = "https://api.lemonsqueezy.com/v1/usage-records";

#[async_trait]
impl MeteredBillingProvider for LemonSqueezyProvider {
    type UsageRequest = LemonSqueezyUsageRecord;

    async fn report_metered_usage(
        &self,
        request: &Self::UsageRequest,
    ) -> Result<UsageReceipt, CapitalError> {
        execute_at(self.usage_api_key(), LEMON_USAGE_RECORDS_ENDPOINT, request).await
    }
}

async fn execute_at(
    api_key: &str,
    endpoint: &str,
    record: &LemonSqueezyUsageRecord,
) -> Result<UsageReceipt, CapitalError> {
    record.validate()?;
    if api_key.is_empty() || api_key.starts_with("mock_") {
        return mock_usage_receipt(
            "lemonsqueezy",
            record.event_key(),
            record.quantity(),
            &[
                record.subscription_item_id(),
                record.application_metric(),
                record.action().as_str(),
                record.event_key(),
            ],
        );
    }

    let response = http_client()
        .post(endpoint)
        .bearer_auth(api_key)
        .header("Accept", "application/vnd.api+json")
        .header("Content-Type", "application/vnd.api+json")
        .json(&request_body(record))
        .send()
        .await
        .map_err(|_| {
            CapitalError::ProviderRequestFailed(
                "Lemon Squeezy usage-record transport failed".to_string(),
            )
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(CapitalError::ProviderRequestFailed(format!(
            "Lemon Squeezy usage-record API returned HTTP {status}"
        )));
    }
    let body = read_bounded_usage_json(response).await?;
    bind_response(record, &body)
}

fn request_body(record: &LemonSqueezyUsageRecord) -> Value {
    json!({
        "data": {
            "type": "usage-records",
            "attributes": {
                "quantity": record.quantity(),
                "action": record.action().as_str()
            },
            "relationships": {
                "subscription-item": {
                    "data": {
                        "type": "subscription-items",
                        "id": record.subscription_item_id()
                    }
                }
            }
        }
    })
}

fn bind_response(
    record: &LemonSqueezyUsageRecord,
    body: &Value,
) -> Result<UsageReceipt, CapitalError> {
    let data = body.get("data");
    let attributes = data.and_then(|value| value.get("attributes"));
    let item_matches = attributes
        .and_then(|value| value.get("subscription_item_id"))
        .is_some_and(|value| numeric_id_matches(value, record.subscription_item_id()));
    let response_matches = data
        .and_then(|value| value.get("type"))
        .and_then(Value::as_str)
        == Some("usage-records")
        && attributes
            .and_then(|value| value.get("quantity"))
            .and_then(Value::as_u64)
            == Some(record.quantity())
        && attributes
            .and_then(|value| value.get("action"))
            .and_then(Value::as_str)
            == Some(record.action().as_str())
        && item_matches;
    if !response_matches {
        return Err(CapitalError::ProviderRequestFailed(
            "Lemon Squeezy usage response did not match the submitted record".to_string(),
        ));
    }
    let record_id = data
        .and_then(|value| value.get("id"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CapitalError::ProviderRequestFailed(
                "Lemon Squeezy usage response omitted its record ID".to_string(),
            )
        })?;

    UsageReceipt::from_verified_provider_response(
        "lemonsqueezy",
        record_id,
        record.event_key(),
        record.quantity(),
        UsageStatus::Accepted,
        UsageDeduplication::ApplicationOutboxRequired,
    )
}

fn numeric_id_matches(value: &Value, expected: &str) -> bool {
    value
        .as_u64()
        .is_some_and(|value| value.to_string() == expected)
        || value.as_str() == Some(expected)
}

#[cfg(all(test, feature = "axum"))]
#[path = "lemonsqueezy_usage_tests.rs"]
mod tests;
