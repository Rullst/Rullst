//! `MinimizedProcessor` forwards only approved, valid, well-ordered spans.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use opentelemetry::trace::{SpanId, SpanKind, TraceFlags, TraceId};
use opentelemetry_sdk::trace::{SpanEvents, SpanLinks};
use std::time::SystemTime;

fn processor() -> (MinimizedProcessor, Arc<Counters>) {
    let counters = Arc::new(Counters::default());
    let inner = BatchSpanProcessor::builder(Destination::Offline(counters.clone())).build();
    let processor = MinimizedProcessor {
        inner,
        operations: BTreeSet::from(["operation".to_string()]),
        counters: counters.clone(),
    };
    (processor, counters)
}

fn span(name: &'static str, valid: bool, start: SystemTime, end: SystemTime) -> SpanData {
    let span_context = if valid {
        SpanContext::new(
            TraceId::from_bytes([1; 16]),
            SpanId::from_bytes([2; 8]),
            TraceFlags::SAMPLED,
            false,
            TraceState::default(),
        )
    } else {
        SpanContext::empty_context()
    };
    SpanData {
        span_context,
        parent_span_id: SpanId::INVALID,
        parent_span_is_remote: false,
        span_kind: SpanKind::Internal,
        name: name.into(),
        start_time: start,
        end_time: end,
        attributes: Vec::new(),
        dropped_attributes_count: 0,
        events: SpanEvents::default(),
        links: SpanLinks::default(),
        status: Status::Unset,
        instrumentation_scope: InstrumentationScope::builder("fixture").build(),
    }
}

fn counts(counters: &Counters) -> (u64, u64) {
    (
        counters.forwarded_spans.load(Ordering::Relaxed),
        counters.rejected_spans.load(Ordering::Relaxed),
    )
}

#[test]
fn only_approved_valid_and_ordered_spans_are_forwarded() {
    let (processor, counters) = processor();
    let start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    let later = start + Duration::from_millis(5);

    processor.on_end(span("operation", true, start, start));
    assert_eq!(counts(&counters), (1, 0), "a zero-length span is valid");
    processor.on_end(span("operation", true, start, later));
    assert_eq!(counts(&counters), (2, 0));

    processor.on_end(span("unapproved", true, start, later));
    assert_eq!(counts(&counters), (2, 1), "unapproved name");
    processor.on_end(span("operation", false, start, later));
    assert_eq!(counts(&counters), (2, 2), "invalid span context");
    processor.on_end(span("operation", true, later, start));
    assert_eq!(counts(&counters), (2, 3), "ends before it starts");

    processor.force_flush().unwrap();
    assert_eq!(counters.offline_spans.load(Ordering::Relaxed), 2);
    processor
        .shutdown_with_timeout(Duration::from_secs(5))
        .unwrap();
}
