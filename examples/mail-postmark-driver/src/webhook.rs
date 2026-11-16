//! Postmark's own suppressions, from its `SubscriptionChange` webhook, as
//! `SuppressionEvent`s for a `MutableSuppressionStore`.
//!
//! Postmark does not sign webhooks; its documented protection is HTTP Basic
//! credentials in the webhook URL (over HTTPS) and, optionally, its IP
//! ranges. [`PostmarkWebhook`] checks the `Authorization` header in constant
//! time before reading the body.

use crate::PROVIDER;
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, SecondsFormat};
use rullst_mail::{SuppressionEvent, SuppressionReason};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;
use subtle::ConstantTimeEq;

/// Largest webhook body read.
const MAX_WEBHOOK_BYTES: usize = 64 * 1024;
/// Shortest Basic-auth password accepted.
const MIN_PASSWORD_BYTES: usize = 16;

/// Why a webhook was refused. It never repeats the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WebhookError {
    /// The username is empty or holds `:`, or the password is shorter than
    /// 16 bytes; both must be printable ASCII without spaces.
    Configuration,
    /// The `Authorization` header is missing or wrong (answer 401).
    Unauthorized,
    /// The body is not a `SubscriptionChange` Rullst can record (answer 400).
    InvalidPayload,
}

impl fmt::Display for WebhookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Configuration => "Postmark webhook credentials are malformed",
            Self::Unauthorized => "Postmark webhook credentials do not match",
            Self::InvalidPayload => "Postmark webhook body is not a subscription change",
        })
    }
}

impl std::error::Error for WebhookError {}

/// Authenticates Postmark webhook requests and reads suppressions from them.
pub struct PostmarkWebhook {
    /// The whole expected header value, `Basic <base64(user:password)>`.
    expected: Vec<u8>,
}

impl PostmarkWebhook {
    /// The Basic credentials configured in the webhook URL
    /// (`https://user:password@example.com/webhooks/postmark`).
    pub fn try_new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, WebhookError> {
        let (username, password) = (username.into(), password.into());
        let printable = |value: &str| value.bytes().all(|byte| byte.is_ascii_graphic());
        if username.is_empty()
            || username.contains(':')
            || !printable(&username)
            || password.len() < MIN_PASSWORD_BYTES
            || !printable(&password)
        {
            return Err(WebhookError::Configuration);
        }
        let credentials = STANDARD.encode(format!("{username}:{password}"));
        Ok(Self {
            expected: format!("Basic {credentials}").into_bytes(),
        })
    }

    /// The suppression in an authenticated request. `Ok(None)` is another
    /// record type or a reactivation (`SuppressSending: false`): acknowledge
    /// it with 200, since lifting a suppression is an operator's decision.
    ///
    /// The event ID is the change time and a digest of the recipient, so a
    /// retried delivery is the same event and one change of many
    /// recipients is many events.
    pub fn suppression_event(
        &self,
        authorization: Option<&str>,
        body: &[u8],
    ) -> Result<Option<SuppressionEvent>, WebhookError> {
        let presented = authorization.unwrap_or_default().as_bytes();
        if !bool::from(presented.ct_eq(self.expected.as_slice())) {
            return Err(WebhookError::Unauthorized);
        }
        if body.len() > MAX_WEBHOOK_BYTES {
            return Err(WebhookError::InvalidPayload);
        }
        let invalid = || WebhookError::InvalidPayload;
        let change: Value = serde_json::from_slice(body).map_err(|_| invalid())?;
        if change["RecordType"] != "SubscriptionChange" || change["SuppressSending"] != true {
            return Ok(None);
        }
        let reason = match change["SuppressionReason"].as_str() {
            Some("HardBounce") => SuppressionReason::HardBounce,
            Some("SpamComplaint") => SuppressionReason::SpamComplaint,
            // Postmark also uses it for unsubscriptions.
            Some("ManualSuppression") => SuppressionReason::Manual,
            _ => return Err(invalid()),
        };
        let recipient = change["Recipient"].as_str().ok_or_else(invalid)?;
        let changed_at = change["ChangedAt"]
            .as_str()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .ok_or_else(invalid)?
            .to_utc();
        let observed_at = u64::try_from(changed_at.timestamp()).map_err(|_| invalid())?;
        let digest = Sha256::digest(recipient.as_bytes());
        let digest: String = digest
            .iter()
            .take(16)
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let event_id = format!(
            "{}:{digest}",
            changed_at.to_rfc3339_opts(SecondsFormat::Nanos, true)
        );
        SuppressionEvent::try_new(PROVIDER, event_id, recipient, reason, observed_at)
            .map(Some)
            .map_err(|_| invalid())
    }
}
