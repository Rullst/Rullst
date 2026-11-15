//! The `rullst.monitor.v2` document: every `rullst.monitor.v1` field, plus
//! `traces` and `security`.
//!
//! Nothing new identifies a person or carries free text: request summaries
//! use the same method and route labels as the statistics, span totals are
//! keyed by a fixed set of kinds (span names, which applications write freely
//! and may contain SQL or paths, are never sent), and security events keep
//! only their type and time (never their details or client address).
//! `payload_v2_tests.rs` pins the exact field set.

use super::MONITOR_V2_SCHEMA;
use super::collector::Collector;
use super::payload::Document;
use super::traces::{MAX_TRACES, RequestTrace};
use crate::security::{LiveSecurityEvent, SecurityStore, TelemetrySnapshot};
use crate::telemetry_spans::TraceSpan;
use serde::Serialize;

/// Span kinds reported by name; any other kind is counted as `other`.
const SPAN_KINDS: [&str; 4] = ["http", "sql", "ai", "job"];
const OTHER_SPAN_KIND: &str = "other";

#[derive(Debug, Serialize)]
pub(super) struct DocumentV2 {
    #[serde(flatten)]
    pub(super) base: Document,
    pub(super) traces: Traces,
    pub(super) security: Security,
}

#[derive(Debug, Serialize)]
pub(super) struct Traces {
    pub(super) sample_limit: usize,
    pub(super) sampled_total: u64,
    pub(super) requests: Vec<RequestTraceReport>,
    pub(super) spans: SpanSummary,
}

#[derive(Debug, Serialize)]
pub(super) struct RequestTraceReport {
    pub(super) at: u64,
    pub(super) method: &'static str,
    pub(super) route: String,
    pub(super) status: u16,
    pub(super) duration_ms: f64,
    pub(super) db_operations: u64,
    pub(super) n_plus_one: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct SpanSummary {
    pub(super) kept: usize,
    pub(super) kinds: Vec<SpanKindReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct SpanKindReport {
    pub(super) kind: &'static str,
    pub(super) count: u64,
    pub(super) total_us: u64,
    pub(super) max_us: u64,
    pub(super) last_at: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct Security {
    pub(super) events_total: u64,
    pub(super) active_bans: usize,
    pub(super) counters: TelemetrySnapshot,
    pub(super) events: Vec<SecurityEventReport>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct SecurityEventReport {
    #[serde(rename = "type")]
    pub(super) event_type: String,
    pub(super) at: u64,
}

impl DocumentV2 {
    /// Extends the version 1 `base` with the collector's request summaries,
    /// the totals of `spans` and the radar of `security`.
    pub(super) fn new(
        mut base: Document,
        collector: &Collector,
        spans: &[TraceSpan],
        security: &SecurityStore,
    ) -> Self {
        base.schema = MONITOR_V2_SCHEMA;
        let (sampled_total, requests) = collector.traces();
        Self {
            base,
            traces: Traces {
                sample_limit: MAX_TRACES,
                sampled_total,
                requests: requests.into_iter().map(trace_report).collect(),
                spans: span_summary(spans),
            },
            security: security_report(security),
        }
    }
}

fn trace_report(trace: RequestTrace) -> RequestTraceReport {
    let millis = trace.latency.as_secs_f64() * 1_000.0;
    RequestTraceReport {
        at: trace.at,
        method: trace.method,
        route: trace.route.to_string(),
        status: trace.status,
        duration_ms: (millis * 1_000.0).round() / 1_000.0,
        db_operations: trace.db_operations,
        n_plus_one: trace.n_plus_one,
    }
}

/// Count, total and longest duration and latest time per span kind, in the
/// order of [`SPAN_KINDS`] then `other`; kinds without spans are left out.
pub(super) fn span_summary(spans: &[TraceSpan]) -> SpanSummary {
    let mut kinds: Vec<SpanKindReport> = SPAN_KINDS
        .iter()
        .chain(std::iter::once(&OTHER_SPAN_KIND))
        .map(|kind| SpanKindReport {
            kind,
            count: 0,
            total_us: 0,
            max_us: 0,
            last_at: 0,
        })
        .collect();
    for span in spans {
        let index = SPAN_KINDS
            .iter()
            .position(|kind| *kind == span.kind)
            .unwrap_or(SPAN_KINDS.len());
        if let Some(entry) = kinds.get_mut(index) {
            entry.count += 1;
            entry.total_us = entry.total_us.saturating_add(span.duration_us);
            entry.max_us = entry.max_us.max(span.duration_us);
            entry.last_at = entry.last_at.max(span.timestamp);
        }
    }
    kinds.retain(|entry| entry.count > 0);
    SpanSummary {
        kept: spans.len(),
        kinds,
    }
}

/// The radar of `store`: counters, active bans, and the type and time of
/// each kept event.
pub(super) fn security_report(store: &SecurityStore) -> Security {
    Security {
        events_total: store.live_events_total(),
        active_bans: store.active_banned_count(),
        counters: store.snapshot(),
        events: store.live_events().iter().map(event_report).collect(),
    }
}

fn event_report(event: &LiveSecurityEvent) -> SecurityEventReport {
    let at = chrono::DateTime::parse_from_rfc3339(&event.timestamp_str)
        .ok()
        .and_then(|time| u64::try_from(time.timestamp()).ok())
        .unwrap_or(0);
    SecurityEventReport {
        event_type: event.event_type.clone(),
        at,
    }
}
