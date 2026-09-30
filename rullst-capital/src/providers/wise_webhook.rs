//! Signed Wise `transfers#state-change` webhooks.
//!
//! Wise signs the exact request body with RSA-SHA256 (PKCS#1 v1.5) and sends
//! the Base64 signature in `X-Signature-SHA256`. Sandbox and production use
//! different keys; the host configures the keys it obtained from Wise.

use super::{PayoutStatus, WiseProvider};
use crate::error::CapitalError;
use base64::{Engine, engine::general_purpose::STANDARD};
use ring::signature::{RSA_PKCS1_2048_8192_SHA256, UnparsedPublicKey};
use serde_json::Value;
use std::collections::HashMap;

const MAX_BODY_BYTES: usize = 64 * 1024;
// Base64 of an 8192-bit signature is 1368 bytes.
const MAX_SIGNATURE_HEADER_BYTES: usize = 1_400;
const MAX_PEM_BYTES: usize = 16 * 1024;
const MAX_WEBHOOK_KEYS: usize = 4;
const PEM_BEGIN: &str = "-----BEGIN PUBLIC KEY-----";
const PEM_END: &str = "-----END PUBLIC KEY-----";
// DER AlgorithmIdentifier { rsaEncryption, NULL }.
const RSA_ENCRYPTION: &[u8] = &[
    0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05, 0x00,
];

/// A Wise transfer state accepted by the verified state-change webhook.
///
/// Only the documented transfer states below are accepted; any other value,
/// including Wise's generic `unknown`, is rejected instead of being guessed.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiseTransferState {
    /// `incoming_payment_waiting`: the transfer awaits funding.
    IncomingPaymentWaiting,
    /// `incoming_payment_initiated`: funding started but has not arrived.
    IncomingPaymentInitiated,
    /// `processing`: Wise received the funds and is processing the transfer.
    Processing,
    /// `funds_converted`: compliance checks passed and funds were converted.
    FundsConverted,
    /// `outgoing_payment_sent`: Wise paid out; the money may still bounce back.
    OutgoingPaymentSent,
    /// `charged_back`: the payer's debit failed or was reversed.
    ChargedBack,
    /// `cancelled`: the transfer was cancelled.
    Cancelled,
    /// `funds_refunded`: the transfer funds were refunded to the payer.
    FundsRefunded,
    /// `bounced_back`: the recipient bank returned the payout.
    BouncedBack,
}

impl WiseTransferState {
    pub(super) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "incoming_payment_waiting" => Self::IncomingPaymentWaiting,
            "incoming_payment_initiated" => Self::IncomingPaymentInitiated,
            "processing" => Self::Processing,
            "funds_converted" => Self::FundsConverted,
            "outgoing_payment_sent" => Self::OutgoingPaymentSent,
            "charged_back" => Self::ChargedBack,
            "cancelled" => Self::Cancelled,
            "funds_refunded" => Self::FundsRefunded,
            "bounced_back" => Self::BouncedBack,
            _ => return None,
        })
    }

    /// Returns Wise's wire name for this state.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IncomingPaymentWaiting => "incoming_payment_waiting",
            Self::IncomingPaymentInitiated => "incoming_payment_initiated",
            Self::Processing => "processing",
            Self::FundsConverted => "funds_converted",
            Self::OutgoingPaymentSent => "outgoing_payment_sent",
            Self::ChargedBack => "charged_back",
            Self::Cancelled => "cancelled",
            Self::FundsRefunded => "funds_refunded",
            Self::BouncedBack => "bounced_back",
        }
    }

    /// Maps to the coarse legacy [`PayoutStatus`] only where no meaning is lost.
    ///
    /// Charge-backs and bounce-backs have no legacy equivalent and return `None`.
    pub fn payout_status(self) -> Option<PayoutStatus> {
        match self {
            Self::IncomingPaymentWaiting
            | Self::IncomingPaymentInitiated
            | Self::Processing
            | Self::FundsConverted => Some(PayoutStatus::Processing),
            Self::OutgoingPaymentSent => Some(PayoutStatus::OutgoingPaymentSent),
            Self::Cancelled => Some(PayoutStatus::Cancelled),
            Self::FundsRefunded => Some(PayoutStatus::FundsRefunded),
            Self::ChargedBack | Self::BouncedBack => None,
        }
    }
}

/// A Wise `transfers#state-change` delivery whose exact body passed
/// `X-Signature-SHA256` verification against a configured Wise public key.
///
/// Wise does not send amount, currency or recipient in this event, so none are
/// reported. The signature covers no timestamp or delivery identity: an exact
/// replay verifies again. Bind the transfer to the application's own record,
/// process state transitions idempotently and read the transfer through an
/// authenticated API call before releasing, re-issuing or refunding money.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WiseTransferStateChange {
    transfer_id: u64,
    profile_id: Option<u64>,
    current_state: WiseTransferState,
    previous_state: Option<WiseTransferState>,
    occurred_at: i64,
}

impl WiseTransferStateChange {
    /// Wise's numeric transfer ID (`data.resource.id`).
    pub fn transfer_id(&self) -> u64 {
        self.transfer_id
    }
    /// The owning Wise profile ID, when the delivery includes one.
    pub fn profile_id(&self) -> Option<u64> {
        self.profile_id
    }
    /// The state the transfer entered.
    pub fn current_state(&self) -> WiseTransferState {
        self.current_state
    }
    /// The previous state; `None` when Wise sent `null` or omitted it.
    pub fn previous_state(&self) -> Option<WiseTransferState> {
        self.previous_state
    }
    /// The state-change time (`data.occurred_at`) in Unix seconds.
    pub fn occurred_at(&self) -> i64 {
        self.occurred_at
    }
}

impl WiseProvider {
    /// Adds a Wise webhook public key in SubjectPublicKeyInfo PEM form
    /// (`-----BEGIN PUBLIC KEY-----`).
    ///
    /// Use the key Wise publishes for the matching environment: sandbox and
    /// production keys differ, and Rullst does not bundle either. Call it again
    /// (up to four keys) to accept a rotated key during a changeover. Only RSA
    /// keys of 2048 to 8192 bits are accepted.
    pub fn with_webhook_public_key_pem(
        mut self,
        pem: impl AsRef<str>,
    ) -> Result<Self, CapitalError> {
        if self.webhook_keys.len() >= MAX_WEBHOOK_KEYS {
            return Err(key_error(
                "at most four Wise webhook public keys may be configured",
            ));
        }
        let key = rsa_public_key_from_pem(pem.as_ref())?;
        self.webhook_keys.push(key);
        Ok(self)
    }

    /// Verifies a Wise `transfers#state-change` webhook and returns its state.
    ///
    /// `headers` uses lowercase names, as Rullst's webhook middleware provides.
    /// The Base64 `x-signature-sha256` value must be a valid RSA-SHA256
    /// signature of the exact `payload` bytes under a configured key; the body
    /// is not parsed before that check. The event must name a transfer with a
    /// positive numeric ID, a documented current state and a valid
    /// `occurred_at`. Nothing missing is replaced with a default.
    pub fn verify_transfer_state_change(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WiseTransferStateChange, CapitalError> {
        if self.webhook_keys.is_empty() {
            return Err(key_error("no Wise webhook public key is configured"));
        }
        if payload.is_empty() || payload.len() > MAX_BODY_BYTES {
            return Err(invalid("payload is empty or exceeds 64 KiB"));
        }
        let header = headers
            .get("x-signature-sha256")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| signature_error("missing X-Signature-SHA256 header"))?;
        if header.len() > MAX_SIGNATURE_HEADER_BYTES {
            return Err(signature_error("X-Signature-SHA256 header is too large"));
        }
        let signature = STANDARD
            .decode(header)
            .map_err(|_| signature_error("X-Signature-SHA256 is not valid Base64"))?;
        let verified = self.webhook_keys.iter().any(|key| {
            UnparsedPublicKey::new(&RSA_PKCS1_2048_8192_SHA256, key)
                .verify(payload, &signature)
                .is_ok()
        });
        if !verified {
            return Err(signature_error("signature verification failed"));
        }
        parse_state_change(payload)
    }
}

fn parse_state_change(payload: &[u8]) -> Result<WiseTransferStateChange, CapitalError> {
    let json: Value = serde_json::from_slice(payload).map_err(|_| invalid("invalid JSON"))?;
    if json["event_type"].as_str() != Some("transfers#state-change") {
        return Err(invalid("unsupported event type"));
    }
    let data = &json["data"];
    let resource = &data["resource"];
    if resource["type"].as_str() != Some("transfer") {
        return Err(invalid("resource is not a transfer"));
    }
    let transfer_id = positive_id(&resource["id"], "transfer ID")?;
    let profile_id = if resource["profile_id"].is_null() {
        None
    } else {
        Some(positive_id(&resource["profile_id"], "profile ID")?)
    };
    let current_state =
        state(&data["current_state"]).ok_or_else(|| invalid("missing or unknown current_state"))?;
    let previous_state = if data["previous_state"].is_null() {
        None
    } else {
        Some(state(&data["previous_state"]).ok_or_else(|| invalid("unknown previous_state"))?)
    };
    let occurred_at = data["occurred_at"]
        .as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|time| time.timestamp())
        .filter(|time| *time > 0)
        .ok_or_else(|| invalid("missing or invalid occurred_at"))?;
    Ok(WiseTransferStateChange {
        transfer_id,
        profile_id,
        current_state,
        previous_state,
        occurred_at,
    })
}

fn positive_id(value: &Value, label: &str) -> Result<u64, CapitalError> {
    value
        .as_u64()
        .filter(|id| *id > 0)
        .ok_or_else(|| invalid(&format!("missing or invalid {label}")))
}

fn state(value: &Value) -> Option<WiseTransferState> {
    value.as_str().and_then(WiseTransferState::parse)
}

fn rsa_public_key_from_pem(pem: &str) -> Result<Vec<u8>, CapitalError> {
    if pem.len() > MAX_PEM_BYTES {
        return Err(key_error("public key PEM is too large"));
    }
    let body = pem
        .trim()
        .strip_prefix(PEM_BEGIN)
        .and_then(|rest| rest.strip_suffix(PEM_END))
        .ok_or_else(|| key_error("expected a single SubjectPublicKeyInfo PEM block"))?;
    let base64: String = body.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let der = STANDARD
        .decode(base64)
        .map_err(|_| key_error("public key PEM is not valid Base64"))?;
    rsa_public_key_from_spki(&der)
        .ok_or_else(|| key_error("expected an RSA public key of 2048 to 8192 bits"))
}

// SubjectPublicKeyInfo ::= SEQUENCE { AlgorithmIdentifier, BIT STRING }.
// ring expects the RSAPublicKey carried by the BIT STRING.
fn rsa_public_key_from_spki(der: &[u8]) -> Option<Vec<u8>> {
    let (spki, rest) = element(der, 0x30)?;
    if !rest.is_empty() {
        return None;
    }
    let algorithm = spki.get(..RSA_ENCRYPTION.len())?;
    if algorithm != RSA_ENCRYPTION {
        return None;
    }
    let (bits, rest) = element(&spki[RSA_ENCRYPTION.len()..], 0x03)?;
    let (&unused_bits, key) = bits.split_first()?;
    if unused_bits != 0 || !rest.is_empty() {
        return None;
    }
    let (fields, rest) = element(key, 0x30)?;
    let (modulus, fields) = element(fields, 0x02)?;
    let (exponent, fields) = element(fields, 0x02)?;
    if !rest.is_empty() || !fields.is_empty() {
        return None;
    }
    let modulus_bits = positive_integer_bits(modulus)?;
    let exponent_bits = positive_integer_bits(exponent)?;
    if !(2048..=8192).contains(&modulus_bits) || !(2..=33).contains(&exponent_bits) {
        return None;
    }
    Some(key.to_vec())
}

// Returns the bit length of a minimally encoded, positive DER INTEGER.
fn positive_integer_bits(content: &[u8]) -> Option<usize> {
    let magnitude = match content {
        [0, next, ..] if *next >= 0x80 => &content[1..],
        [first, ..] if *first != 0 && *first < 0x80 => content,
        _ => return None,
    };
    let first = *magnitude.first()?;
    Some((magnitude.len() - 1) * 8 + (8 - first.leading_zeros() as usize))
}

// Reads one definite-length DER element with the expected tag.
fn element(input: &[u8], tag: u8) -> Option<(&[u8], &[u8])> {
    let (&found, rest) = input.split_first()?;
    if found != tag {
        return None;
    }
    let (&first, rest) = rest.split_first()?;
    let (length, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let count = usize::from(first & 0x7f);
        if !(1..=2).contains(&count) || rest.len() < count || rest[0] == 0 {
            return None;
        }
        let (bytes, rest) = rest.split_at(count);
        let length = bytes
            .iter()
            .fold(0_usize, |length, byte| (length << 8) | usize::from(*byte));
        if length < 0x80 {
            return None;
        }
        (length, rest)
    };
    (rest.len() >= length).then(|| rest.split_at(length))
}

fn invalid(reason: &str) -> CapitalError {
    CapitalError::PayloadParseError(format!("Wise: {reason}"))
}

fn signature_error(reason: &str) -> CapitalError {
    CapitalError::InvalidSignature(format!("Wise: {reason}"))
}

fn key_error(reason: &str) -> CapitalError {
    CapitalError::ConfigurationError(format!("Wise: {reason}"))
}

#[cfg(test)]
#[path = "wise_webhook_tests.rs"]
mod tests;
