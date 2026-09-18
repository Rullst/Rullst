//! Signed Checkout events alongside the subscription event contract.
use crate::{BillingProvider, CapitalError, StripeProvider, WebhookVerificationMode};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone)]
pub struct StripeCheckoutEvent {
    id: String,
    kind: String,
    owner: String,
    customer: String,
    session: String,
    subscription: Option<String>,
    attempt: String,
    livemode: bool,
    digest: [u8; 32],
}
impl StripeCheckoutEvent {
    pub fn event_id(&self) -> &str {
        &self.id
    }
    pub fn event_type(&self) -> &str {
        &self.kind
    }
    pub fn owner_reference(&self) -> &str {
        &self.owner
    }
    pub fn customer_id(&self) -> &str {
        &self.customer
    }
    pub fn session_id(&self) -> &str {
        &self.session
    }
    pub fn subscription_id(&self) -> Option<&str> {
        self.subscription.as_deref()
    }
    pub fn attempt_reference(&self) -> &str {
        &self.attempt
    }
    pub fn livemode(&self) -> bool {
        self.livemode
    }
    pub fn payload_digest(&self) -> [u8; 32] {
        self.digest
    }
    /// Stable mutation identity excluding delivery counters, JSON formatting
    /// and optional contact data. Scope the durable key by account and mode.
    pub fn mutation_digest(&self) -> [u8; 32] {
        let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
        hash.update(b"rullst.stripe.checkout-event.v1\0");
        hash.update(&[u8::from(self.livemode)]);
        for value in [
            self.event_id(),
            self.event_type(),
            self.owner_reference(),
            self.customer_id(),
            self.session_id(),
            self.subscription_id().unwrap_or(""),
            self.attempt_reference(),
        ] {
            hash.update(&(value.len() as u64).to_be_bytes());
            hash.update(value.as_bytes());
        }
        let mut digest = [0; 32];
        digest.copy_from_slice(hash.finish().as_ref());
        digest
    }
}
impl std::fmt::Debug for StripeCheckoutEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripeCheckoutEvent")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
impl StripeProvider {
    /// Verifies real platform Checkout notifications without consuming replay
    /// admission. Persist completion with domain writes; reconcile subscription
    /// state before granting access. Navigation never establishes ownership.
    pub fn verify_checkout_event(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<StripeCheckoutEvent, CapitalError> {
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
        let owner = reference(&data["client_reference_id"], "")?;
        let status = data["status"].as_str().ok_or_else(invalid)?;
        if root["object"].as_str() != Some("event")
            || !root["account"].is_null()
            || root["created"].as_i64().is_none_or(|time| time <= 0)
            || root["api_version"]
                .as_str()
                .is_none_or(|version| version.is_empty() || version.len() > 80)
            || data["object"].as_str() != Some("checkout.session")
            || data["mode"].as_str() != Some("subscription")
            || data["livemode"].as_bool() != Some(livemode)
            || data["metadata"]["rullst_owner_reference"].as_str() != Some(owner)
            || !matches!(
                (kind, status),
                ("checkout.session.expired", "expired")
                    | (
                        "checkout.session.completed"
                            | "checkout.session.async_payment_succeeded"
                            | "checkout.session.async_payment_failed",
                        "complete"
                    )
            )
        {
            return Err(invalid());
        }
        let subscription = if data["subscription"].is_null() && status == "expired" {
            None
        } else {
            Some(reference(&data["subscription"], "sub_")?.into())
        };
        let mut digest = [0; 32];
        digest.copy_from_slice(ring::digest::digest(&ring::digest::SHA256, payload).as_ref());
        Ok(StripeCheckoutEvent {
            id: reference(&root["id"], "evt_")?.into(),
            kind: kind.into(),
            owner: owner.into(),
            customer: reference(&data["customer"], "cus_")?.into(),
            session: reference(&data["id"], "cs_")?.into(),
            subscription,
            attempt: reference(&data["metadata"]["rullst_attempt_reference"], "")?.into(),
            livemode,
            digest,
        })
    }
}
fn reference<'a>(value: &'a Value, prefix: &str) -> Result<&'a str, CapitalError> {
    value
        .as_str()
        .filter(|id| crate::providers::stripe_contract::valid_reference(id, prefix, 255))
        .ok_or_else(invalid)
}
fn invalid() -> CapitalError {
    CapitalError::PayloadParseError("Invalid Stripe checkout event".into())
}
