//! Bounded authenticated delivery for secret-minimized AI audit events.

use super::AiCancellation;
use futures_util::StreamExt as _;
use hmac::{Hmac, KeyInit, Mac};
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{fmt, time::Duration};
use zeroize::Zeroizing;

mod validation;

use validation::{ClientMode, EndpointScope, validate_endpoint, validate_identifier, validate_key};

const MAX_EVENT_BYTES: usize = 16 * 1_024;
const MAX_ACK_BYTES: usize = 8 * 1_024;
const MAX_ACK_BYTES_U64: u64 = 8 * 1_024;
const MAX_UNIX_MILLIS: u64 = 9_223_372_036_854_775_807;
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(5);
const SIGNATURE_DOMAIN: &[u8] = b"RULLST-AI-AUDIT-V1\n";

/// Whether a receipt came from a live endpoint or the explicit offline fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AuditDeliveryMode {
    /// An authenticated remote endpoint returned a bound acknowledgement.
    Live,
    /// Empty or `mock_*` credentials selected deterministic offline behavior.
    OfflineMock,
}

/// Bounded retry configuration for an idempotent audit event identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AuditRetryPolicy {
    max_attempts: u8,
    base_delay: Duration,
}

impl AuditRetryPolicy {
    /// Creates a policy with one to five attempts and a delay no greater than five seconds.
    pub fn try_new(max_attempts: u8, base_delay: Duration) -> Result<Self, AuditDeliveryError> {
        if !(1..=5).contains(&max_attempts) {
            return Err(AuditDeliveryError::InvalidConfiguration(
                "audit delivery attempts must be between 1 and 5",
            ));
        }
        if base_delay > MAX_RETRY_DELAY {
            return Err(AuditDeliveryError::InvalidConfiguration(
                "audit delivery base delay cannot exceed 5 seconds",
            ));
        }
        Ok(Self {
            max_attempts,
            base_delay,
        })
    }

    fn delay_for(self, completed_attempts: u8) -> Duration {
        let exponent = u32::from(completed_attempts.saturating_sub(1)).min(4);
        self.base_delay
            .checked_mul(1_u32 << exponent)
            .unwrap_or(MAX_RETRY_DELAY)
            .min(MAX_RETRY_DELAY)
    }
}

impl Default for AuditRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(200),
        }
    }
}

/// Typed delivery failures that never include an endpoint, key, or event body.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum AuditDeliveryError {
    /// Constructor or request metadata violated the closed contract.
    #[error("invalid AI audit delivery configuration: {0}")]
    InvalidConfiguration(&'static str),
    /// JSON encoding failed without exposing event data.
    #[error("AI audit event encoding failed")]
    Encoding,
    /// The serialized envelope exceeded the fixed per-event limit.
    #[error("AI audit event exceeds 16 KiB")]
    EventTooLarge,
    /// The caller cancelled before a bound acknowledgement was received.
    #[error("AI audit delivery was cancelled")]
    Cancelled,
    /// The request failed locally without exposing transport details.
    #[error("AI audit transport failed")]
    Transport,
    /// The configured request deadline elapsed.
    #[error("AI audit delivery deadline elapsed")]
    Deadline,
    /// A non-retryable response or exhausted transient response was returned.
    #[error("AI audit endpoint rejected the event with HTTP {status}")]
    Rejected {
        /// Numeric HTTP status without response content.
        status: u16,
    },
    /// The acknowledgement exceeded the response limit.
    #[error("AI audit acknowledgement exceeds 8 KiB")]
    AckTooLarge,
    /// The acknowledgement was malformed or did not bind the event identifier.
    #[error("AI audit endpoint returned an invalid acknowledgement")]
    InvalidAck,
}

/// Successful delivery evidence without an event body or secret.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AuditDeliveryReceipt {
    event_id: String,
    attempts: u8,
    mode: AuditDeliveryMode,
}

impl AuditDeliveryReceipt {
    /// Stable caller-supplied event identifier acknowledged by the endpoint.
    #[must_use]
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    /// Number of attempts used by this call.
    #[must_use]
    pub const fn attempts(&self) -> u8 {
        self.attempts
    }

    /// Distinguishes live acknowledgement from deterministic offline acceptance.
    #[must_use]
    pub const fn mode(&self) -> AuditDeliveryMode {
        self.mode
    }
}

/// HMAC-authenticated HTTP delivery for minimized RAG/tool/provider audit records.
///
/// The receiving service must deduplicate `event_id`, validate the exact body
/// signature and return the documented acknowledgement. The client does not
/// retain an event after this future returns; durable retry orchestration remains
/// the application's responsibility.
#[non_exhaustive]
pub struct AuditDeliveryClient {
    endpoint: String,
    source: String,
    key_id: String,
    signing_key: Zeroizing<Vec<u8>>,
    client: reqwest::Client,
    request_timeout: Duration,
    retry: AuditRetryPolicy,
    mode: ClientMode,
}

impl AuditDeliveryClient {
    /// Creates a remote HTTPS client. Empty or `mock_*` keys select offline mode.
    pub fn try_cloud(
        endpoint: impl Into<String>,
        source: impl Into<String>,
        key_id: impl Into<String>,
        signing_key: impl Into<String>,
    ) -> Result<Self, AuditDeliveryError> {
        Self::try_build(
            endpoint.into(),
            source.into(),
            key_id.into(),
            signing_key.into(),
            EndpointScope::Cloud,
        )
    }

    /// Creates a development client for a literal loopback HTTP(S) endpoint.
    pub fn try_local(
        endpoint: impl Into<String>,
        source: impl Into<String>,
        key_id: impl Into<String>,
        signing_key: impl Into<String>,
    ) -> Result<Self, AuditDeliveryError> {
        Self::try_build(
            endpoint.into(),
            source.into(),
            key_id.into(),
            signing_key.into(),
            EndpointScope::Loopback,
        )
    }

    fn try_build(
        endpoint: String,
        source: String,
        key_id: String,
        signing_key: String,
        scope: EndpointScope,
    ) -> Result<Self, AuditDeliveryError> {
        validate_identifier("source", &source)?;
        validate_identifier("key ID", &key_id)?;
        let endpoint = validate_endpoint(endpoint, scope)?;
        let mode = if signing_key.is_empty() || signing_key.starts_with("mock_") {
            ClientMode::Mock
        } else {
            ClientMode::Live
        };
        validate_key(&signing_key, mode)?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|_| AuditDeliveryError::Transport)?;
        Ok(Self {
            endpoint,
            source,
            key_id,
            signing_key: Zeroizing::new(signing_key.into_bytes()),
            client,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            retry: AuditRetryPolicy::default(),
            mode,
        })
    }

    /// Selects a request deadline. Zero and values above five minutes fail closed.
    pub fn try_with_request_timeout(
        mut self,
        timeout: Duration,
    ) -> Result<Self, AuditDeliveryError> {
        if timeout.is_zero() || timeout > Duration::from_secs(300) {
            return Err(AuditDeliveryError::InvalidConfiguration(
                "audit delivery timeout must be between 1 nanosecond and 5 minutes",
            ));
        }
        self.request_timeout = timeout;
        Ok(self)
    }

    /// Selects a bounded idempotent retry policy.
    #[must_use]
    pub fn with_retry_policy(mut self, retry: AuditRetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Signs and delivers one minimized serializable event.
    pub async fn publish<E>(
        &self,
        event_id: impl Into<String>,
        occurred_at_ms: u64,
        event: &E,
        cancellation: &AiCancellation,
    ) -> Result<AuditDeliveryReceipt, AuditDeliveryError>
    where
        E: Serialize + Sync,
    {
        let event_id = event_id.into();
        validate_identifier("event ID", &event_id)?;
        if occurred_at_ms == 0 || occurred_at_ms > MAX_UNIX_MILLIS {
            return Err(AuditDeliveryError::InvalidConfiguration(
                "audit event time is outside the supported Unix millisecond range",
            ));
        }
        if cancellation.is_cancelled() {
            return Err(AuditDeliveryError::Cancelled);
        }
        let body = serde_json::to_vec(&AuditEnvelope {
            schema_version: 1,
            source: &self.source,
            event_id: &event_id,
            occurred_at_ms,
            event,
        })
        .map_err(|_| AuditDeliveryError::Encoding)?;
        if body.len() > MAX_EVENT_BYTES {
            return Err(AuditDeliveryError::EventTooLarge);
        }
        if self.mode == ClientMode::Mock {
            return Ok(AuditDeliveryReceipt {
                event_id,
                attempts: 1,
                mode: AuditDeliveryMode::OfflineMock,
            });
        }

        let signature = self.signature(occurred_at_ms, &body)?;
        let mut attempt = 1;
        loop {
            match self
                .attempt(&event_id, occurred_at_ms, &signature, &body, cancellation)
                .await
            {
                Ok(()) => {
                    return Ok(AuditDeliveryReceipt {
                        event_id,
                        attempts: attempt,
                        mode: AuditDeliveryMode::Live,
                    });
                }
                Err(error) if attempt < self.retry.max_attempts && retryable_delivery(&error) => {
                    let delay = self.retry.delay_for(attempt);
                    tokio::select! {
                        () = cancellation.cancelled() => return Err(AuditDeliveryError::Cancelled),
                        () = tokio::time::sleep(delay) => {}
                    }
                    attempt = attempt.saturating_add(1);
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn attempt(
        &self,
        event_id: &str,
        occurred_at_ms: u64,
        signature: &str,
        body: &[u8],
        cancellation: &AiCancellation,
    ) -> Result<(), AuditDeliveryError> {
        let request = self
            .client
            .post(&self.endpoint)
            .timeout(self.request_timeout)
            .header(CONTENT_TYPE, "application/json")
            .header("x-rullst-ai-key-id", &self.key_id)
            .header("x-rullst-ai-timestamp", occurred_at_ms.to_string())
            .header("x-rullst-ai-signature", signature)
            .body(body.to_vec());
        let response = tokio::select! {
            () = cancellation.cancelled() => return Err(AuditDeliveryError::Cancelled),
            response = request.send() => response.map_err(classify_transport_error)?,
        };
        let status = response.status();
        if !status.is_success() {
            return Err(AuditDeliveryError::Rejected {
                status: status.as_u16(),
            });
        }
        let content_type_is_json = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
        if !content_type_is_json {
            return Err(AuditDeliveryError::InvalidAck);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_ACK_BYTES_U64)
        {
            return Err(AuditDeliveryError::AckTooLarge);
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        loop {
            let next = tokio::select! {
                () = cancellation.cancelled() => return Err(AuditDeliveryError::Cancelled),
                next = stream.next() => next,
            };
            let Some(chunk) = next else {
                break;
            };
            let chunk = chunk.map_err(classify_transport_error)?;
            if bytes.len().saturating_add(chunk.len()) > MAX_ACK_BYTES {
                return Err(AuditDeliveryError::AckTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        let ack: AuditAck =
            serde_json::from_slice(&bytes).map_err(|_| AuditDeliveryError::InvalidAck)?;
        if ack.schema_version != 1 || !ack.accepted || ack.event_id != event_id {
            return Err(AuditDeliveryError::InvalidAck);
        }
        Ok(())
    }

    fn signature(&self, occurred_at_ms: u64, body: &[u8]) -> Result<String, AuditDeliveryError> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.signing_key)
            .map_err(|_| AuditDeliveryError::InvalidConfiguration("invalid signing key"))?;
        mac.update(SIGNATURE_DOMAIN);
        mac.update(self.key_id.as_bytes());
        mac.update(b"\n");
        mac.update(occurred_at_ms.to_string().as_bytes());
        mac.update(b"\n");
        mac.update(body);
        Ok(format!(
            "sha256={}",
            hex(mac.finalize().into_bytes().as_slice())
        ))
    }
}

impl fmt::Debug for AuditDeliveryClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuditDeliveryClient")
            .field("endpoint", &"[CONFIGURED]")
            .field("source", &self.source)
            .field("key_id", &self.key_id)
            .field("signing_key", &"[REDACTED]")
            .field("request_timeout", &self.request_timeout)
            .field("retry", &self.retry)
            .field("mode", &self.mode)
            .finish()
    }
}

#[derive(Serialize)]
struct AuditEnvelope<'a, E> {
    schema_version: u8,
    source: &'a str,
    event_id: &'a str,
    occurred_at_ms: u64,
    event: &'a E,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditAck {
    schema_version: u8,
    event_id: String,
    accepted: bool,
}

fn classify_transport_error(error: reqwest::Error) -> AuditDeliveryError {
    if error.is_timeout() {
        AuditDeliveryError::Deadline
    } else {
        AuditDeliveryError::Transport
    }
}

fn retryable_delivery(error: &AuditDeliveryError) -> bool {
    matches!(
        error,
        AuditDeliveryError::Transport | AuditDeliveryError::Deadline
    ) || matches!(error, AuditDeliveryError::Rejected { status } if *status == 429 || *status >= 500)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        encoded.push(DIGITS[usize::from(byte >> 4)] as char);
        encoded.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    encoded
}

#[cfg(test)]
mod tests;
