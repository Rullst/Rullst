use crate::{BillingProvider, CapitalError, StripeSubscriptionEvent};
use serde_json::Value;
use std::collections::HashMap;

impl super::StripeProvider {
    /// Verifies a bounded event without claiming replay admission before domain processing.
    ///
    /// A production consumer must call `require_real`, bind the configured
    /// account/mode and persisted customer/owner, then claim the stable event
    /// ID in the same transaction as domain changes. Do not combine this path
    /// with middleware that consumes replay admission before your handler.
    pub fn verify_subscription_event(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<StripeSubscriptionEvent, CapitalError> {
        if payload.is_empty() || payload.len() > 2 * 1024 * 1024 {
            return Err(invalid());
        }
        let subscription = self.handle_webhook(payload, headers)?;
        let verification_mode = self.webhook_verification_mode()?;
        let json: Value = serde_json::from_slice(payload).map_err(|_| invalid())?;
        let event_id = reference(&json["id"], "evt_")?;
        let api_version = json["api_version"]
            .as_str()
            .filter(|version| {
                !version.is_empty()
                    && version.len() <= 80
                    && version
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
            })
            .ok_or_else(invalid)?;
        let event_type = json["type"].as_str().ok_or_else(invalid)?;
        let created_at = json["created"]
            .as_i64()
            .filter(|time| *time > 0)
            .ok_or_else(invalid)?;
        let livemode = json["livemode"].as_bool().ok_or_else(invalid)?;
        let data = &json["data"]["object"];
        if json["object"].as_str() != Some("event") || data["livemode"].as_bool() != Some(livemode)
        {
            return Err(invalid());
        }
        let connected_account = if json["account"].is_null() {
            None
        } else {
            Some(reference(&json["account"], "acct_")?.to_owned())
        };
        let metadata = data["metadata"].as_object().ok_or_else(invalid)?;
        let owner_reference = metadata
            .get("rullst_owner_reference")
            .map(|value| reference(value, "").map(str::to_owned))
            .transpose()?;
        let provider_status = data["status"].as_str().ok_or_else(invalid)?.to_owned();
        let mut payload_digest = [0; 32];
        payload_digest
            .copy_from_slice(ring::digest::digest(&ring::digest::SHA256, payload).as_ref());
        Ok(StripeSubscriptionEvent {
            event_id: event_id.to_owned(),
            event_type: event_type.to_owned(),
            api_version: api_version.to_owned(),
            created_at,
            livemode,
            connected_account,
            owner_reference,
            provider_status,
            subscription,
            verification_mode,
            payload_digest,
        })
    }
}

fn reference<'a>(value: &'a Value, prefix: &str) -> Result<&'a str, CapitalError> {
    value
        .as_str()
        .filter(|value| super::stripe_contract::valid_reference(value, prefix, 200))
        .ok_or_else(invalid)
}

fn invalid() -> CapitalError {
    CapitalError::PayloadParseError("Invalid Stripe subscription event envelope".into())
}
