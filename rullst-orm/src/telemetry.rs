//! Internal, secret-free telemetry wrappers used by generated ORM code.

use std::{pin::Pin, task::Poll};

use futures::Stream;

/// A stream that enters its query span only while the inner stream is polled.
///
/// Keeping the span guard inside `poll_next` avoids holding a tracing guard
/// across an asynchronous suspension point.
#[doc(hidden)]
#[non_exhaustive]
pub struct QueryTelemetryStream<S> {
    inner: Pin<Box<S>>,
    span: tracing::Span,
}

impl<S> Stream for QueryTelemetryStream<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let _guard = this.span.enter();
        this.inner.as_mut().poll_next(context)
    }
}

/// Attaches bounded model metadata to every poll of a generated query stream.
#[doc(hidden)]
pub fn instrument_query_stream<S>(
    stream: S,
    model: &'static str,
    table: &'static str,
    operation: &'static str,
) -> QueryTelemetryStream<S>
where
    S: Stream,
{
    QueryTelemetryStream {
        inner: Box::pin(stream),
        span: tracing::info_span!(
            target: "rullst_orm",
            "rullst.orm.query",
            orm.model = model,
            orm.table = table,
            orm.operation = operation,
        ),
    }
}
