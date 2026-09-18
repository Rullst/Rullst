use super::paddle_checkout::{mismatch, price_matches};
use super::{BillingProvider, PaddleProvider, SubscriptionStatus, WebhookVerificationMode};
use crate::paddle_checkout::{id, invalid};
use crate::{
    CapitalError, PaddleCheckoutRequest, PaddleCheckoutSession, PaddleSubscriptionEvent,
    PaddleSubscriptionSnapshot, WebhookEvent,
};
use serde_json::Value;
use std::collections::HashMap;

impl PaddleProvider {
    pub(super) async fn change_subscription(
        &self,
        subscription_id: &str,
        action: &'static str,
    ) -> Result<(), CapitalError> {
        if !id(subscription_id, "sub_") {
            return Err(invalid());
        }
        let value = self
            .billing_json(
                reqwest::Method::POST,
                &format!("/subscriptions/{subscription_id}/{action}"),
                None,
                "change subscription",
            )
            .await?;
        let data = &value["data"];
        let terminal = if action == "cancel" {
            "canceled"
        } else {
            "paused"
        };
        if data["id"].as_str() != Some(subscription_id)
            || (data["status"].as_str() != Some(terminal)
                && (!matches!(
                    data["status"].as_str(),
                    Some("active" | "trialing" | "past_due")
                ) || data["scheduled_change"]["action"].as_str() != Some(action)
                    || timestamp(&data["scheduled_change"]["effective_at"]).is_err()))
        {
            return Err(mismatch());
        }
        Ok(())
    }
    /// Verifies raw signed lifecycle data and every checkout owner/attempt,
    /// customer and price binding. Subscription-created includes the original
    /// transaction ID; compare it with the persisted checkout before first bind.
    /// Subsequent events must match the application's saved subscription ID.
    pub fn verify_checkout_subscription(
        &self,
        request: &PaddleCheckoutRequest,
        checkout: &PaddleCheckoutSession,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<PaddleSubscriptionEvent, CapitalError> {
        if payload.len() > 2 * 1024 * 1024 {
            return Err(mismatch());
        }
        let mode = self.webhook_verification_mode()?;
        if checkout.request_digest() != request.request_digest()
            || (mode == WebhookVerificationMode::Real && checkout.sandbox() != Some(self.sandbox))
            || (mode == WebhookVerificationMode::Mock && !checkout.is_mock())
        {
            return Err(mismatch());
        }
        self.verify_signature(
            payload,
            headers.get("paddle-signature").ok_or_else(mismatch)?,
        )?;
        let value: Value = serde_json::from_slice(payload).map_err(|_| mismatch())?;
        let event_id = value["event_id"]
            .as_str()
            .filter(|value| id(value, "evt_"))
            .ok_or_else(mismatch)?;
        let event_type = value["event_type"].as_str().ok_or_else(mismatch)?;
        let (subscription, status) = parse_subscription(request, &value["data"], None)?;
        if checkout
            .subscription_id()
            .is_some_and(|id| id != subscription.subscription_id)
        {
            return Err(mismatch());
        }
        let expected = match event_type {
            "subscription.created" | "subscription.updated" => None,
            "subscription.activated" | "subscription.resumed" => Some("active"),
            "subscription.trialing" => Some("trialing"),
            "subscription.past_due" => Some("past_due"),
            "subscription.paused" => Some("paused"),
            "subscription.canceled" => Some("canceled"),
            _ => return Err(mismatch()),
        };
        if expected.is_some_and(|expected| status != expected) {
            return Err(mismatch());
        }
        let transaction = if value["data"]["transaction_id"].is_null() {
            None
        } else {
            Some(
                value["data"]["transaction_id"]
                    .as_str()
                    .filter(|value| id(value, "txn_"))
                    .ok_or_else(mismatch)?
                    .into(),
            )
        };
        if event_type == "subscription.created" && transaction.is_none() {
            return Err(mismatch());
        }
        if transaction.as_deref().is_some_and(|id| id != checkout.id()) {
            return Err(mismatch());
        }
        if event_type != "subscription.created" && checkout.subscription_id().is_none() {
            return Err(mismatch());
        }
        Ok(PaddleSubscriptionEvent {
            owner: request.owner.clone(),
            attempt: request.attempt.clone(),
            event_id: event_id.into(),
            event_type: event_type.into(),
            occurred_at: timestamp(&value["occurred_at"])?,
            transaction,
            status,
            subscription,
            mock: mode == WebhookVerificationMode::Mock,
        })
    }

    /// Reads current state for a persisted subscription binding. Use a durable
    /// revision fence before this read and a compare-and-swap when committing.
    pub async fn retrieve_bound_subscription(
        &self,
        request: &PaddleCheckoutRequest,
        subscription_id: &str,
    ) -> Result<PaddleSubscriptionSnapshot, CapitalError> {
        if !id(subscription_id, "sub_") {
            return Err(invalid());
        }
        if self.offline() {
            return Ok(PaddleSubscriptionSnapshot {
                subscription: WebhookEvent {
                    subscription_id: subscription_id.into(),
                    customer_id: request.customer.clone(),
                    customer_email: String::new(),
                    plan_id: request.price.clone(),
                    status: SubscriptionStatus::Active,
                    ends_at: None,
                },
                status: "active".into(),
                sandbox: None,
            });
        }
        let value = self
            .billing_json(
                reqwest::Method::GET,
                &format!("/subscriptions/{subscription_id}"),
                None,
                "retrieve bound subscription",
            )
            .await?;
        let (subscription, status) =
            parse_subscription(request, &value["data"], Some(subscription_id))?;
        Ok(PaddleSubscriptionSnapshot {
            subscription,
            status,
            sandbox: Some(self.sandbox),
        })
    }
}

fn parse_subscription(
    request: &PaddleCheckoutRequest,
    data: &Value,
    expected: Option<&str>,
) -> Result<(WebhookEvent, String), CapitalError> {
    let subscription = data["id"]
        .as_str()
        .filter(|value| id(value, "sub_"))
        .ok_or_else(mismatch)?;
    if expected.is_some_and(|expected| subscription != expected)
        || data["customer_id"].as_str() != Some(&request.customer)
        || data["collection_mode"].as_str() != Some("automatic")
        || data["custom_data"]["rullst_owner_reference"].as_str() != Some(&request.owner)
        || data["custom_data"]["rullst_attempt_reference"].as_str() != Some(&request.attempt)
    {
        return Err(mismatch());
    }
    price_matches(request, &data["items"])?;
    let raw = data["status"].as_str().ok_or_else(mismatch)?;
    let status = match raw {
        "active" => SubscriptionStatus::Active,
        "trialing" => SubscriptionStatus::Trialing,
        "past_due" => SubscriptionStatus::PastDue,
        "paused" => SubscriptionStatus::Paused,
        "canceled" => SubscriptionStatus::Canceled,
        _ => return Err(mismatch()),
    };
    let ends_at = if data["current_billing_period"].is_null() {
        if !matches!(raw, "paused" | "canceled") {
            return Err(mismatch());
        }
        None
    } else {
        let start = timestamp(&data["current_billing_period"]["starts_at"])?;
        let end = timestamp(&data["current_billing_period"]["ends_at"])?;
        if end <= start {
            return Err(mismatch());
        }
        Some(end)
    };
    Ok((
        WebhookEvent {
            subscription_id: subscription.into(),
            customer_id: request.customer.clone(),
            customer_email: String::new(),
            plan_id: request.price.clone(),
            status,
            ends_at,
        },
        raw.into(),
    ))
}
fn timestamp(value: &Value) -> Result<i64, CapitalError> {
    value
        .as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|time| time.timestamp())
        .filter(|time| *time > 0)
        .ok_or_else(mismatch)
}

#[cfg(test)]
#[path = "paddle_subscription_tests.rs"]
mod tests;
