//! Signed subscription snapshots, never an order/payment settlement parser.
use crate::{CapitalError, SubscriptionStatus, WebhookEvent};
use serde_json::Value;

pub(super) fn parse(payload: &[u8]) -> Result<WebhookEvent, CapitalError> {
    if payload.is_empty() || payload.len() > 2 * 1024 * 1024 {
        return Err(invalid("subscription payload exceeds the supported bound"));
    }
    let json: Value = serde_json::from_slice(payload).map_err(|_| invalid("invalid JSON"))?;
    let kind = json["type"]
        .as_str()
        .ok_or_else(|| invalid("missing event type"))?;
    if !matches!(
        kind,
        "subscription.created"
            | "subscription.updated"
            | "subscription.active"
            | "subscription.uncanceled"
            | "subscription.canceled"
            | "subscription.revoked"
            | "subscription.cycled"
            | "subscription.past_due"
            | "subscription.paused"
            | "subscription.resumed"
            | "subscription.migrated"
    ) {
        return Err(invalid("unsupported subscription lifecycle event"));
    }
    let data = &json["data"];
    let subscription_id = identity(&data["id"])?;
    let customer_id = identity(primary(&data["customer_id"], &data["user_id"]))?;
    for customer in [
        &data["customer_id"],
        &data["user_id"],
        &data["customer"]["id"],
        &data["user"]["id"],
    ] {
        if !customer.is_null() && identity(customer)? != customer_id {
            return Err(invalid("conflicting subscription customer identities"));
        }
    }
    let plan_id = identity(primary(&data["product_id"], &data["price_id"]))?;
    let status_text = data["status"]
        .as_str()
        .ok_or_else(|| invalid("missing subscription status"))?;
    let status = match status_text {
        "active" => SubscriptionStatus::Active,
        "trialing" => SubscriptionStatus::Trialing,
        "paused" => SubscriptionStatus::Paused,
        "canceled" => SubscriptionStatus::Canceled,
        "past_due" => SubscriptionStatus::PastDue,
        "unpaid" | "incomplete" | "incomplete_expired" => SubscriptionStatus::Unpaid,
        _ => return Err(invalid("unsupported subscription status")),
    };
    if (!data["cancel_at_period_end"].is_null() && !data["cancel_at_period_end"].is_boolean())
        || (kind == "subscription.revoked" && status != SubscriptionStatus::Canceled)
        || (kind == "subscription.paused" && status != SubscriptionStatus::Paused)
        || (kind == "subscription.past_due" && status != SubscriptionStatus::PastDue)
        || (matches!(
            kind,
            "subscription.active" | "subscription.resumed" | "subscription.uncanceled"
        ) && !matches!(
            status,
            SubscriptionStatus::Active | SubscriptionStatus::Trialing
        ))
        || (kind == "subscription.uncanceled"
            && data["cancel_at_period_end"].as_bool() == Some(true))
        || (kind == "subscription.canceled"
            && status != SubscriptionStatus::Canceled
            && !(matches!(
                status,
                SubscriptionStatus::Active | SubscriptionStatus::Trialing
            ) && data["cancel_at_period_end"].as_bool() == Some(true)))
    {
        return Err(invalid("event and subscription state disagree"));
    }
    for contact in [&data["customer"], &data["user"]] {
        if !contact.is_null() && !contact.is_object() {
            return Err(invalid("invalid customer object"));
        }
    }
    let contact = primary(
        &data["customer"]["email"],
        primary(&data["user"]["email"], &data["email"]),
    );
    let email = if contact.is_null() {
        ""
    } else {
        contact
            .as_str()
            .ok_or_else(|| invalid("invalid customer contact"))?
    };
    if email.len() > 254 || email.chars().any(char::is_control) {
        return Err(invalid("invalid customer contact"));
    }
    Ok(WebhookEvent {
        subscription_id: subscription_id.to_owned(),
        customer_id: customer_id.to_owned(),
        customer_email: email.to_owned(),
        plan_id: plan_id.to_owned(),
        status,
        ends_at: period(&data["current_period_end"])?,
    })
}

fn primary<'a>(current: &'a Value, legacy: &'a Value) -> &'a Value {
    if current.is_null() { legacy } else { current }
}

fn identity(value: &Value) -> Result<&str, CapitalError> {
    value
        .as_str()
        .filter(|id| {
            !id.is_empty()
                && id.len() <= 200
                && id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        })
        .ok_or_else(|| invalid("missing or invalid subscription identity"))
}

fn period(value: &Value) -> Result<Option<i64>, CapitalError> {
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_i64()
        .or_else(|| {
            value.as_str().and_then(|text| {
                chrono::DateTime::parse_from_rfc3339(text)
                    .ok()
                    .map(|date| date.timestamp())
            })
        })
        .filter(|time| *time > 0)
        .map(Some)
        .ok_or_else(|| invalid("invalid subscription billing period"))
}

fn invalid(reason: &str) -> CapitalError {
    CapitalError::PayloadParseError(format!("Polar: {reason}"))
}
