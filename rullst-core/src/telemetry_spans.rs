//! Distributed Tracing Telemetry & Flamegraph Collector.

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Represents a single telemetry span (HTTP request, ORM query, AI prompt).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceSpan {
    /// Name of the operation (e.g. "GET /api/users", "SELECT * FROM users").
    pub name: String,
    /// Kind of span: "http", "sql", "ai", "job".
    pub kind: String,
    /// Execution duration in microseconds.
    pub duration_us: u64,
    /// Epoch timestamp in seconds.
    pub timestamp: u64,
}

/// In-memory circular buffer collector for distributed trace spans.
pub struct SpanCollector {
    spans: RwLock<Vec<TraceSpan>>,
    capacity: usize,
}

impl SpanCollector {
    /// Creates a new SpanCollector with a fixed capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            spans: RwLock::new(Vec::with_capacity(capacity)),
            capacity,
        }
    }

    /// Records a new trace span into the circular buffer.
    pub fn record(&self, span: TraceSpan) {
        if let Ok(mut lock) = self.spans.write() {
            if lock.len() >= self.capacity {
                lock.remove(0);
            }
            lock.push(span);
        }
    }

    /// Returns a snapshot copy of all recorded trace spans.
    pub fn snapshot(&self) -> Vec<TraceSpan> {
        self.spans
            .read()
            .map(|lock| lock.clone())
            .unwrap_or_default()
    }
}

static GLOBAL_SPAN_COLLECTOR: std::sync::OnceLock<SpanCollector> = std::sync::OnceLock::new();

/// Returns the global SpanCollector instance.
pub fn global_span_collector() -> &'static SpanCollector {
    GLOBAL_SPAN_COLLECTOR.get_or_init(|| SpanCollector::new(500))
}
