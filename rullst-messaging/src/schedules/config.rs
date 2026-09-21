use super::*;
use crate::{Namespace, PublishRequest};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::Duration};

#[derive(Clone)]
pub struct RecurringConfig {
    pub(super) namespace: Namespace,
    pub(super) schedules: usize,
    pub(super) occurrences: usize,
    pub(super) lease_ms: i64,
    pub(super) window_ms: i64,
}
impl RecurringConfig {
    /// Server configuration: up to 10,000 definitions and 100,000 occurrences.
    /// Leases last 60 seconds; automatic/manual delivery retry is limited to one day.
    pub fn new(namespace: impl Into<String>, schedules: usize, occurrences: usize) -> Result<Self> {
        let namespace =
            Namespace::try_new(namespace).map_err(|_| RecurringError::InvalidInput("namespace"))?;
        if !(1..=10_000).contains(&schedules) || !(1..=100_000).contains(&occurrences) {
            return Err(RecurringError::InvalidInput("quotas"));
        }
        Ok(Self {
            namespace,
            schedules,
            occurrences,
            lease_ms: 60_000,
            window_ms: 86_400_000,
        })
    }
    pub fn with_delivery_window(mut self, window: Duration) -> Result<Self> {
        let window = i64::try_from(window.as_millis())
            .map_err(|_| RecurringError::InvalidInput("delivery window"))?;
        if !(self.lease_ms * 2..=7 * 86_400_000).contains(&window) {
            return Err(RecurringError::InvalidInput("delivery window"));
        }
        self.window_ms = window;
        Ok(self)
    }
    pub fn with_lease(mut self, lease: Duration) -> Result<Self> {
        let lease =
            i64::try_from(lease.as_millis()).map_err(|_| RecurringError::InvalidInput("lease"))?;
        if !(1000..=300_000).contains(&lease) || lease * 2 > self.window_ms {
            return Err(RecurringError::InvalidInput("lease"));
        }
        self.lease_ms = lease;
        Ok(self)
    }
    pub fn namespace(&self) -> &str {
        self.namespace.as_str()
    }
    pub(super) fn binding(&self) -> Vec<u8> {
        format!(
            "recurring-v1\n{}\n{}\n{}\n{}\n{}",
            self.namespace.as_str(),
            self.schedules,
            self.occurrences,
            self.lease_ms,
            self.window_ms
        )
        .into_bytes()
    }
}
impl std::fmt::Debug for RecurringConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecurringConfig")
            .field("schedules", &self.schedules)
            .field("occurrences", &self.occurrences)
            .field("lease_ms", &self.lease_ms)
            .field("delivery_window_ms", &self.window_ms)
            .finish_non_exhaustive()
    }
}

/// UTC policy for occurrences missed while no instance was ticking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissedRunPolicy {
    /// Preserve each due time, producing at most the caller's bounded tick budget.
    CatchUp,
    /// Emit the oldest outstanding due time once, then advance after the current time.
    Coalesce,
}

/// Frozen, bounded broker message template. No payload appears in Debug.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredMessage")]
pub struct ScheduledMessage {
    topic: String,
    event_kind: String,
    content_type: String,
    headers: BTreeMap<String, String>,
    payload: Vec<u8>,
}
impl ScheduledMessage {
    pub fn new(
        topic: impl Into<String>,
        event_kind: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self> {
        let request = PublishRequest::try_new(topic, event_kind, "recurring-template", payload)
            .map_err(|_| RecurringError::InvalidInput("message"))?;
        Self::from_request(request)
    }
    fn from_request(request: PublishRequest) -> Result<Self> {
        request
            .validate_payload(MAX_PAYLOAD_BYTES)
            .map_err(|_| RecurringError::InvalidInput("payload size"))?;
        Ok(Self {
            topic: request.topic().as_str().to_owned(),
            event_kind: request.event_kind().as_str().to_owned(),
            content_type: request.content_type().as_str().to_owned(),
            headers: request
                .headers()
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
            payload: request.payload().to_vec(),
        })
    }
    pub fn with_content_type(self, content_type: impl Into<String>) -> Result<Self> {
        Self::from_request(
            self.request("recurring-template")?
                .with_content_type(content_type)
                .map_err(|_| RecurringError::InvalidInput("content type"))?,
        )
    }
    pub fn with_header(self, name: impl Into<String>, value: impl Into<String>) -> Result<Self> {
        Self::from_request(
            self.request("recurring-template")?
                .with_header(name, value)
                .map_err(|_| RecurringError::InvalidInput("message header"))?,
        )
    }
    pub(super) fn request(&self, key: &str) -> Result<PublishRequest> {
        let mut request =
            PublishRequest::try_new(&self.topic, &self.event_kind, key, self.payload.clone())
                .and_then(|r| r.with_content_type(&self.content_type))
                .map_err(|_| RecurringError::InvalidInput("message"))?;
        request
            .validate_payload(MAX_PAYLOAD_BYTES)
            .map_err(|_| RecurringError::InvalidInput("payload size"))?;
        for (name, value) in &self.headers {
            request = request
                .with_header(name, value)
                .map_err(|_| RecurringError::InvalidInput("message header"))?;
        }
        Ok(request)
    }
}
impl std::fmt::Debug for ScheduledMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScheduledMessage([REDACTED])")
    }
}

/// Immutable UTC schedule. Host authorization precedes creation/cancellation.
///
/// Uses the `cron` crate's five calendar fields (minute, hour, day, month,
/// weekday), with seconds fixed to zero and years 1970..=2100. Weekdays use
/// 1=Sunday through 7=Saturday or names. Day-of-month and weekday restrictions
/// intersect. This is not a POSIX/crontab parser: weekday 0 is rejected.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredDefinition")]
pub struct RecurringDefinition {
    pub(super) name: String,
    pub(super) expression: String,
    pub(super) first_after: i64,
    pub(super) missed: MissedRunPolicy,
    pub(super) message: ScheduledMessage,
}
impl RecurringDefinition {
    pub fn new(
        name: impl Into<String>,
        expression: impl Into<String>,
        first_after_ms: i64,
        missed: MissedRunPolicy,
        message: ScheduledMessage,
    ) -> Result<Self> {
        let value = Self {
            name: name.into(),
            expression: expression
                .into()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            first_after: first_after_ms,
            missed,
            message,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn cron_expression(&self) -> &str {
        &self.expression
    }
    pub fn missed_run_policy(&self) -> MissedRunPolicy {
        self.missed
    }
    pub fn first_after_ms(&self) -> i64 {
        self.first_after
    }
    pub(super) fn validate(&self) -> Result<()> {
        crate::validation::validate_route_identifier("schedule name", &self.name, 128)
            .map_err(|_| RecurringError::InvalidInput("schedule name"))?;
        if !(0..=MAX_TIMESTAMP).contains(&self.first_after) {
            return Err(RecurringError::InvalidInput("first-after time"));
        }
        self.next_after(self.first_after)?
            .ok_or(RecurringError::InvalidInput("exhausted cron"))?;
        self.message.request("recurring-template")?;
        Ok(())
    }
    pub(super) fn next_after(&self, after: i64) -> Result<Option<i64>> {
        if self.expression.len() > 128 || self.expression.split_whitespace().count() != 5 {
            return Err(RecurringError::InvalidInput("five-field UTC cron"));
        }
        let schedule: cron::Schedule = format!("0 {} *", self.expression)
            .parse()
            .map_err(|_| RecurringError::InvalidInput("five-field UTC cron"))?;
        let time = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(after)
            .ok_or(RecurringError::InvalidInput("cron timestamp"))?;
        Ok(schedule
            .after(&time)
            .next()
            .map(|next| next.timestamp_millis())
            .filter(|value| *value <= MAX_TIMESTAMP))
    }
}
impl std::fmt::Debug for RecurringDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecurringDefinition([REDACTED])")
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredMessage {
    topic: String,
    event_kind: String,
    content_type: String,
    headers: BTreeMap<String, String>,
    payload: Vec<u8>,
}
impl TryFrom<StoredMessage> for ScheduledMessage {
    type Error = RecurringError;
    fn try_from(value: StoredMessage) -> Result<Self> {
        let mut message = Self::new(value.topic, value.event_kind, value.payload)?
            .with_content_type(value.content_type)?;
        for (name, value) in value.headers {
            message = message.with_header(name, value)?;
        }
        Ok(message)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredDefinition {
    name: String,
    expression: String,
    first_after: i64,
    missed: MissedRunPolicy,
    message: ScheduledMessage,
}
impl TryFrom<StoredDefinition> for RecurringDefinition {
    type Error = RecurringError;
    fn try_from(value: StoredDefinition) -> Result<Self> {
        Self::new(
            value.name,
            value.expression,
            value.first_after,
            value.missed,
            value.message,
        )
    }
}
