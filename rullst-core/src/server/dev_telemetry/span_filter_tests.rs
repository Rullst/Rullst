//! Whether the installed subscriber keeps the ORM spans the query counts need.

use super::orm_spans_enabled;

#[test]
fn orm_spans_count_only_when_the_subscriber_keeps_info_spans() {
    let quiet = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .finish();
    assert!(!tracing::subscriber::with_default(quiet, orm_spans_enabled));
    let verbose = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish();
    assert!(tracing::subscriber::with_default(
        verbose,
        orm_spans_enabled
    ));
}
