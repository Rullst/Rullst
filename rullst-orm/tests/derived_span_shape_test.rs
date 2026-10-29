//! The `rullst.orm.query` span of generated methods keeps its shape: name,
//! target, level, fields in order and how each is recorded, and the query runs
//! inside it. Before 13.0.0-alpha.4 `#[derive(Orm)]` built the span with
//! `#[tracing::instrument]`; it now builds it through `rullst_orm::_tracing`,
//! and the traces must not change.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use rullst_orm::{Error, Orm};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber, span};
use tracing_subscriber::{Layer, layer::Context, prelude::*, registry::LookupSpan};

#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "span_shape_probe")]
struct SpanShapeProbe {
    id: i32,
    label: String,
}

/// A query span as a subscriber sees it.
#[derive(Debug, Clone, PartialEq)]
struct QuerySpan {
    target: String,
    level: Level,
    /// `name=kind:value`, in the order the span declares its fields.
    fields: Vec<String>,
}

struct Recorder<'a>(&'a mut Vec<String>);

impl Visit for Recorder<'_> {
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.push(format!("{}=u64:{value}", field.name()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.push(format!("{}=i64:{value}", field.name()));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push(format!("{}=str:{value}", field.name()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0.push(format!("{}=debug:{value:?}", field.name()));
    }
}

#[derive(Clone, Default)]
struct Capture {
    /// Query spans by operation, in creation order.
    spans: Arc<Mutex<Vec<(String, QuerySpan)>>>,
    operations: Arc<Mutex<HashMap<span::Id, String>>>,
    /// Targets of events emitted inside a query span, with its operation.
    events: Arc<Mutex<Vec<(String, String)>>>,
}

impl<S> Layer<S> for Capture
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attributes: &span::Attributes<'_>, id: &span::Id, _: Context<'_, S>) {
        let metadata = attributes.metadata();
        if metadata.name() != "rullst.orm.query" {
            return;
        }
        let mut fields = Vec::new();
        attributes.record(&mut Recorder(&mut fields));
        let operation = fields
            .iter()
            .find_map(|field| field.strip_prefix("orm.operation=str:"))
            .unwrap_or_default()
            .to_owned();
        self.operations
            .lock()
            .unwrap()
            .insert(id.clone(), operation.clone());
        self.spans.lock().unwrap().push((
            operation,
            QuerySpan {
                target: metadata.target().to_owned(),
                level: *metadata.level(),
                fields,
            },
        ));
    }

    fn on_event(&self, event: &Event<'_>, context: Context<'_, S>) {
        let Some(scope) = context.event_scope(event) else {
            return;
        };
        let operations = self.operations.lock().unwrap();
        if let Some(operation) = scope
            .into_iter()
            .find_map(|span| operations.get(&span.id()))
        {
            self.events
                .lock()
                .unwrap()
                .push((event.metadata().target().to_owned(), operation.clone()));
        }
    }
}

fn expected(recorded: &[&str], operation: &str) -> QuerySpan {
    let mut fields = recorded
        .iter()
        .map(|field| (*field).to_owned())
        .collect::<Vec<_>>();
    fields.extend([
        "orm.model=str:SpanShapeProbe".to_owned(),
        "orm.table=str:span_shape_probe".to_owned(),
        format!("orm.operation=str:{operation}"),
    ]);
    QuerySpan {
        target: "rullst_orm".to_owned(),
        level: Level::INFO,
        fields,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn derived_query_spans_keep_their_shape_and_enclose_the_query() {
    let capture = Capture::default();
    let _guard =
        tracing::subscriber::set_default(tracing_subscriber::registry().with(capture.clone()));

    Orm::init_with_options("sqlite::memory:", 1, 10)
        .await
        .unwrap();
    Orm::raw("CREATE TABLE span_shape_probe (id INTEGER PRIMARY KEY, label TEXT NOT NULL)")
        .execute()
        .await
        .unwrap();
    // Only the generated methods below are under test, not `Orm::raw`.
    capture.spans.lock().unwrap().clear();

    let mut probe = SpanShapeProbe {
        id: 0,
        label: "first".to_owned(),
    };
    probe.save().await.unwrap();
    let page = SpanShapeProbe::query().paginate(2, 5).await.unwrap();
    assert_eq!(page.total, 1);
    SpanShapeProbe::query()
        .chunk_by_id(10, |rows| async move {
            assert_eq!(rows.len(), 1);
            Ok::<(), Error>(())
        })
        .await
        .unwrap();
    assert_eq!(SpanShapeProbe::query().count().await.unwrap(), 1);

    let spans = capture.spans.lock().unwrap().clone();
    let first = |operation: &str| {
        spans
            .iter()
            .find(|(captured, _)| captured == operation)
            .map(|(_, span)| span.clone())
            .unwrap_or_else(|| panic!("no {operation} span in {spans:#?}"))
    };
    assert_eq!(first("save"), expected(&[], "save"));
    // `instrument` recorded the parameters it did not skip, primitives by value.
    assert_eq!(
        first("paginate"),
        expected(&["page=u64:2", "per_page=u64:5"], "paginate")
    );
    assert_eq!(
        first("chunk_by_id"),
        expected(&["size=u64:10"], "chunk_by_id")
    );
    assert_eq!(first("select_many"), expected(&[], "select_many"));
    assert_eq!(first("count"), expected(&[], "count"));
    // paginate and chunk_by_id read their rows through a nested get().
    let operations = spans
        .iter()
        .map(|(operation, _)| operation.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        operations,
        [
            "save",
            "paginate",
            "select_many",
            "chunk_by_id",
            "select_many",
            "count"
        ]
    );

    // The pool checkout of each query happens while its span is entered.
    let events = capture.events.lock().unwrap().clone();
    for operation in ["save", "paginate", "select_many", "count"] {
        assert!(
            events
                .iter()
                .any(|(target, inside)| target == "sqlx::pool::acquire" && inside == operation),
            "{operation}: {events:#?}"
        );
    }
}
