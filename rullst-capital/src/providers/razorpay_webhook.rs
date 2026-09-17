//! Subscription lifecycle normalization after Razorpay signature verification.

use super::{SubscriptionStatus, WebhookEvent};
use crate::CapitalError;
use serde_json::Value;

fn invalid() -> CapitalError {
    CapitalError::PayloadParseError("Invalid Razorpay subscription event".into())
}

fn identifier(value: &Value, prefix: &str) -> Result<String, CapitalError> {
    value
        .as_str()
        .filter(|value| {
            value.len() > prefix.len()
                && value.len() <= 200
                && value.starts_with(prefix)
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
        .map(str::to_owned)
        .ok_or_else(invalid)
}

pub(super) fn parse(json: &Value) -> Result<WebhookEvent, CapitalError> {
    // Authentication may only authorize a future charge. A standalone payment
    // or order cannot establish this subscription's identity or current state.
    let (expected_state, status) = match json["event"].as_str() {
        Some("subscription.activated" | "subscription.charged" | "subscription.resumed") => {
            ("active", SubscriptionStatus::Active)
        }
        Some("subscription.cancelled") => ("cancelled", SubscriptionStatus::Canceled),
        Some("subscription.pending") => ("pending", SubscriptionStatus::PastDue),
        Some("subscription.halted") => ("halted", SubscriptionStatus::Unpaid),
        Some("subscription.paused") => ("paused", SubscriptionStatus::Paused),
        _ => {
            return Err(CapitalError::PayloadParseError(
                "Unsupported Razorpay event".into(),
            ));
        }
    };
    let subscription = &json["payload"]["subscription"]["entity"];
    if subscription["entity"].as_str() != Some("subscription")
        || subscription["status"].as_str() != Some(expected_state)
    {
        return Err(invalid());
    }
    let subscription_id = identifier(&subscription["id"], "sub_")?;
    let customer_id = identifier(&subscription["customer_id"], "cust_")?;
    let plan_id = identifier(&subscription["plan_id"], "plan_")?;
    let payment = &json["payload"]["payment"]["entity"];
    if !payment["customer_id"].is_null()
        && payment["customer_id"].as_str() != Some(customer_id.as_str())
    {
        return Err(invalid());
    }
    // Email is optional display/contact data, never a substitute for owner binding.
    let customer_email = payment["email"]
        .as_str()
        .or_else(|| subscription["notes"]["customer_email"].as_str())
        .unwrap_or_default();
    if customer_email.len() > 254 || customer_email.chars().any(char::is_control) {
        return Err(invalid());
    }
    let ends_at = if subscription["current_end"].is_null() {
        None
    } else {
        Some(
            subscription["current_end"]
                .as_i64()
                .filter(|time| *time > 0)
                .ok_or_else(invalid)?,
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
