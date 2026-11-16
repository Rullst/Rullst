//! Alert rules per project, evaluated on each poll with an explicit clock.
//!
//! Messages are Studio's own wording with the project name and numbers; no
//! label from the remote document reaches them, so they are safe to send to
//! a webhook.

use super::history::Sample;
use serde::{Deserialize, Serialize};

/// The 5xx-rate rule needs at least this many requests in the window.
pub(crate) const MIN_REQUESTS_FOR_ERROR_RATE: u64 = 20;
const DEFAULT_COOLDOWN_MINUTES: u32 = 15;
const MAX_COOLDOWN_MINUTES: u32 = 1_440;
const MAX_LATENCY_MS: f64 = 600_000.0;
const MAX_QUEUE_THRESHOLD: u64 = 1_000_000_000;

/// The thresholds of one project; `None` disables a rule.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct AlertRules {
    /// Fire when the project cannot be read.
    #[serde(default)]
    pub unreachable: bool,
    /// Fire when the window's 5xx rate reaches this percentage.
    #[serde(default)]
    pub error_rate_percent: Option<f64>,
    /// Fire when the window's p95 latency reaches this many milliseconds.
    #[serde(default)]
    pub p95_ms: Option<f64>,
    /// Fire when the window's p99 latency reaches this many milliseconds
    /// (since 13.0.0-alpha.6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p99_ms: Option<f64>,
    /// Fire when the pending jobs of all queues reach this count.
    #[serde(default)]
    pub queue_pending: Option<u64>,
    /// Fire when this many queue failures happen between two polls.
    #[serde(default)]
    pub queue_failures: Option<u64>,
    /// Fire when the application records this many security events between
    /// two polls (a `rullst.monitor.v2` document is needed; since
    /// 13.0.0-alpha.6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_events: Option<u64>,
    /// Minutes before the same rule fires again for the project.
    #[serde(default = "default_cooldown")]
    pub cooldown_minutes: u32,
}

fn default_cooldown() -> u32 {
    DEFAULT_COOLDOWN_MINUTES
}

impl Default for AlertRules {
    fn default() -> Self {
        Self {
            unreachable: false,
            error_rate_percent: None,
            p95_ms: None,
            p99_ms: None,
            queue_pending: None,
            queue_failures: None,
            security_events: None,
            cooldown_minutes: DEFAULT_COOLDOWN_MINUTES,
        }
    }
}

impl AlertRules {
    /// Whether every threshold is within its range.
    pub(crate) fn valid(&self) -> bool {
        let percent = self
            .error_rate_percent
            .is_none_or(|value| value.is_finite() && value > 0.0 && value <= 100.0);
        let latency = |value: Option<f64>| {
            value.is_none_or(|value| value.is_finite() && value > 0.0 && value <= MAX_LATENCY_MS)
        };
        let count = |value: Option<u64>| {
            value.is_none_or(|value| (1..=MAX_QUEUE_THRESHOLD).contains(&value))
        };
        percent
            && latency(self.p95_ms)
            && latency(self.p99_ms)
            && count(self.queue_pending)
            && count(self.queue_failures)
            && count(self.security_events)
            && (1..=MAX_COOLDOWN_MINUTES).contains(&self.cooldown_minutes)
    }
}

/// The rules an alert can come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuleKind {
    Unreachable,
    ErrorRate,
    P95,
    P99,
    QueuePending,
    QueueFailures,
    SecurityEvents,
}

impl RuleKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Unreachable => "unreachable",
            Self::ErrorRate => "error_rate",
            Self::P95 => "p95_latency",
            Self::P99 => "p99_latency",
            Self::QueuePending => "queue_pending",
            Self::QueueFailures => "queue_failures",
            Self::SecurityEvents => "security_events",
        }
    }
}

/// One rule that fired.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Firing {
    pub(crate) rule: RuleKind,
    pub(crate) message: String,
}

/// The rules of `project` that fire for `current`, given the `previous`
/// sample and when each rule last fired (`last_fired`), at time `now`.
pub(crate) fn evaluate(
    project: &str,
    rules: &AlertRules,
    current: &Sample,
    previous: Option<&Sample>,
    now: u64,
    last_fired: impl Fn(RuleKind) -> Option<u64>,
) -> Vec<Firing> {
    let cooldown = u64::from(rules.cooldown_minutes).saturating_mul(60);
    let mut firings = Vec::new();
    let mut fire = |rule: RuleKind, message: String| {
        let cooled = last_fired(rule).is_none_or(|at| now.saturating_sub(at) >= cooldown);
        if cooled {
            firings.push(Firing { rule, message });
        }
    };
    if !current.reachable {
        if rules.unreachable {
            fire(
                RuleKind::Unreachable,
                format!("{project} could not be read"),
            );
        }
        return firings;
    }
    if let (Some(threshold), Some(rate)) = (rules.error_rate_percent, current.error_rate_percent)
        && current.window_requests.unwrap_or(0) >= MIN_REQUESTS_FOR_ERROR_RATE
        && rate >= threshold
    {
        fire(
            RuleKind::ErrorRate,
            format!("{project}: 5xx rate {rate:.2}% (threshold {threshold}%)"),
        );
    }
    if let (Some(threshold), Some(p95)) = (rules.p95_ms, current.p95_ms)
        && p95 >= threshold
    {
        fire(
            RuleKind::P95,
            format!("{project}: p95 latency {p95:.1} ms (threshold {threshold} ms)"),
        );
    }
    if let (Some(threshold), Some(p99)) = (rules.p99_ms, current.p99_ms)
        && p99 >= threshold
    {
        fire(
            RuleKind::P99,
            format!("{project}: p99 latency {p99:.1} ms (threshold {threshold} ms)"),
        );
    }
    if let (Some(threshold), Some(pending)) = (rules.queue_pending, current.queue_pending)
        && pending >= threshold
    {
        fire(
            RuleKind::QueuePending,
            format!("{project}: {pending} pending jobs (threshold {threshold})"),
        );
    }
    let failures = previous
        .and_then(|previous| previous.queue_failed_total)
        .zip(current.queue_failed_total)
        // A restart resets the counter; never treat that as failures.
        .and_then(|(before, after)| after.checked_sub(before));
    if let (Some(threshold), Some(failures)) = (rules.queue_failures, failures)
        && failures >= threshold
    {
        fire(
            RuleKind::QueueFailures,
            format!(
                "{project}: {failures} queue failures since the previous poll (threshold {threshold})"
            ),
        );
    }
    let events = previous
        .and_then(|previous| previous.security_events_total)
        .zip(current.security_events_total)
        .and_then(|(before, after)| after.checked_sub(before));
    if let (Some(threshold), Some(events)) = (rules.security_events, events)
        && events >= threshold
    {
        fire(
            RuleKind::SecurityEvents,
            format!(
                "{project}: {events} security events since the previous poll (threshold {threshold})"
            ),
        );
    }
    firings
}
