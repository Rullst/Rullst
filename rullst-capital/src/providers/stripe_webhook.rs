//! Bounded normalization of signed Stripe subscription snapshots.

use crate::{CapitalError, SubscriptionStatus, WebhookEvent};
use serde_json::Value;

pub(super) fn parse(payload: &[u8]) -> Result<WebhookEvent, CapitalError> {
    if payload.is_empty() || payload.len() > 2 * 1024 * 1024 {
        return Err(invalid("subscription payload exceeds the supported bound"));
    }
    let json: Value =
        serde_json::from_slice(payload).map_err(|_| invalid("invalid subscription JSON"))?;
    let event = json["type"]
        .as_str()
        .ok_or_else(|| invalid("missing event type"))?;
    if !matches!(
        event,
        "customer.subscription.created"
            | "customer.subscription.updated"
            | "customer.subscription.deleted"
            | "customer.subscription.paused"
            | "customer.subscription.resumed"
    ) {
        return Err(invalid("unsupported subscription lifecycle event"));
    }
    let data = &json["data"]["object"];
    if data["object"].as_str() != Some("subscription") {
        return Err(invalid("event must contain a subscription object"));
    }
    let subscription_id = reference(&data["id"], "sub_")?;
    let customer_id = reference(&data["customer"], "cus_")?;
    let status = match data["status"].as_str() {
        Some("active") => SubscriptionStatus::Active,
        Some("canceled") => SubscriptionStatus::Canceled,
        Some("past_due") => SubscriptionStatus::PastDue,
        Some("unpaid" | "incomplete" | "incomplete_expired") => SubscriptionStatus::Unpaid,
        Some("trialing") => SubscriptionStatus::Trialing,
        Some("paused") => SubscriptionStatus::Paused,
        _ => return Err(invalid("unsupported subscription status")),
    };
    if (event == "customer.subscription.deleted" && status != SubscriptionStatus::Canceled)
        || (event == "customer.subscription.paused" && status != SubscriptionStatus::Paused)
        || (event == "customer.subscription.resumed"
            && matches!(
                status,
                SubscriptionStatus::Paused | SubscriptionStatus::Canceled
            ))
    {
        return Err(invalid("event and subscription status disagree"));
    }
    let items = data["items"]["data"]
        .as_array()
        .filter(|items| items.len() == 1)
        .ok_or_else(|| invalid("exactly one subscription item is required"))?;
    if !data["items"]["has_more"].is_null() && data["items"]["has_more"].as_bool() != Some(false) {
        return Err(invalid("truncated subscription items are unsupported"));
    }
    let item = &items[0];
    if !item["subscription"].is_null() && item["subscription"].as_str() != Some(subscription_id) {
        return Err(invalid("subscription item belongs to another subscription"));
    }
    let price = if item["price"].is_null() {
        &item["plan"]
    } else {
        &item["price"]
    };
    // Legacy Stripe plans may use merchant-chosen IDs instead of price_* IDs.
    let plan_id = reference(&price["id"], "")?;
    if !price["type"].is_null() && price["type"].as_str() != Some("recurring") {
        return Err(invalid("subscription price must be recurring"));
    }
    let item_end = period_end(&item["current_period_end"])?;
    let legacy_end = period_end(&data["current_period_end"])?;
    if item_end.is_some() && legacy_end.is_some() && item_end != legacy_end {
        return Err(invalid("conflicting subscription billing periods"));
    }
    let email = data["customer_email"]
        .as_str()
        .or_else(|| data["customer_details"]["email"].as_str())
        .or_else(|| data["email"].as_str())
        .unwrap_or("");
    if email.len() > 254 || email.chars().any(char::is_control) {
        return Err(invalid("invalid subscription contact field"));
    }
    Ok(WebhookEvent {
        subscription_id: subscription_id.to_owned(),
        customer_id: customer_id.to_owned(),
        customer_email: email.to_owned(),
        plan_id: plan_id.to_owned(),
        status,
        ends_at: item_end.or(legacy_end),
    })
}

fn reference<'a>(value: &'a Value, prefix: &str) -> Result<&'a str, CapitalError> {
    value
        .as_str()
        .filter(|value| crate::checkout::valid_reference(value, prefix, 200))
        .ok_or_else(|| invalid("missing or invalid subscription identity"))
}

fn period_end(value: &Value) -> Result<Option<i64>, CapitalError> {
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_i64()
        .filter(|value| *value > 0)
        .map(Some)
        .ok_or_else(|| invalid("invalid subscription billing period"))
}

fn invalid(reason: &str) -> CapitalError {
    CapitalError::PayloadParseError(format!("Stripe: {reason}"))
}
