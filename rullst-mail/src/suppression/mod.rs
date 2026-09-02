//! Provider-neutral recipient suppression with bounded replay evidence.

use crate::drivers::MailDriver;
use crate::{DeliveryPipeline, MailError, Message, validate_email_syntax};
use async_trait::async_trait;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

mod memory;
pub use memory::InMemorySuppressionStore;
#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteSuppressionStore;

const MAX_STORE_ENTRIES: usize = 1_000_000;
const MAX_PROVIDER_BYTES: usize = 64;
const MAX_EVENT_ID_BYTES: usize = 128;
const MAX_EMAIL_BYTES: usize = 320;
const MAX_FUTURE_SKEW_SECONDS: u64 = 300;

/// Why future delivery to one recipient must be blocked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SuppressionReason {
    /// An authorized operator or application policy suppressed the recipient.
    Manual,
    /// A provider reported a permanent delivery failure.
    HardBounce,
    /// A provider reported that the recipient marked mail as spam.
    SpamComplaint,
}

impl SuppressionReason {
    /// Stable low-cardinality label suitable for persistence and telemetry.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::HardBounce => "hard_bounce",
            Self::SpamComplaint => "spam_complaint",
        }
    }

    pub(crate) const fn rank(self) -> i64 {
        match self {
            Self::Manual => 1,
            Self::HardBounce => 2,
            Self::SpamComplaint => 3,
        }
    }

    #[cfg(feature = "sqlite")]
    pub(crate) fn from_rank(rank: i64) -> Result<Self, SuppressionError> {
        match rank {
            1 => Ok(Self::Manual),
            2 => Ok(Self::HardBounce),
            3 => Ok(Self::SpamComplaint),
            _ => Err(SuppressionError::CorruptStorage("suppression reason")),
        }
    }
}

/// A verified provider event or explicit application decision.
#[derive(Clone, Eq, PartialEq)]
pub struct SuppressionEvent {
    provider: String,
    event_id: String,
    recipient: String,
    reason: SuppressionReason,
    observed_at: u64,
}

impl SuppressionEvent {
    /// Creates a bounded event after validating its stable identifiers and recipient.
    pub fn try_new(
        provider: impl Into<String>,
        event_id: impl Into<String>,
        recipient: impl Into<String>,
        reason: SuppressionReason,
        observed_at: u64,
    ) -> Result<Self, SuppressionError> {
        let provider = provider.into();
        let event_id = event_id.into();
        validate_identifier(&provider, MAX_PROVIDER_BYTES, "provider")?;
        validate_identifier(&event_id, MAX_EVENT_ID_BYTES, "event ID")?;
        let recipient = normalize_recipient(&recipient.into())?;
        let now = unix_time()?;
        if observed_at == 0 || observed_at > now.saturating_add(MAX_FUTURE_SKEW_SECONDS) {
            return Err(SuppressionError::InvalidEvent("observation time"));
        }
        Ok(Self {
            provider,
            event_id,
            recipient,
            reason,
            observed_at,
        })
    }

    /// Returns the validated provider identifier.
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Returns the validated provider-scoped event identifier.
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    /// Returns the normalized recipient address.
    pub fn recipient(&self) -> &str {
        &self.recipient
    }

    /// Returns the suppression reason.
    pub const fn reason(&self) -> SuppressionReason {
        self.reason
    }

    /// Returns the verified event timestamp as Unix seconds.
    pub const fn observed_at(&self) -> u64 {
        self.observed_at
    }
}

impl fmt::Debug for SuppressionEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SuppressionEvent")
            .field("provider", &self.provider)
            .field("event_id", &"[REDACTED]")
            .field("recipient", &"[REDACTED]")
            .field("reason", &self.reason)
            .field("observed_at", &self.observed_at)
            .finish()
    }
}

/// Current authoritative suppression state for one recipient.
#[derive(Clone, Eq, PartialEq)]
pub struct SuppressionRecord {
    recipient: String,
    reason: SuppressionReason,
    provider: String,
    first_seen_at: u64,
    last_seen_at: u64,
}

impl SuppressionRecord {
    /// Returns the normalized recipient address.
    pub fn recipient(&self) -> &str {
        &self.recipient
    }

    /// Returns the strongest recorded suppression reason.
    pub const fn reason(&self) -> SuppressionReason {
        self.reason
    }

    /// Returns the provider associated with the authoritative reason.
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Returns the first observed event timestamp.
    pub const fn first_seen_at(&self) -> u64 {
        self.first_seen_at
    }

    /// Returns the last observed event timestamp.
    pub const fn last_seen_at(&self) -> u64 {
        self.last_seen_at
    }
}

impl fmt::Debug for SuppressionRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SuppressionRecord")
            .field("recipient", &"[REDACTED]")
            .field("reason", &self.reason)
            .field("provider", &self.provider)
            .field("first_seen_at", &self.first_seen_at)
            .field("last_seen_at", &self.last_seen_at)
            .finish()
    }
}

/// Typed failures which never include recipient addresses or event identifiers.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SuppressionError {
    InvalidConfiguration(&'static str),
    InvalidEvent(&'static str),
    EventConflict,
    CapacityExceeded,
    StorageUnavailable(&'static str),
    CorruptStorage(&'static str),
}

impl fmt::Display for SuppressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(context) => {
                write!(formatter, "invalid suppression configuration: {context}")
            }
            Self::InvalidEvent(context) => {
                write!(formatter, "invalid suppression event: {context}")
            }
            Self::EventConflict => {
                formatter.write_str("provider event identifier was reused with different contents")
            }
            Self::CapacityExceeded => {
                formatter.write_str("suppression store capacity is exhausted")
            }
            Self::StorageUnavailable(operation) => {
                write!(
                    formatter,
                    "suppression storage operation failed: {operation}"
                )
            }
            Self::CorruptStorage(context) => {
                write!(formatter, "suppression storage is corrupt: {context}")
            }
        }
    }
}

impl std::error::Error for SuppressionError {}

/// Bounded counts for one suppression store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SuppressionSnapshot {
    recipients: usize,
    events: usize,
    max_recipients: usize,
    max_events: usize,
}

impl SuppressionSnapshot {
    pub(crate) const fn new(
        recipients: usize,
        events: usize,
        max_recipients: usize,
        max_events: usize,
    ) -> Self {
        Self {
            recipients,
            events,
            max_recipients,
            max_events,
        }
    }

    /// Returns the number of suppressed recipients.
    pub const fn recipients(self) -> usize {
        self.recipients
    }

    /// Returns the number of retained replay identifiers.
    pub const fn events(self) -> usize {
        self.events
    }

    /// Returns the configured recipient quota.
    pub const fn max_recipients(self) -> usize {
        self.max_recipients
    }

    /// Returns the configured replay-evidence quota.
    pub const fn max_events(self) -> usize {
        self.max_events
    }
}

/// Read contract used before transport dispatch.
pub trait SuppressionStore: Send + Sync {
    /// Returns the current suppression state, if any.
    fn lookup(
        &self,
        recipient: &str,
    ) -> impl std::future::Future<Output = Result<Option<SuppressionRecord>, SuppressionError>> + Send;
}

/// Mutable event-ingestion contract for verified provider adapters.
pub trait MutableSuppressionStore: SuppressionStore {
    /// Idempotently records one verified event.
    fn record(
        &self,
        event: SuppressionEvent,
    ) -> impl std::future::Future<Output = Result<SuppressionRecord, SuppressionError>> + Send;
}

/// Driver wrapper which checks suppression state before every official dispatch.
pub struct SuppressionGuard<D, S> {
    driver: D,
    store: S,
}

impl<D, S> SuppressionGuard<D, S> {
    /// Wraps a driver and a suppression store using static dispatch.
    pub const fn new(driver: D, store: S) -> Self {
        Self { driver, store }
    }

    /// Returns the wrapped driver.
    pub const fn driver(&self) -> &D {
        &self.driver
    }

    /// Returns the suppression store.
    pub const fn store(&self) -> &S {
        &self.store
    }

    async fn enforce(&self, recipient: &str) -> Result<(), MailError>
    where
        S: SuppressionStore,
    {
        let record = self
            .store
            .lookup(recipient)
            .await
            .map_err(|_| MailError::SuppressionUnavailable)?;
        if let Some(record) = record {
            return Err(MailError::SuppressedRecipient {
                reason: record.reason().as_str(),
            });
        }
        Ok(())
    }
}

#[async_trait]
impl<D, S> MailDriver for SuppressionGuard<D, S>
where
    D: MailDriver,
    S: SuppressionStore,
{
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare(message)?;
        self.enforce(&prepared.message().to).await?;
        self.driver.send(prepared.message()).await
    }

    async fn send_for_tenant(&self, tenant_id: &str, message: &Message) -> Result<(), MailError> {
        let prepared = DeliveryPipeline::prepare_for_tenant(tenant_id, message)?;
        self.enforce(&prepared.message().to).await?;
        self.driver
            .send_for_tenant(tenant_id, prepared.message())
            .await
    }
}

pub(crate) fn normalize_recipient(recipient: &str) -> Result<String, SuppressionError> {
    if recipient.trim() != recipient
        || recipient.is_empty()
        || recipient.len() > MAX_EMAIL_BYTES
        || !recipient.is_ascii()
        || recipient.chars().any(char::is_control)
        || validate_email_syntax(recipient).is_err()
    {
        return Err(SuppressionError::InvalidEvent("recipient"));
    }
    let (local, domain) = recipient
        .rsplit_once('@')
        .ok_or(SuppressionError::InvalidEvent("recipient"))?;
    Ok(format!("{local}@{}", domain.to_ascii_lowercase()))
}

pub(crate) fn validate_limits(
    max_recipients: usize,
    max_events: usize,
) -> Result<(), SuppressionError> {
    if !(1..=MAX_STORE_ENTRIES).contains(&max_recipients)
        || !(1..=MAX_STORE_ENTRIES).contains(&max_events)
    {
        return Err(SuppressionError::InvalidConfiguration("store limits"));
    }
    Ok(())
}

fn validate_identifier(
    value: &str,
    max_bytes: usize,
    field: &'static str,
) -> Result<(), SuppressionError> {
    if value.is_empty()
        || value.len() > max_bytes
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
    {
        return Err(SuppressionError::InvalidEvent(field));
    }
    Ok(())
}

pub(crate) fn unix_time() -> Result<u64, SuppressionError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SuppressionError::CorruptStorage("system time"))
        .map(|duration| duration.as_secs())
}

pub(crate) const fn unavailable(operation: &'static str) -> SuppressionError {
    SuppressionError::StorageUnavailable(operation)
}

#[cfg(test)]
mod tests;
