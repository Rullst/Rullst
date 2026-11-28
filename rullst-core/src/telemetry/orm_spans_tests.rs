//! The ORM span call sites wait for a recorder; every other call site, and
//! every ORM span under a span directive, is decided by the `EnvFilter`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{OrmSpansOnDemand, has_span_directives};
use crate::telemetry::DEFAULT_LOG_FILTER;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_core::callsite::DefaultCallsite;
use tracing_core::metadata::Kind;
use tracing_core::{Level, Metadata, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;

/// Static metadata of a call site, to ask a subscriber directly.
macro_rules! callsite {
    ($kind:expr, $target:literal, $name:literal) => {{
        static CALLSITE: DefaultCallsite = DefaultCallsite::new(&META);
        static META: Metadata<'static> = tracing_core::metadata! {
            name: $name,
            target: $target,
            level: Level::INFO,
            fields: &[],
            callsite: &CALLSITE,
            kind: $kind,
        };
        &META
    }};
}

#[test]
fn only_bracketed_directives_select_by_span() {
    for (directives, expected) in [
        ("info,sqlx=warn", false),
        ("off", false),
        ("warn,rullst_orm=info", false),
        ("info,[rullst.orm.query]=debug", true),
        ("my_app[request{id=1}]=trace", true),
        ("[{orm.model}]=debug", true),
    ] {
        let filter = EnvFilter::new(directives);
        assert_eq!(has_span_directives(&filter), expected, "{directives}");
    }
}

#[test]
fn orm_span_call_sites_wait_for_a_recorder() {
    static RECORDED: AtomicBool = AtomicBool::new(false);
    let filter = EnvFilter::new(DEFAULT_LOG_FILTER);
    let orm_spans = OrmSpansOnDemand::new(&filter, &RECORDED);
    let subscriber = tracing_subscriber::registry().with(filter).with(orm_spans);
    let query = callsite!(Kind::SPAN, "rullst_orm", "rullst.orm.query");
    let begin = callsite!(Kind::SPAN, "rullst_orm", "rullst.orm.transaction.begin");

    assert!(subscriber.register_callsite(query).is_sometimes());
    assert!(subscriber.register_callsite(begin).is_sometimes());
    assert!(!subscriber.enabled(query));
    assert!(!subscriber.enabled(begin));
    RECORDED.store(true, Ordering::Relaxed);
    assert!(subscriber.enabled(query));
    assert!(subscriber.enabled(begin));
    assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::INFO));
}

#[test]
fn other_call_sites_follow_the_env_filter_alone() {
    static RECORDED: AtomicBool = AtomicBool::new(false);
    let filter = EnvFilter::new(DEFAULT_LOG_FILTER);
    let orm_spans = OrmSpansOnDemand::new(&filter, &RECORDED);
    let subscriber = tracing_subscriber::registry().with(filter).with(orm_spans);
    let kept = [
        // An application span that borrows the ORM's span name.
        callsite!(Kind::SPAN, "my_app", "rullst.orm.query"),
        // An ORM event (events are named `event <file>:<line>` unless named).
        callsite!(Kind::EVENT, "rullst_orm", "rullst.orm.query"),
        // `tracing::enabled!(kind: SPAN, target: "rullst_orm", INFO)`.
        callsite!(Kind::SPAN.hint(), "rullst_orm", "enabled src/check.rs:1"),
    ];
    for metadata in kept {
        assert!(
            subscriber.register_callsite(metadata).is_always(),
            "{metadata:?}"
        );
        assert!(subscriber.enabled(metadata), "{metadata:?}");
    }
    let sqlx_info = callsite!(Kind::EVENT, "sqlx::pool::acquire", "event src/pool.rs:1");
    assert!(subscriber.register_callsite(sqlx_info).is_never());
    assert!(!subscriber.enabled(sqlx_info));
}

#[test]
fn a_span_directive_keeps_every_orm_span() {
    static RECORDED: AtomicBool = AtomicBool::new(false);
    let filter = EnvFilter::new("info,[rullst.orm.query]=debug");
    let orm_spans = OrmSpansOnDemand::new(&filter, &RECORDED);
    let subscriber = tracing_subscriber::registry().with(filter).with(orm_spans);
    let query = callsite!(Kind::SPAN, "rullst_orm", "rullst.orm.query");
    assert!(subscriber.register_callsite(query).is_always());
    assert!(subscriber.enabled(query));
}
