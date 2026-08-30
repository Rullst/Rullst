#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use std::sync::{Arc, Mutex};

use futures::StreamExt;
use rullst_orm::{Error, Orm};
use tracing::{Event, Subscriber, span};
use tracing_subscriber::{Layer, layer::Context, prelude::*};

#[derive(Debug, Clone, rullst_orm::FromRow, rullst_orm::Orm)]
#[orm(table = "telemetry_probe")]
struct TelemetryProbe {
    id: i32,
    label: String,
}

#[derive(Clone, Default)]
struct CaptureLayer {
    spans: Arc<Mutex<Vec<String>>>,
    event_targets: Arc<Mutex<Vec<String>>>,
    fields: Arc<Mutex<Vec<String>>>,
}

struct FieldVisitor<'a> {
    fields: &'a Arc<Mutex<Vec<String>>>,
}

impl tracing::field::Visit for FieldVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if let Ok(mut fields) = self.fields.lock() {
            fields.push(format!("{}={value:?}", field.name()));
        }
    }
}

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber,
{
    fn on_new_span(&self, attributes: &span::Attributes<'_>, _id: &span::Id, _ctx: Context<'_, S>) {
        if let Ok(mut spans) = self.spans.lock() {
            spans.push(attributes.metadata().name().to_owned());
        }
        attributes.record(&mut FieldVisitor {
            fields: &self.fields,
        });
    }

    fn on_record(&self, _span: &span::Id, values: &span::Record<'_>, _ctx: Context<'_, S>) {
        values.record(&mut FieldVisitor {
            fields: &self.fields,
        });
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if let Ok(mut targets) = self.event_targets.lock() {
            targets.push(event.metadata().target().to_owned());
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn queries_transactions_and_pool_acquires_emit_redacted_telemetry() {
    let capture = CaptureLayer::default();
    let subscriber = tracing_subscriber::registry().with(capture.clone());
    let _subscriber_guard = tracing::subscriber::set_default(subscriber);

    Orm::init_with_options("sqlite::memory:", 1, 10)
        .await
        .expect("isolated telemetry database should initialize");
    Orm::raw("CREATE TABLE telemetry_probe (id INTEGER PRIMARY KEY, label TEXT NOT NULL)")
        .execute()
        .await
        .expect("telemetry table should be created");

    let mut probe = TelemetryProbe {
        id: 0,
        label: "model-secret-value".to_owned(),
    };
    probe
        .save()
        .await
        .expect("instrumented save should succeed");
    let rows = TelemetryProbe::query()
        .get()
        .await
        .expect("instrumented query should succeed");
    assert_eq!(rows.len(), 1);
    {
        let stream_query = TelemetryProbe::query();
        let mut stream = std::pin::pin!(stream_query.stream());
        assert!(stream.next().await.is_some());
    }

    Orm::transaction(|_| {
        Box::pin(async {
            Orm::raw("INSERT INTO telemetry_probe (label) VALUES (?)")
                .bind("redacted-value")
                .execute()
                .await?;
            Ok::<(), Error>(())
        })
    })
    .await
    .expect("instrumented transaction should commit");

    let rollback = Orm::transaction(|_| {
        Box::pin(async { Err::<(), _>("intentional rollback for telemetry") })
    })
    .await;
    assert!(rollback.is_err());

    let spans = capture.spans.lock().expect("span capture lock");
    assert!(spans.iter().any(|name| name == "rullst.orm.query"));
    assert!(spans.iter().any(|name| name == "rullst.orm.transaction"));
    assert!(
        spans
            .iter()
            .any(|name| name == "rullst.orm.transaction.begin")
    );
    drop(spans);

    let targets = capture
        .event_targets
        .lock()
        .expect("event target capture lock");
    assert!(targets.iter().any(|target| target == "sqlx::pool::acquire"));
    drop(targets);

    let fields = capture.fields.lock().expect("field capture lock");
    assert!(
        fields
            .iter()
            .any(|field| field == "orm.model=\"TelemetryProbe\"")
    );
    assert!(
        fields
            .iter()
            .any(|field| field == "orm.table=\"telemetry_probe\"")
    );
    for operation in ["save", "select_many", "stream"] {
        assert!(
            fields
                .iter()
                .any(|field| field == &format!("orm.operation={operation:?}"))
        );
    }
    assert!(
        fields
            .iter()
            .any(|field| field == "orm.outcome=\"committed\"")
    );
    assert!(
        fields
            .iter()
            .any(|field| field == "orm.outcome=\"rolled_back\"")
    );
    assert!(fields.iter().all(|field| !field.contains("redacted-value")));
    assert!(
        fields
            .iter()
            .all(|field| !field.contains("model-secret-value"))
    );
    assert!(fields.iter().all(|field| !field.contains("CREATE TABLE")));
    assert!(fields.iter().all(|field| !field.contains("INSERT INTO")));
}
