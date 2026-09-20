//! Signed notification hints for one-time checkout, refunds and disputes.
//! Always reconcile a persisted attempt with `read_one_time_receipt` before a
//! transactional entitlement update; a webhook body alone grants no access.

use crate::{BillingProvider, CapitalError, StripeProvider, WebhookVerificationMode};
use serde_json::Value;
use std::collections::HashMap;

/// Cryptographically verified event identity and provider reconciliation keys.
#[derive(Clone)]
pub struct StripeOneTimeEvent {
    id: String,
    kind: String,
    session: Option<String>,
    payment_intent: Option<String>,
    livemode: bool,
    digest: [u8; 32],
}

impl StripeOneTimeEvent {
    pub fn event_id(&self) -> &str {
        &self.id
    }
    pub fn event_type(&self) -> &str {
        &self.kind
    }
    pub fn session_id(&self) -> Option<&str> {
        self.session.as_deref()
    }
    pub fn payment_intent_id(&self) -> Option<&str> {
        self.payment_intent.as_deref()
    }
    pub fn livemode(&self) -> bool {
        self.livemode
    }
    pub fn payload_digest(&self) -> [u8; 32] {
        self.digest
    }
    /// Bind this digest and event ID inside the application's account/mode inbox.
    pub fn mutation_digest(&self) -> [u8; 32] {
        let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
        hash.update(b"rullst.stripe.one-time-event.v1\0");
        hash.update(&[u8::from(self.livemode)]);
        for value in [
            self.event_id(),
            self.event_type(),
            self.session_id().unwrap_or(""),
            self.payment_intent_id().unwrap_or(""),
        ] {
            hash.update(&(value.len() as u64).to_be_bytes());
            hash.update(value.as_bytes());
        }
        let mut result = [0; 32];
        result.copy_from_slice(hash.finish().as_ref());
        result
    }
}

impl std::fmt::Debug for StripeOneTimeEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripeOneTimeEvent")
            .field("kind", &self.kind)
            .field("livemode", &self.livemode)
            .finish_non_exhaustive()
    }
}

impl StripeProvider {
    /// Verifies an exact raw payload with signature freshness and mode binding.
    /// Supports platform accounts only. Persist replay claims with domain writes.
    pub fn verify_one_time_event(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<StripeOneTimeEvent, CapitalError> {
        if payload.is_empty() || payload.len() > 2 * 1024 * 1024 {
            return Err(invalid());
        }
        if self.webhook_verification_mode()? != WebhookVerificationMode::Real {
            return Err(CapitalError::MockWebhookNotAllowed("stripe".into()));
        }
        self.verify_signature(
            payload,
            headers.get("stripe-signature").ok_or_else(invalid)?,
        )?;
        let root: Value = serde_json::from_slice(payload).map_err(|_| invalid())?;
        let data = &root["data"]["object"];
        let kind = root["type"].as_str().ok_or_else(invalid)?;
        let livemode = root["livemode"].as_bool().ok_or_else(invalid)?;
        if root["object"] != "event"
            || !root["account"].is_null()
            || root["created"].as_i64().is_none_or(|time| time <= 0)
            || data["livemode"].as_bool() != Some(livemode)
            || crate::providers::stripe_contract::credential_mode(self.usage_api_key())
                != Some(livemode)
        {
            return Err(invalid());
        }
        let (session, payment_intent) = match kind {
            "checkout.session.completed"
            | "checkout.session.async_payment_succeeded"
            | "checkout.session.async_payment_failed"
            | "checkout.session.expired" => {
                if data["object"] != "checkout.session"
                    || data["mode"] != "payment"
                    || !data["subscription"].is_null()
                {
                    return Err(invalid());
                }
                let intent = if data["payment_intent"].is_null() {
                    None
                } else {
                    Some(reference(&data["payment_intent"], "pi_")?)
                };
                (Some(reference(&data["id"], "cs_")?), intent)
            }
            "charge.refunded" => {
                if data["object"] != "charge" {
                    return Err(invalid());
                }
                (None, Some(reference(&data["payment_intent"], "pi_")?))
            }
            "charge.dispute.created"
            | "charge.dispute.updated"
            | "charge.dispute.closed"
            | "charge.dispute.funds_withdrawn"
            | "charge.dispute.funds_reinstated" => {
                if data["object"] != "dispute" {
                    return Err(invalid());
                }
                (None, Some(reference(&data["payment_intent"], "pi_")?))
            }
            _ => return Err(invalid()),
        };
        let mut digest = [0; 32];
        digest.copy_from_slice(ring::digest::digest(&ring::digest::SHA256, payload).as_ref());
        Ok(StripeOneTimeEvent {
            id: reference(&root["id"], "evt_")?,
            kind: kind.into(),
            session,
            payment_intent,
            livemode,
            digest,
        })
    }
}

fn reference(value: &Value, prefix: &str) -> Result<String, CapitalError> {
    value
        .as_str()
        .filter(|v| crate::providers::stripe_contract::valid_reference(v, prefix, 255))
        .map(str::to_owned)
        .ok_or_else(invalid)
}
fn invalid() -> CapitalError {
    CapitalError::PayloadParseError("Invalid Stripe one-time event".into())
}
