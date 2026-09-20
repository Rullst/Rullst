//! Verified Resend/Svix delivery feedback. Raw bodies and recipients are omitted
//! from Debug. Scope verifier/store instances to the same application/tenant.

use crate::{SuppressionEvent, SuppressionReason};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
};
use hmac::{Hmac, KeyInit, Mac};
use secrecy::{ExposeSecret, SecretString};
use sha2::Sha256;

/// Minimized delivery feedback classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MailFeedbackKind {
    Delivered,
    Delayed,
    PermanentBounce,
    Complaint,
    Failed,
}

/// Redacted authentication/contract failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MailFeedbackError {
    Configuration,
    InvalidSignature,
    Stale,
    InvalidPayload,
    MockNotAllowed,
}

impl std::fmt::Display for MailFeedbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mail feedback rejected: {self:?}")
    }
}
impl std::error::Error for MailFeedbackError {}

/// Authenticated delivery outcome, without provider bodies or arbitrary error text.
pub struct VerifiedMailFeedback {
    event_id: String,
    email_id: String,
    recipient: String,
    kind: MailFeedbackKind,
    observed_at: u64,
}

impl std::fmt::Debug for VerifiedMailFeedback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedMailFeedback")
            .field("kind", &self.kind)
            .field("observed_at", &self.observed_at)
            .finish_non_exhaustive()
    }
}

impl VerifiedMailFeedback {
    pub fn event_id(&self) -> &str {
        &self.event_id
    }
    pub fn email_id(&self) -> &str {
        &self.email_id
    }
    /// Sensitive lookup key for an authoritative suppression store, never logs.
    pub fn recipient(&self) -> &str {
        &self.recipient
    }
    pub fn kind(&self) -> MailFeedbackKind {
        self.kind
    }
    /// Only authenticated permanent bounces/complaints suppress future mail.
    /// `MutableSuppressionStore::record` atomically binds this replay identity.
    pub fn suppression_event(&self) -> Result<Option<SuppressionEvent>, MailFeedbackError> {
        let reason = match self.kind {
            MailFeedbackKind::PermanentBounce => SuppressionReason::HardBounce,
            MailFeedbackKind::Complaint => SuppressionReason::SpamComplaint,
            _ => return Ok(None),
        };
        SuppressionEvent::try_new(
            "resend",
            &self.event_id,
            &self.recipient,
            reason,
            self.observed_at,
        )
        .map(Some)
        .map_err(|_| MailFeedbackError::InvalidPayload)
    }
}

/// Provider-specific verifier using the exact raw payload and Svix v1 contract.
pub struct ResendFeedbackVerifier {
    secret: SecretString,
}

impl ResendFeedbackVerifier {
    pub fn new(secret: impl Into<String>) -> Result<Self, MailFeedbackError> {
        let secret = secret.into();
        if !secret.is_empty() && !secret.starts_with("mock_") {
            let key = decode_secret(&secret)?;
            if !(16..=128).contains(&key.len()) {
                return Err(MailFeedbackError::Configuration);
            }
        }
        Ok(Self {
            secret: SecretString::from(secret),
        })
    }

    /// HTTP adapters must reject duplicate signature headers before passing the
    /// three exact header values here. `now` must come from a trusted UTC clock.
    pub fn verify(
        &self,
        payload: &[u8],
        event_id: &str,
        timestamp: &str,
        signatures: &str,
        now: u64,
    ) -> Result<VerifiedMailFeedback, MailFeedbackError> {
        let secret = self.secret.expose_secret();
        if secret.is_empty() || secret.starts_with("mock_") {
            return Err(MailFeedbackError::MockNotAllowed);
        }
        if payload.is_empty()
            || payload.len() > 1024 * 1024
            || !identifier(event_id, 128)
            || timestamp.len() > 20
            || signatures.len() > 1024
        {
            return Err(MailFeedbackError::InvalidPayload);
        }
        let signed_at = timestamp
            .parse::<u64>()
            .map_err(|_| MailFeedbackError::InvalidSignature)?;
        if signed_at.abs_diff(now) > 300 {
            return Err(MailFeedbackError::Stale);
        }
        let key = decode_secret(secret)?;
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&key).map_err(|_| MailFeedbackError::Configuration)?;
        mac.update(event_id.as_bytes());
        mac.update(b".");
        mac.update(timestamp.as_bytes());
        mac.update(b".");
        mac.update(payload);
        let mut valid = false;
        for signature in signatures.split_ascii_whitespace().take(16) {
            if let Some(signature) = signature.strip_prefix("v1,")
                && let Ok(bytes) = STANDARD.decode(signature)
            {
                valid |= mac.clone().verify_slice(&bytes).is_ok();
            }
        }
        if !valid {
            return Err(MailFeedbackError::InvalidSignature);
        }
        let value: serde_json::Value =
            serde_json::from_slice(payload).map_err(|_| MailFeedbackError::InvalidPayload)?;
        let data = &value["data"];
        let kind = match value["type"].as_str() {
            Some("email.delivered") => MailFeedbackKind::Delivered,
            Some("email.delivery_delayed") => MailFeedbackKind::Delayed,
            Some("email.bounced") if data["bounce"]["type"] == "Permanent" => {
                MailFeedbackKind::PermanentBounce
            }
            Some("email.bounced")
                if matches!(
                    data["bounce"]["type"].as_str(),
                    Some("Transient" | "Undetermined")
                ) =>
            {
                MailFeedbackKind::Delayed
            }
            Some("email.complained") => MailFeedbackKind::Complaint,
            Some("email.failed") => MailFeedbackKind::Failed,
            _ => return Err(MailFeedbackError::InvalidPayload),
        };
        let recipients = data["to"]
            .as_array()
            .filter(|to| to.len() == 1)
            .ok_or(MailFeedbackError::InvalidPayload)?;
        let recipient = recipients[0]
            .as_str()
            .ok_or(MailFeedbackError::InvalidPayload)?;
        crate::validate_email_syntax(recipient).map_err(|_| MailFeedbackError::InvalidPayload)?;
        if recipient.len() > 254 {
            return Err(MailFeedbackError::InvalidPayload);
        }
        let email_id = data["email_id"]
            .as_str()
            .filter(|id| identifier(id, 128))
            .ok_or(MailFeedbackError::InvalidPayload)?;
        let created = chrono::DateTime::parse_from_rfc3339(
            value["created_at"]
                .as_str()
                .ok_or(MailFeedbackError::InvalidPayload)?,
        )
        .map_err(|_| MailFeedbackError::InvalidPayload)?
        .timestamp();
        let observed_at = u64::try_from(created).map_err(|_| MailFeedbackError::InvalidPayload)?;
        if observed_at > now.saturating_add(300) {
            return Err(MailFeedbackError::InvalidPayload);
        }
        Ok(VerifiedMailFeedback {
            event_id: event_id.into(),
            email_id: email_id.into(),
            recipient: recipient.into(),
            kind,
            observed_at,
        })
    }
}

fn identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}
fn decode_secret(secret: &str) -> Result<Vec<u8>, MailFeedbackError> {
    let encoded = secret
        .strip_prefix("whsec_")
        .ok_or(MailFeedbackError::Configuration)?;
    STANDARD
        .decode(encoded)
        .or_else(|_| STANDARD_NO_PAD.decode(encoded))
        .map_err(|_| MailFeedbackError::Configuration)
}
