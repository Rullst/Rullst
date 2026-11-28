//! The subscriber `init_telemetry` installs without the `telemetry` feature:
//! the lines it writes at the default filter, `RUST_LOG` overrides and the
//! ORM span check of the development dashboard and the monitor.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::telemetry::{DEFAULT_LOG_FILTER, RedactPersonalDataLayer};
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing_core::{Level, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;

/// What the `fmt` layer wrote.
#[derive(Clone, Default)]
pub(crate) struct Lines(Arc<Mutex<Vec<u8>>>);

impl Lines {
    pub(crate) fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl Write for Lines {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Lines {
    type Writer = Lines;

    fn make_writer(&'a self) -> Lines {
        self.clone()
    }
}

/// The layers `init_telemetry` installs without the `telemetry` feature,
/// with `directives` as `RUST_LOG` and the `fmt` output in `lines`
/// (without colours).
pub(crate) fn subscriber(directives: &str, lines: Lines) -> impl Subscriber + Send + Sync {
    tracing_subscriber::registry()
        .with(EnvFilter::new(directives))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(lines)
                .with_ansi(false),
        )
        .with(RedactPersonalDataLayer)
        .with(crate::server::dev_telemetry::query_layer())
}

/// One ORM operation span, as `rullst-orm` emits it.
pub(crate) fn orm_span(model: &str, table: &str) -> tracing::Span {
    tracing::info_span!(
        target: "rullst_orm",
        "rullst.orm.query",
        orm.model = model,
        orm.table = table,
        orm.operation = "find"
    )
}

#[test]
fn the_default_filter_writes_application_and_sqlx_warning_lines() {
    let lines = Lines::default();
    tracing::subscriber::with_default(subscriber(DEFAULT_LOG_FILTER, lines.clone()), || {
        tracing::info!(target: "my_app::orders", "order placed");
        tracing::info!(target: "sqlx::pool::acquire", "acquired connection");
        tracing::warn!(target: "sqlx::pool::acquire", "slow acquire");
        orm_span("Post", "posts").in_scope(|| tracing::warn!(target: "my_app::orders", "inside"));
    });
    let text = lines.text();
    assert!(
        text.contains(" INFO my_app::orders: order placed\n"),
        "{text}"
    );
    assert!(!text.contains("acquired connection"), "{text}");
    assert!(
        text.contains(" WARN sqlx::pool::acquire: slow acquire\n"),
        "{text}"
    );
    assert!(
        text.contains(
            " WARN rullst.orm.query{orm.model=\"Post\" orm.table=\"posts\" \
             orm.operation=\"find\"}: my_app::orders: inside\n"
        ),
        "{text}"
    );
    assert_eq!(text.lines().count(), 3, "{text}");
}

#[test]
fn rust_log_overrides_keep_their_meaning() {
    let lines = Lines::default();
    tracing::subscriber::with_default(subscriber("info,sqlx=info", lines.clone()), || {
        tracing::info!(target: "sqlx::pool::acquire", "acquired connection");
    });
    let text = lines.text();
    assert!(
        text.contains(" INFO sqlx::pool::acquire: acquired connection\n"),
        "{text}"
    );

    // `warn` drops INFO lines and the ORM spans; the dashboard and the
    // monitor see that through `tracing::enabled!`.
    let lines = Lines::default();
    tracing::subscriber::with_default(subscriber("warn", lines.clone()), || {
        tracing::info!(target: "my_app::orders", "order placed");
        assert!(orm_span("Post", "posts").is_disabled());
        assert!(!tracing::enabled!(
            kind: tracing::metadata::Kind::SPAN,
            target: "rullst_orm",
            Level::INFO
        ));
    });
    assert_eq!(lines.text(), "");

    // A span directive selects DEBUG events inside ORM spans.
    let lines = Lines::default();
    let directives = "info,[rullst.orm.query]=debug";
    tracing::subscriber::with_default(subscriber(directives, lines.clone()), || {
        tracing::debug!(target: "my_app::orders", "before");
        let span = orm_span("Post", "posts");
        assert!(!span.is_disabled());
        span.in_scope(|| tracing::debug!(target: "my_app::orders", "inside"));
        tracing::debug!(target: "my_app::orders", "after");
    });
    let text = lines.text();
    assert!(text.contains("}: my_app::orders: inside\n"), "{text}");
    assert_eq!(text.lines().count(), 1, "{text}");
}

#[test]
fn the_dashboard_and_monitor_check_follows_the_filter() {
    crate::server::dev_telemetry::mark_installed();
    let default = subscriber(DEFAULT_LOG_FILTER, Lines::default());
    tracing::subscriber::with_default(default, || {
        assert_eq!(crate::server::dev_telemetry::orm_unavailable_reason(), None);
    });
    let quiet = subscriber("warn", Lines::default());
    tracing::subscriber::with_default(quiet, || {
        assert_eq!(
            crate::server::dev_telemetry::orm_unavailable_reason(),
            Some("orm_spans_filtered")
        );
    });
}
