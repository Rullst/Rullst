//! Subscription lifecycle normalization, separate from subscription invoices.
use crate::{CapitalError, SubscriptionStatus, WebhookEvent};
use serde_json::Value;

pub(super) fn parse(
    payload: &[u8],
    expected_store: Option<&str>,
) -> Result<WebhookEvent, CapitalError> {
    if payload.is_empty() || payload.len() > 2 * 1024 * 1024 {
        return Err(invalid("subscription payload exceeds the supported bound"));
    }
    let json: Value = serde_json::from_slice(payload).map_err(|_| invalid("invalid JSON"))?;
    let kind = json["meta"]["event_name"]
        .as_str()
        .ok_or_else(|| invalid("missing event type"))?;
    if !matches!(
        kind,
        "subscription_created"
            | "subscription_updated"
            | "subscription_cancelled"
            | "subscription_resumed"
            | "subscription_expired"
            | "subscription_paused"
            | "subscription_unpaused"
    ) {
        return Err(invalid("unsupported subscription lifecycle event"));
    }
    let data = &json["data"];
    if data["type"].as_str() != Some("subscriptions") {
        return Err(invalid("event must contain a subscriptions object"));
    }
    let attrs = &data["attributes"];
    let subscription_id = identity(&data["id"])?;
    let customer_id = identity(&attrs["customer_id"])?;
    let plan_id = identity(&attrs["variant_id"])?;
    let store = identity(&attrs["store_id"])?;
    if expected_store.is_some_and(|expected| store != expected) {
        return Err(invalid("subscription belongs to another store"));
    }
    let test_mode = attrs["test_mode"]
        .as_bool()
        .ok_or_else(|| invalid("missing subscription mode"))?;
    if let Some(mode) = json["meta"].get("test_mode")
        && mode.as_bool() != Some(test_mode)
    {
        return Err(invalid("conflicting subscription mode"));
    }
    let status_text = attrs["status"]
        .as_str()
        .ok_or_else(|| invalid("missing subscription status"))?;
    let status = match status_text {
        "active" => SubscriptionStatus::Active,
        "on_trial" => SubscriptionStatus::Trialing,
        "paused" => SubscriptionStatus::Paused,
        "past_due" => SubscriptionStatus::PastDue,
        "unpaid" => SubscriptionStatus::Unpaid,
        "cancelled" | "expired" => SubscriptionStatus::Canceled,
        _ => return Err(invalid("unsupported subscription status")),
    };
    if (kind == "subscription_cancelled" && status_text != "cancelled")
        || (kind == "subscription_expired" && status_text != "expired")
        || (kind == "subscription_paused" && status_text != "paused")
        || (matches!(kind, "subscription_resumed" | "subscription_unpaused")
            && matches!(status_text, "cancelled" | "expired" | "paused"))
    {
        return Err(invalid("event and subscription status disagree"));
    }
    let ends_at = if attrs["ends_at"].is_null() {
        None
    } else {
        Some(
            attrs["ends_at"]
                .as_str()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.timestamp())
                .filter(|value| *value > 0)
                .ok_or_else(|| invalid("invalid subscription end timestamp"))?,
        )
    };
    if status == SubscriptionStatus::Canceled && ends_at.is_none() {
        return Err(invalid(
            "cancelled or expired subscription requires an end timestamp",
        ));
    }
    let email = if attrs["user_email"].is_null() {
        ""
    } else {
        attrs["user_email"]
            .as_str()
            .ok_or_else(|| invalid("invalid contact field"))?
    };
    if email.len() > 254 || email.chars().any(char::is_control) {
        return Err(invalid("invalid contact field"));
    }
    Ok(WebhookEvent {
        subscription_id,
        customer_id,
        customer_email: email.to_owned(),
        plan_id,
        status,
        ends_at,
    })
}

fn identity(value: &Value) -> Result<String, CapitalError> {
    let id = if let Some(text) = value.as_str() {
        if text.len() > 20 {
            return Err(invalid("invalid subscription identity"));
        }
        text.parse::<u64>().ok().filter(|id| id.to_string() == text)
    } else {
        value.as_u64()
    };
    id.filter(|id| *id > 0)
        .map(|id| id.to_string())
        .ok_or_else(|| invalid("missing or invalid subscription identity"))
}

fn invalid(reason: &str) -> CapitalError {
    CapitalError::PayloadParseError(format!("Lemon Squeezy: {reason}"))
}
