use super::Counters;
use opentelemetry::{
    Context, InstrumentationScope,
    trace::{SpanContext, Status, TraceState},
};
use opentelemetry_sdk::{
    Resource,
    error::OTelSdkResult,
    trace::{BatchSpanProcessor, Span, SpanData, SpanExporter, SpanProcessor},
};
use std::{
    collections::BTreeSet,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

#[derive(Debug)]
pub(super) struct MinimizedProcessor {
    pub(super) inner: BatchSpanProcessor,
    pub(super) operations: BTreeSet<String>,
    pub(super) counters: Arc<Counters>,
}
impl SpanProcessor for MinimizedProcessor {
    fn on_start(&self, span: &mut Span, context: &Context) {
        self.inner.on_start(span, context);
    }
    fn on_end(&self, mut span: SpanData) {
        if !self.operations.contains(span.name.as_ref())
            || !span.span_context.is_valid()
            || span.end_time < span.start_time
        {
            self.counters.rejected_spans.fetch_add(1, Ordering::Relaxed);
            return;
        }
        span.attributes = Vec::new();
        span.events = Default::default();
        span.links = Default::default();
        span.dropped_attributes_count = 0;
        if matches!(span.status, Status::Error { .. }) {
            span.status = Status::error("");
        }
        let context = &span.span_context;
        span.span_context = SpanContext::new(
            context.trace_id(),
            context.span_id(),
            context.trace_flags(),
            context.is_remote(),
            TraceState::default(),
        );
        span.instrumentation_scope = InstrumentationScope::builder("rullst.minimized").build();
        self.counters
            .forwarded_spans
            .fetch_add(1, Ordering::Relaxed);
        self.inner.on_end(span);
    }
    fn force_flush(&self) -> OTelSdkResult {
        self.inner.force_flush()
    }
    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.inner.shutdown_with_timeout(timeout)
    }
    fn set_resource(&mut self, resource: &Resource) {
        self.inner.set_resource(resource);
    }
}

#[derive(Debug)]
pub(super) enum Destination {
    Remote(opentelemetry_otlp::SpanExporter),
    Offline(Arc<Counters>),
}
impl SpanExporter for Destination {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        match self {
            Self::Remote(exporter) => exporter.export(batch).await,
            Self::Offline(counters) => {
                counters
                    .offline_spans
                    .fetch_add(batch.len() as u64, Ordering::Relaxed);
                Ok(())
            }
        }
    }
    fn set_resource(&mut self, resource: &Resource) {
        if let Self::Remote(exporter) = self {
            exporter.set_resource(resource);
        }
    }
}
