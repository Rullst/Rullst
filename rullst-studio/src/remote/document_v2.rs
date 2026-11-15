//! The sections `rullst.monitor.v2` adds to the version 1 document: sampled
//! request traces and the security radar.
//!
//! They come from another machine, like the rest of the document: every list
//! and label is bounded by [`Traces::validate`] and [`Security::validate`],
//! and pages escape every string.

use serde::Deserialize;
use std::collections::BTreeMap;

/// The version 2 schema.
pub const MONITOR_V2_SCHEMA: &str = "rullst.monitor.v2";
const MAX_REQUESTS: usize = 64;
const MAX_SPAN_KINDS: usize = 8;
const MAX_COUNTERS: usize = 64;
const MAX_EVENTS: usize = 64;

/// Sampled summaries of recent requests and per-kind span totals.
#[derive(Debug, Clone, Deserialize)]
pub struct Traces {
    /// Most summaries the server keeps.
    pub sample_limit: u64,
    /// Summaries kept since the process started.
    pub sampled_total: u64,
    /// The kept summaries, newest first.
    pub requests: Vec<RequestTrace>,
    /// Totals of the spans the application recorded.
    pub spans: SpanSummary,
}

/// One sampled request.
#[derive(Debug, Clone, Deserialize)]
pub struct RequestTrace {
    /// Unix time, in seconds.
    pub at: u64,
    /// Normalized method.
    pub method: String,
    /// Route pattern.
    pub route: String,
    /// Response status.
    pub status: u16,
    /// Duration in milliseconds.
    pub duration_ms: f64,
    /// ORM operations the request started.
    pub db_operations: u64,
    /// Whether the request repeated an ORM operation (possible N+1).
    pub n_plus_one: bool,
}

/// Span totals; span names are never sent.
#[derive(Debug, Clone, Deserialize)]
pub struct SpanSummary {
    /// Spans in the application's buffer.
    pub kept: u64,
    /// Totals per kind.
    pub kinds: Vec<SpanKind>,
}

/// The totals of one span kind.
#[derive(Debug, Clone, Deserialize)]
pub struct SpanKind {
    /// `http`, `sql`, `ai`, `job` or `other`.
    pub kind: String,
    /// Spans of this kind.
    pub count: u64,
    /// Their total duration in microseconds.
    pub total_us: u64,
    /// The longest, in microseconds.
    pub max_us: u64,
    /// Unix time of the latest.
    pub last_at: u64,
}

/// The security radar.
#[derive(Debug, Clone, Deserialize)]
pub struct Security {
    /// Security events recorded since the process started.
    pub events_total: u64,
    /// Peers currently banned by a honeypot.
    pub active_bans: u64,
    /// The security store's counters, by name.
    pub counters: BTreeMap<String, u64>,
    /// The type and time of the kept events, newest first.
    pub events: Vec<SecurityEvent>,
}

/// One security event: its type and time only.
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityEvent {
    /// Uppercase identifier, such as `RASP_PAYLOAD_INTERCEPTED`.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Unix time, in seconds.
    pub at: u64,
}

impl Traces {
    /// Checks the bounds and adds the labels to `labels`.
    pub(crate) fn validate<'a>(&'a self, labels: &mut Vec<&'a str>) -> Result<(), &'static str> {
        if self.requests.len() > MAX_REQUESTS || self.spans.kinds.len() > MAX_SPAN_KINDS {
            return Err("too many entries");
        }
        for request in &self.requests {
            labels.extend([request.method.as_str(), request.route.as_str()]);
        }
        labels.extend(self.spans.kinds.iter().map(|kind| kind.kind.as_str()));
        Ok(())
    }
}

impl Security {
    /// Checks the bounds and adds the labels to `labels`.
    pub(crate) fn validate<'a>(&'a self, labels: &mut Vec<&'a str>) -> Result<(), &'static str> {
        if self.counters.len() > MAX_COUNTERS || self.events.len() > MAX_EVENTS {
            return Err("too many entries");
        }
        labels.extend(self.counters.keys().map(String::as_str));
        labels.extend(self.events.iter().map(|event| event.event_type.as_str()));
        Ok(())
    }
}
