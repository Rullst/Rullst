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
#[non_exhaustive]
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

/// The grammar of a [`RecurringDefinition`]'s cron expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum CronSyntax {
    /// POSIX five-field cron, read exactly as Core's `Scheduler::task` reads
    /// it: weekdays 0-7 (0 and 7 are Sunday), a day matching either day
    /// field when both are restricted, each field valid on its own. Every
    /// definition built with [`RecurringDefinition::new`] uses it, since
    /// 13.0.0-alpha.6.
    Posix,
    /// The grammar of the `cron` crate 0.17, which evaluated every schedule
    /// until 13.0.0-alpha.5: weekdays 1 (Sunday) to 7 (Saturday), both day
    /// fields must match, and tokens may join or split. Stored definitions
    /// without a syntax marker keep it and their occurrences. Deprecated:
    /// removal is planned for 14.0.
    Legacy,
}

/// The message of the [`RecurringError::InvalidInput`] for an expression that
/// only the legacy grammar accepts.
pub(super) const LEGACY_ONLY: &str =
    "POSIX five-field UTC cron; this expression uses the legacy syntax";

/// Immutable UTC schedule. Host authorization precedes creation/cancellation.
///
/// The expression is POSIX five-field cron (`minute hour day-of-month month
/// day-of-week`), read in UTC exactly as Core's `Scheduler::task` reads it:
/// numbers, `*`, ranges, lists and steps, month and weekday names, `?` in
/// the day fields, weekdays 0-7 (0 and 7 are Sunday), and a day matching
/// *either* day field when both are restricted (neither starts with `*` or
/// `?`), otherwise both. Each field must be valid on its own. Occurrences are
/// whole minutes from 1970 through 2100.
///
/// Definitions stored before 13.0.0-alpha.6 have no syntax marker and keep
/// the [`CronSyntax::Legacy`] grammar of the `cron` crate, so their
/// occurrences never change; [`Self::syntax`] tells them apart.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredDefinition")]
pub struct RecurringDefinition {
    pub(super) name: String,
    pub(super) expression: String,
    pub(super) first_after: i64,
    pub(super) missed: MissedRunPolicy,
    pub(super) message: ScheduledMessage,
    #[serde(skip_serializing_if = "is_legacy")]
    pub(super) syntax: CronSyntax,
}
impl RecurringDefinition {
    /// A POSIX definition; see [`RecurringDefinition`] for the grammar.
    ///
    /// An expression only the legacy grammar accepts, such as `1, 2 * * *`,
    /// fails with `InvalidInput("POSIX five-field UTC cron; this expression
    /// uses the legacy syntax")`; any other invalid expression with
    /// `InvalidInput("five-field UTC cron")`.
    pub fn new(
        name: impl Into<String>,
        expression: impl Into<String>,
        first_after_ms: i64,
        missed: MissedRunPolicy,
        message: ScheduledMessage,
    ) -> Result<Self> {
        Self::build(
            name.into(),
            expression.into(),
            first_after_ms,
            missed,
            message,
            CronSyntax::Posix,
        )
    }
    fn build(
        name: String,
        expression: String,
        first_after: i64,
        missed: MissedRunPolicy,
        message: ScheduledMessage,
        syntax: CronSyntax,
    ) -> Result<Self> {
        let value = Self {
            name,
            expression: expression.split_whitespace().collect::<Vec<_>>().join(" "),
            first_after,
            missed,
            message,
            syntax,
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
    /// The grammar of [`Self::cron_expression`]: [`CronSyntax::Posix`] for a
    /// definition built with [`Self::new`], [`CronSyntax::Legacy`] for one
    /// stored before 13.0.0-alpha.6. Since 13.0.0-alpha.6.
    pub fn syntax(&self) -> CronSyntax {
        self.syntax
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
        const FIVE_FIELDS: RecurringError = RecurringError::InvalidInput("five-field UTC cron");
        if self.expression.len() > 128 || self.expression.split_whitespace().count() != 5 {
            return Err(FIVE_FIELDS);
        }
        let legacy = || cron_expression::Schedule::parse_five_fields(&self.expression);
        let next = match self.syntax {
            CronSyntax::Posix => posix_cron::PosixSchedule::parse(&self.expression)
                .map_err(|_| match legacy() {
                    Some(_) => RecurringError::InvalidInput(LEGACY_ONLY),
                    None => FIVE_FIELDS,
                })?
                .next_after(after),
            CronSyntax::Legacy => legacy()
                .ok_or(FIVE_FIELDS)?
                .next_after(after)
                .map_err(|_| RecurringError::InvalidInput("cron timestamp"))?,
        };
        Ok(next.filter(|value| *value <= MAX_TIMESTAMP))
    }
    /// What `create` does when `self` is stored under the requested
    /// definition's name: `Ok(false)` replays an equal definition and
    /// `Ok(true)` replays a legacy one that differs only in its syntax,
    /// which keeps its legacy meaning.
    pub(super) fn replays(&self, requested: &Self) -> Result<bool> {
        if self == requested {
            return Ok(false);
        }
        match self.syntax {
            CronSyntax::Legacy if self.differs_only_in_syntax(requested) => Ok(true),
            CronSyntax::Legacy => Err(RecurringError::LegacyDefinition),
            _ => Err(RecurringError::Conflict),
        }
    }
    fn differs_only_in_syntax(&self, other: &Self) -> bool {
        self.name == other.name
            && self.expression == other.expression
            && self.first_after == other.first_after
            && self.missed == other.missed
            && self.message == other.message
    }
}

fn is_legacy(syntax: &CronSyntax) -> bool {
    *syntax == CronSyntax::Legacy
}

fn legacy() -> CronSyntax {
    CronSyntax::Legacy
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
    /// Absent in definitions stored before 13.0.0-alpha.6.
    #[serde(default = "legacy")]
    syntax: CronSyntax,
}
impl TryFrom<StoredDefinition> for RecurringDefinition {
    type Error = RecurringError;
    fn try_from(value: StoredDefinition) -> Result<Self> {
        Self::build(
            value.name,
            value.expression,
            value.first_after,
            value.missed,
            value.message,
            value.syntax,
        )
    }
}
