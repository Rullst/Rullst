//! Sampling and bounds of the request summaries.
#![allow(clippy::unwrap_used)]

use super::collector::Collector;
use super::traces::{MAX_TRACES, RequestTrace, Sampler, is_outlier};
use crate::server::dev_telemetry::request_scope::RequestQueries;
use axum::http::Method;
use std::sync::Arc;
use std::time::Duration;

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

fn trace(status: u16) -> RequestTrace {
    RequestTrace {
        at: 1_800_000_000,
        method: "GET",
        route: Arc::from("/orders/{id}"),
        status,
        latency: ms(3),
        db_operations: 2,
        n_plus_one: false,
    }
}

#[test]
fn server_errors_and_slow_requests_are_outliers() {
    assert!(is_outlier(500, ms(1)));
    assert!(is_outlier(503, ms(1)));
    assert!(!is_outlier(499, ms(999)));
    assert!(is_outlier(200, ms(1_000)));
    assert!(!is_outlier(200, ms(999)));
}

#[test]
fn ordinary_requests_are_kept_at_most_every_100_ms() {
    let sampler = Sampler::default();
    assert!(sampler.admit(ms(0), false));
    assert!(!sampler.admit(ms(0), false));
    assert!(!sampler.admit(ms(99), false));
    assert!(sampler.admit(ms(100), false));
    assert!(!sampler.admit(ms(150), false));
    assert!(sampler.admit(ms(250), false));
    assert!(!sampler.admit(ms(349), false));
}

#[test]
fn outliers_have_their_own_10_ms_gate() {
    let sampler = Sampler::default();
    assert!(sampler.admit(ms(0), false));
    // The ordinary gate is closed; the outlier gate is not.
    assert!(sampler.admit(ms(5), true));
    assert!(!sampler.admit(ms(14), true));
    assert!(sampler.admit(ms(15), true));
    assert!(!sampler.admit(ms(20), false));
    assert!(sampler.admit(ms(100), false));
}

#[test]
fn the_newest_summaries_are_kept_first_and_bounded() {
    let sampler = Sampler::default();
    for status in 0..(MAX_TRACES as u16 + 5) {
        sampler.keep(trace(status));
    }
    let (total, kept) = sampler.snapshot();
    assert_eq!(total, 55);
    assert_eq!(kept.len(), 50);
    assert_eq!(kept[0].status, 54);
    assert_eq!(kept[49].status, 5);
}

#[test]
fn the_collector_keeps_a_sampled_summary_per_request() {
    let collector = Collector::new();
    let repeated = RequestQueries {
        operations: 5,
        fingerprints: vec!["Post.find (posts)".to_string(); 3],
    };
    let none = RequestQueries::default();
    collector.record_at(
        ms(0),
        &Method::GET,
        Some("/posts/{id}"),
        200,
        ms(3),
        &repeated,
    );
    // Within 100 ms of the first: an ordinary request is skipped...
    collector.record_at(ms(40), &Method::POST, Some("/posts"), 201, ms(2), &none);
    // ...an error is kept.
    collector.record_at(ms(50), &Method::DELETE, None, 502, ms(7), &none);
    let (total, kept) = collector.traces();
    assert_eq!(total, 2);
    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].method, "DELETE");
    assert_eq!(&*kept[0].route, "(unmatched)");
    assert_eq!(kept[0].status, 502);
    assert_eq!(kept[0].latency, ms(7));
    assert_eq!(kept[0].db_operations, 0);
    assert!(!kept[0].n_plus_one);
    assert_eq!(&*kept[1].route, "/posts/{id}");
    assert_eq!(kept[1].db_operations, 5);
    assert!(kept[1].n_plus_one);
    assert!(kept[1].at > 1_700_000_000);
}
