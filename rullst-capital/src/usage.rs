use crate::CapitalError;
use async_trait::async_trait;
use ring::digest::{SHA256, digest};
use serde_json::Value;

/// Largest quantity accepted by one usage event.
pub const MAX_USAGE_QUANTITY: u64 = i64::MAX as u64;

const MAX_PROVIDER_NAME_LEN: usize = 64;
const MAX_PROVIDER_REFERENCE_LEN: usize = 255;
const MAX_METRIC_LEN: usize = 100;
const MAX_EVENT_KEY_LEN: usize = 100;
const MAX_USAGE_RESPONSE_BYTES: usize = 1024 * 1024;
const STRIPE_MAX_PAST_SECONDS: i64 = 35 * 24 * 60 * 60;
const STRIPE_MAX_FUTURE_SECONDS: i64 = 5 * 60;

/// One event for Stripe's current Billing Meter Events API.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct StripeMeterEvent {
    customer_id: String,
    event_name: String,
    value: u64,
    occurred_at: i64,
    identifier: String,
}

impl StripeMeterEvent {
    /// Creates an event using the current UTC time.
    pub fn new(
        customer_id: impl Into<String>,
        event_name: impl Into<String>,
        value: u64,
        identifier: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let now = chrono::Utc::now().timestamp();
        Self::new_at(customer_id, event_name, value, now, identifier, now)
    }

    /// Creates an event against an explicit clock for delayed workers and tests.
    pub fn new_at(
        customer_id: impl Into<String>,
        event_name: impl Into<String>,
        value: u64,
        occurred_at: i64,
        identifier: impl Into<String>,
        now: i64,
    ) -> Result<Self, CapitalError> {
        let event = Self {
            customer_id: customer_id.into(),
            event_name: event_name.into(),
            value,
            occurred_at,
            identifier: identifier.into(),
        };
        event.validate_at(now)?;
        Ok(event)
    }

    /// Stripe customer receiving the metered usage.
    pub fn customer_id(&self) -> &str {
        &self.customer_id
    }

    /// Event name configured on the Stripe meter.
    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    /// Positive whole-number usage value.
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Unix timestamp attributed to the event.
    pub fn occurred_at(&self) -> i64 {
        self.occurred_at
    }

    /// Provider-forwarded identifier used for Stripe's rolling deduplication window.
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// Revalidates the provider timestamp window against a caller-supplied clock.
    pub fn validate_at(&self, now: i64) -> Result<(), CapitalError> {
        validate_usage_reference("Stripe customer ID", &self.customer_id, 255)?;
        if !self.customer_id.starts_with("cus_") {
            return Err(invalid_usage("Stripe customer ID must start with `cus_`"));
        }
        validate_metric("Stripe event name", &self.event_name)?;
        validate_quantity(self.value)?;
        validate_event_key("Stripe identifier", &self.identifier)?;
        let earliest = now.saturating_sub(STRIPE_MAX_PAST_SECONDS);
        let latest = now.saturating_add(STRIPE_MAX_FUTURE_SECONDS);
        if !(earliest..=latest).contains(&self.occurred_at) {
            return Err(invalid_usage(
                "Stripe event timestamp must be within 35 days in the past and 5 minutes in the future",
            ));
        }
        Ok(())
    }
}

impl std::fmt::Debug for StripeMeterEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StripeMeterEvent")
            .field("customer_id", &"[REDACTED]")
            .field("event_name", &self.event_name)
            .field("value", &self.value)
            .field("occurred_at", &self.occurred_at)
            .field("identifier", &"[REDACTED]")
            .finish()
    }
}

/// Aggregation action configured for one Lemon Squeezy usage record.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LemonSqueezyUsageAction {
    /// Adds the quantity to the billing-period total.
    Increment,
    /// Replaces the billing-period total with the quantity.
    Set,
}

impl LemonSqueezyUsageAction {
    /// Exact action token accepted by Lemon Squeezy.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Increment => "increment",
            Self::Set => "set",
        }
    }
}

/// One provider-specific Lemon Squeezy usage record.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct LemonSqueezyUsageRecord {
    subscription_item_id: String,
    application_metric: String,
    quantity: u64,
    action: LemonSqueezyUsageAction,
    event_key: String,
}

impl LemonSqueezyUsageRecord {
    /// Creates a record with an application-owned durable deduplication key.
    pub fn new(
        subscription_item_id: impl Into<String>,
        application_metric: impl Into<String>,
        quantity: u64,
        action: LemonSqueezyUsageAction,
        event_key: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let record = Self {
            subscription_item_id: subscription_item_id.into(),
            application_metric: application_metric.into(),
            quantity,
            action,
            event_key: event_key.into(),
        };
        record.validate()?;
        Ok(record)
    }

    /// Lemon Squeezy subscription-item relationship identifier.
    pub fn subscription_item_id(&self) -> &str {
        &self.subscription_item_id
    }

    /// Application-side metric label used for audit and mock identity.
    pub fn application_metric(&self) -> &str {
        &self.application_metric
    }

    /// Positive whole-number usage quantity.
    pub fn quantity(&self) -> u64 {
        self.quantity
    }

    /// Aggregation action that must match the provider-side meter configuration.
    pub fn action(&self) -> LemonSqueezyUsageAction {
        self.action
    }

    /// Stable key the application must claim durably before live submission.
    pub fn event_key(&self) -> &str {
        &self.event_key
    }

    pub(crate) fn validate(&self) -> Result<(), CapitalError> {
        if self.subscription_item_id.is_empty()
            || self.subscription_item_id.len() > 64
            || !self
                .subscription_item_id
                .bytes()
                .all(|byte| byte.is_ascii_digit())
        {
            return Err(invalid_usage(
                "Lemon Squeezy subscription-item ID must contain 1 to 64 ASCII digits",
            ));
        }
        validate_metric("application metric", &self.application_metric)?;
        validate_quantity(self.quantity)?;
        validate_event_key("application event key", &self.event_key)
    }
}

impl std::fmt::Debug for LemonSqueezyUsageRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LemonSqueezyUsageRecord")
            .field("subscription_item_id", &"[REDACTED]")
            .field("application_metric", &self.application_metric)
            .field("quantity", &self.quantity)
            .field("action", &self.action)
            .field("event_key", &"[REDACTED]")
            .finish()
    }
}

/// Provider-reported state of a usage submission.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageStatus {
    /// A live provider accepted and structurally echoed the event.
    Accepted,
    /// Deterministic offline fixture; no billable usage was recorded.
    Mock,
}

/// Deduplication boundary available for an accepted usage submission.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageDeduplication {
    /// Stripe received the identifier used by its bounded rolling window.
    ProviderRollingWindow,
    /// The provider exposes no equivalent key; a durable application outbox is required.
    ApplicationOutboxRequired,
    /// Deterministic offline fixture only.
    Mock,
}

/// Provider-bound result of a live or explicitly mocked usage submission.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub struct UsageReceipt {
    provider: &'static str,
    record_id: String,
    event_key: String,
    quantity: u64,
    status: UsageStatus,
    deduplication: UsageDeduplication,
}

impl UsageReceipt {
    /// Constructs a receipt from a response already authenticated and bound by an adapter.
    ///
    /// Custom [`MeteredBillingProvider`] implementations call this only after
    /// checking that the provider response represents the submitted request.
    /// This constructor validates structure; it does not authenticate a provider.
    pub fn from_verified_provider_response(
        provider: &'static str,
        record_id: impl Into<String>,
        event_key: impl Into<String>,
        quantity: u64,
        status: UsageStatus,
        deduplication: UsageDeduplication,
    ) -> Result<Self, CapitalError> {
        validate_usage_reference("provider name", provider, MAX_PROVIDER_NAME_LEN)?;
        let record_id = record_id.into();
        validate_usage_reference(
            "provider usage-record ID",
            &record_id,
            MAX_PROVIDER_REFERENCE_LEN,
        )?;
        let event_key = event_key.into();
        validate_event_key("usage event key", &event_key)?;
        validate_quantity(quantity)?;
        let semantics_match = matches!(
            (status, deduplication),
            (UsageStatus::Mock, UsageDeduplication::Mock)
                | (
                    UsageStatus::Accepted,
                    UsageDeduplication::ProviderRollingWindow
                        | UsageDeduplication::ApplicationOutboxRequired
                )
        );
        if !semantics_match {
            return Err(CapitalError::ProviderRequestFailed(
                "provider returned inconsistent usage status and deduplication evidence"
                    .to_string(),
            ));
        }
        Ok(Self {
            provider,
            record_id,
            event_key,
            quantity,
            status,
            deduplication,
        })
    }

    /// Name of the adapter that processed the request.
    pub fn provider(&self) -> &'static str {
        self.provider
    }

    /// Provider-returned record or meter-event identifier.
    pub fn record_id(&self) -> &str {
        &self.record_id
    }

    /// Stable provider or application deduplication identity.
    pub fn event_key(&self) -> &str {
        &self.event_key
    }

    /// Usage quantity bound to the response.
    pub fn quantity(&self) -> u64 {
        self.quantity
    }

    /// Whether this receipt represents a live acceptance or an offline fixture.
    pub fn status(&self) -> UsageStatus {
        self.status
    }

    /// Deduplication guarantee available for retries.
    pub fn deduplication(&self) -> UsageDeduplication {
        self.deduplication
    }

    /// Returns true only for an acceptance reported by a live provider.
    pub fn is_live_accepted(&self) -> bool {
        self.status == UsageStatus::Accepted
    }
}

impl std::fmt::Debug for UsageReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UsageReceipt")
            .field("provider", &self.provider)
            .field("record_id", &"[REDACTED]")
            .field("event_key", &"[REDACTED]")
            .field("quantity", &self.quantity)
            .field("status", &self.status)
            .field("deduplication", &self.deduplication)
            .finish()
    }
}

/// Static-dispatch boundary for provider-specific metered-usage contracts.
#[async_trait]
pub trait MeteredBillingProvider: Send + Sync {
    /// Request shape required by this provider's current reviewed API.
    type UsageRequest: Send + Sync;

    /// Reports one validated usage event and binds the provider response to it.
    async fn report_metered_usage(
        &self,
        request: &Self::UsageRequest,
    ) -> Result<UsageReceipt, CapitalError>;
}

pub(crate) fn mock_usage_receipt(
    provider: &'static str,
    event_key: &str,
    quantity: u64,
    identity_parts: &[&str],
) -> Result<UsageReceipt, CapitalError> {
    let mut material = Vec::with_capacity(512);
    material.extend_from_slice(provider.as_bytes());
    material.push(0);
    for part in identity_parts {
        material.extend_from_slice(part.as_bytes());
        material.push(0);
    }
    material.extend_from_slice(&quantity.to_be_bytes());
    let encoded = hex::encode(digest(&SHA256, &material));
    let fingerprint = encoded.get(..24).ok_or_else(|| {
        CapitalError::ProviderRequestFailed(
            "failed to construct deterministic mock usage ID".to_string(),
        )
    })?;
    UsageReceipt::from_verified_provider_response(
        provider,
        format!("usage_mock_{fingerprint}"),
        event_key,
        quantity,
        UsageStatus::Mock,
        UsageDeduplication::Mock,
    )
}

pub(crate) async fn read_bounded_usage_json(
    mut response: reqwest::Response,
) -> Result<Value, CapitalError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        CapitalError::ProviderRequestFailed("failed to read provider response".to_string())
    })? {
        if body.len().saturating_add(chunk.len()) > MAX_USAGE_RESPONSE_BYTES {
            return Err(CapitalError::ProviderRequestFailed(
                "provider usage response exceeded 1 MiB".to_string(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| {
        CapitalError::PayloadParseError("provider returned malformed usage JSON".to_string())
    })
}

fn validate_quantity(quantity: u64) -> Result<(), CapitalError> {
    if quantity == 0 || quantity > MAX_USAGE_QUANTITY {
        return Err(invalid_usage(format!(
            "quantity must contain 1 to {MAX_USAGE_QUANTITY} whole units"
        )));
    }
    Ok(())
}

fn validate_metric(label: &str, value: &str) -> Result<(), CapitalError> {
    validate_usage_reference(label, value, MAX_METRIC_LEN)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(invalid_usage(format!(
            "{label} must contain only ASCII letters, digits, `.`, `_`, or `-`"
        )));
    }
    Ok(())
}

fn validate_event_key(label: &str, value: &str) -> Result<(), CapitalError> {
    validate_usage_reference(label, value, MAX_EVENT_KEY_LEN)?;
    if !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(invalid_usage(format!(
            "{label} must contain only visible ASCII characters"
        )));
    }
    Ok(())
}

fn validate_usage_reference(label: &str, value: &str, max_len: usize) -> Result<(), CapitalError> {
    if value.is_empty()
        || value.len() > max_len
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(invalid_usage(format!(
            "{label} must contain 1 to {max_len} non-control characters without surrounding whitespace"
        )));
    }
    Ok(())
}

fn invalid_usage(message: impl Into<String>) -> CapitalError {
    CapitalError::InvalidUsage(message.into())
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
