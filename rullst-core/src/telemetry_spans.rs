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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_span_collector_capacity_and_eviction() {
        let collector = SpanCollector::new(3);
        assert_eq!(collector.snapshot().len(), 0);

        for i in 1..=5 {
            collector.record(TraceSpan {
                name: format!("operation_{}", i),
                kind: "sql".to_string(),
                duration_us: i * 100,
                timestamp: 1700000000 + i,
            });
        }

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.len(), 3);
        assert_eq!(snapshot[0].name, "operation_3");
        assert_eq!(snapshot[1].name, "operation_4");
        assert_eq!(snapshot[2].name, "operation_5");
    }

    #[test]
    fn test_global_span_collector_and_serde() {
        let global = global_span_collector();
        global.record(TraceSpan {
            name: "GET /api/v1/users".to_string(),
            kind: "http".to_string(),
            duration_us: 1500,
            timestamp: 1700000000,
        });

        let snapshot = global.snapshot();
        assert!(!snapshot.is_empty());

        let span = &snapshot[snapshot.len() - 1];
        let json = serde_json::to_string(span).unwrap();
        assert!(json.contains("GET /api/v1/users"));

        let deserialized: TraceSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, span.name);
        assert_eq!(deserialized.kind, span.kind);
        assert_eq!(deserialized.duration_us, span.duration_us);
    }
}
