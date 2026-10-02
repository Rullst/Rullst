//! Counts ORM operations from the secret-free `rullst.orm.query` spans.
//!
//! The spans carry only static model, table and operation labels; SQL text and
//! bindings are never part of them. Only the outermost ORM span of a call tree
//! is counted, so an operation that runs another (eager loading, a save that
//! inserts) is one operation, timed from span creation until it closes.

use super::recorder::{self, QueryLabels, Recorder};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tracing_core::field::{Field, Visit};
use tracing_core::{Subscriber, span};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

pub(super) const QUERY_SPAN: &str = "rullst.orm.query";
pub(super) const QUERY_TARGET: &str = "rullst_orm";
/// Ancestors inspected when deciding whether an ORM span is nested.
const MAX_ANCESTORS: usize = 64;

static INSTALLED: AtomicBool = AtomicBool::new(false);

/// Records that the global subscriber includes [`OrmQueryLayer`].
pub(crate) fn mark_installed() {
    INSTALLED.store(true, Ordering::Relaxed);
}

pub(super) fn installed() -> bool {
    INSTALLED.load(Ordering::Relaxed)
}

/// The layer `telemetry::init_telemetry` adds in debug builds. It records
/// nothing until the development endpoint has been mounted.
pub(crate) fn debug_layer() -> Option<OrmQueryLayer> {
    cfg!(debug_assertions).then(OrmQueryLayer::global)
}

pub(crate) struct OrmQueryLayer {
    sink: Option<Arc<Recorder>>,
}

impl OrmQueryLayer {
    fn global() -> Self {
        Self { sink: None }
    }

    #[cfg(test)]
    pub(super) fn local(recorder: Arc<Recorder>) -> Self {
        Self {
            sink: Some(recorder),
        }
    }

    fn recorder(&self) -> Option<&Arc<Recorder>> {
        self.sink.as_ref().or_else(|| recorder::global())
    }
}

struct Started {
    at: Instant,
    labels: QueryLabels,
}

fn is_query_span(metadata: &tracing_core::Metadata<'_>) -> bool {
    metadata.name() == QUERY_SPAN && metadata.target() == QUERY_TARGET
}

impl<S> Layer<S> for OrmQueryLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attributes: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        if !is_query_span(attributes.metadata()) || self.recorder().is_none() {
            return;
        }
        let Some(span) = ctx.span(id) else {
            return;
        };
        let nested = span
            .scope()
            .skip(1)
            .take(MAX_ANCESTORS)
            .any(|ancestor| is_query_span(ancestor.metadata()));
        if nested {
            return;
        }
        let mut labels = LabelVisitor::default();
        attributes.record(&mut labels);
        span.extensions_mut().insert(Started {
            at: Instant::now(),
            labels: labels.0,
        });
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else {
            return;
        };
        let Some(started) = span.extensions_mut().remove::<Started>() else {
            return;
        };
        if let Some(recorder) = self.recorder() {
            recorder.record_query(started.labels, started.at.elapsed());
        }
    }
}

#[derive(Default)]
struct LabelVisitor(QueryLabels);

impl Visit for LabelVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        let slot = match field.name() {
            "orm.operation" => &mut self.0.operation,
            "orm.model" => &mut self.0.model,
            "orm.table" => &mut self.0.table,
            _ => return,
        };
        *slot = Some(value.to_string());
    }

    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
}
