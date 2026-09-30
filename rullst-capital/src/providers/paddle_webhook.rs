//! Legacy Paddle subscription normalization after signature verification.
//!
//! Only Paddle Billing `subscription.*` lifecycle events become a
//! `WebhookEvent`. Transaction, adjustment, customer, price, address and other
//! signed events are rejected instead of masquerading as subscription state.

use super::{SubscriptionStatus, WebhookEvent};
use crate::CapitalError;
use crate::paddle_checkout::id;
use serde_json::Value;

const MAX_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;

pub(super) fn parse(payload: &[u8]) -> Result<WebhookEvent, CapitalError> {
    if payload.is_empty() || payload.len() > MAX_PAYLOAD_BYTES {
        return Err(invalid("payload exceeds the supported bound"));
    }
    let json: Value = serde_json::from_slice(payload).map_err(|_| invalid("invalid JSON"))?;
    // The required subscription status for each documented lifecycle event.
    let expected = match json["event_type"].as_str() {
        Some("subscription.created" | "subscription.updated" | "subscription.imported") => None,
        Some("subscription.activated" | "subscription.resumed") => Some("active"),
        Some("subscription.trialing") => Some("trialing"),
        Some("subscription.past_due") => Some("past_due"),
        Some("subscription.paused") => Some("paused"),
        Some("subscription.canceled") => Some("canceled"),
        _ => return Err(invalid("only subscription lifecycle events are normalized")),
    };
    let data = &json["data"];
    let subscription_id = prefixed(&data["id"], "sub_", "subscription ID")?;
    let customer_id = prefixed(&data["customer_id"], "ctm_", "customer ID")?;
    // The legacy event keeps its original first-item plan projection.
    let plan_id = prefixed(&data["items"][0]["price"]["id"], "pri_", "price ID")?;
    let raw_status = data["status"]
        .as_str()
        .ok_or_else(|| invalid("missing subscription status"))?;
    let status = match raw_status {
        "active" => SubscriptionStatus::Active,
        "trialing" => SubscriptionStatus::Trialing,
        "past_due" => SubscriptionStatus::PastDue,
        "paused" => SubscriptionStatus::Paused,
        "canceled" => SubscriptionStatus::Canceled,
        _ => return Err(invalid("unsupported subscription status")),
    };
    if expected.is_some_and(|expected| expected != raw_status) {
        return Err(invalid("event type and subscription status disagree"));
    }
    // Email is optional contact data, never an ownership binding.
    let email = &data["customer"]["email"];
    let customer_email = if email.is_null() {
        ""
    } else {
        email
            .as_str()
            .filter(|email| email.len() <= 254 && !email.chars().any(char::is_control))
            .ok_or_else(|| invalid("invalid customer email"))?
    };
    let period = &data["current_billing_period"];
    let ends_at = if period.is_null() {
        None
    } else {
        Some(
            period["ends_at"]
                .as_str()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|time| time.timestamp())
                .filter(|time| *time > 0)
                .ok_or_else(|| invalid("invalid billing period end"))?,
        )
    };
    Ok(WebhookEvent {
        subscription_id,
        customer_id,
        customer_email: customer_email.to_owned(),
        plan_id,
        status,
        ends_at,
    })
}

fn prefixed(value: &Value, prefix: &str, label: &str) -> Result<String, CapitalError> {
    value
        .as_str()
        .filter(|value| id(value, prefix))
        .map(str::to_owned)
        .ok_or_else(|| invalid(&format!("missing or invalid {label}")))
}

fn invalid(reason: &str) -> CapitalError {
    CapitalError::PayloadParseError(format!("Paddle: {reason}"))
}
