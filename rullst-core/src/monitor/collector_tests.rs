#![allow(clippy::unwrap_used)]

use super::collector::{
    Collector, MAX_FINDINGS, MAX_ROUTES, method_index, method_label, route_label,
};
use crate::server::dev_telemetry::request_scope::RequestQueries;
use axum::http::Method;
use std::time::Duration;

fn no_queries() -> RequestQueries {
    RequestQueries::default()
}

fn record(collector: &Collector, method: Method, route: Option<&str>, status: u16) {
    collector.record_at(
        Duration::from_secs(1),
        &method,
        route,
        status,
        Duration::from_millis(2),
        &no_queries(),
    );
}

fn repeated(fingerprint: &str, times: usize) -> RequestQueries {
    RequestQueries {
        operations: times as u64,
        fingerprints: vec![fingerprint.to_string(); times],
    }
}

#[test]
fn methods_are_normalized_to_a_fixed_set() {
    let cases = [
        (Method::GET, "GET", 0),
        (Method::HEAD, "HEAD", 1),
        (Method::POST, "POST", 2),
        (Method::PUT, "PUT", 3),
        (Method::PATCH, "PATCH", 4),
        (Method::DELETE, "DELETE", 5),
        (Method::OPTIONS, "OPTIONS", 6),
        (Method::CONNECT, "CONNECT", 7),
        (Method::TRACE, "TRACE", 8),
        (Method::from_bytes(b"PURGE").unwrap(), "OTHER", 9),
    ];
    assert_eq!(method_label(10), "OTHER");
    for (method, label, index) in cases {
        assert_eq!(method_label(method_index(&method)), label);
        assert_eq!(method_index(&method), index);
    }
}

#[test]
fn route_labels_are_cleaned_and_bounded() {
    assert_eq!(route_label("/users/{id}"), "/users/{id}");
    assert_eq!(route_label("/a\u{1b}[31m/b"), "/a?[31m/b");
    assert_eq!(
        route_label("/static/{*__private__axum_nest_tail_param}"),
        "/static/{*rest}"
    );
    let long = format!("/{}", "é".repeat(200));
    let label = route_label(&long);
    assert_eq!(label.len(), 255);
    assert!(label.starts_with("/é"));
    assert_eq!(route_label(&"/x".repeat(128)).len(), 256);
    assert_eq!(route_label(&"/x".repeat(129)).len(), 256);
}

#[test]
fn routes_are_reported_by_pattern_and_method_in_order() {
    let collector = Collector::new();
    record(&collector, Method::POST, Some("/users/{id}"), 201);
    record(&collector, Method::GET, Some("/users/{id}"), 200);
    record(&collector, Method::GET, Some("/users/{id}"), 404);
    record(&collector, Method::GET, None, 404);
    record(&collector, Method::GET, Some("/"), 200);

    let routes = collector.routes();
    let keys: Vec<(&str, &str, u64)> = routes
        .iter()
        .map(|route| (route.method, &*route.route, route.stats.requests_total))
        .collect();
    assert_eq!(
        keys,
        [
            ("GET", "(unmatched)", 1),
            ("GET", "/", 1),
            ("GET", "/users/{id}", 2),
            ("POST", "/users/{id}", 1),
        ]
    );
    assert_eq!(routes[2].stats.status_total.client_error, 1);
    assert!(!collector.overflowed());
}

#[test]
fn patterns_beyond_the_limit_share_the_other_bucket() {
    let collector = Collector::new();
    for index in 0..MAX_ROUTES {
        record(&collector, Method::GET, Some(&format!("/r{index}")), 200);
    }
    assert!(!collector.overflowed());
    assert_eq!(collector.routes().len(), 256);

    for index in 0..40 {
        record(&collector, Method::GET, Some(&format!("/late{index}")), 200);
    }
    // A new method on a known route is a new pair too.
    record(&collector, Method::DELETE, Some("/r0"), 204);
    // Known pairs keep their own statistics.
    record(&collector, Method::GET, Some("/r0"), 200);

    assert!(collector.overflowed());
    let routes = collector.routes();
    assert_eq!(routes.len(), 257);
    let other = routes.last().unwrap();
    assert_eq!((other.method, &*other.route), ("*", "(other)"));
    assert_eq!(other.stats.requests_total, 41);
    let first = routes.iter().find(|route| &*route.route == "/r0").unwrap();
    assert_eq!(first.stats.requests_total, 2);
}

#[test]
fn queries_and_repetitions_are_aggregated_per_route_and_fingerprint() {
    let collector = Collector::new();
    let at = Duration::from_secs(1);
    let latency = Duration::from_millis(1);
    let route = Some("/posts/{id}");
    collector.record_at(
        at,
        &Method::GET,
        route,
        200,
        latency,
        &repeated("Post.find (posts)", 2),
    );
    assert_eq!(collector.findings(), (0, vec![]));
    assert_eq!(collector.queries(), 2);

    collector.record_at(
        at,
        &Method::GET,
        route,
        200,
        latency,
        &repeated("Post.find (posts)", 3),
    );
    collector.record_at(
        at,
        &Method::GET,
        route,
        200,
        latency,
        &repeated("Post.find (posts)", 5),
    );
    collector.record_at(
        at,
        &Method::GET,
        route,
        200,
        latency,
        &repeated("Post.find (posts)", 4),
    );
    collector.record_at(
        at,
        &Method::POST,
        route,
        200,
        latency,
        &repeated("Comment.find (comments)", 3),
    );
    assert_eq!(collector.queries(), 17);

    let (total, findings) = collector.findings();
    assert_eq!(total, 4);
    assert_eq!(findings.len(), 2);
    // Most recently seen first.
    assert_eq!(findings[0].method, "POST");
    assert_eq!(findings[0].fingerprint, "Comment.find (comments)");
    assert_eq!(findings[0].requests, 1);
    let posts = &findings[1];
    assert_eq!((posts.method, &*posts.route), ("GET", "/posts/{id}"));
    assert_eq!(posts.requests, 3);
    assert_eq!(posts.max_occurrences, 5);
    assert!(posts.last_seen > 1_700_000_000);
}

#[test]
fn the_least_recently_seen_finding_is_replaced_at_the_limit() {
    let collector = Collector::new();
    let at = Duration::from_secs(1);
    let latency = Duration::from_millis(1);
    let record = |fingerprint: &str| {
        collector.record_at(
            at,
            &Method::GET,
            Some("/feed"),
            200,
            latency,
            &repeated(fingerprint, 3),
        );
    };
    for index in 0..MAX_FINDINGS {
        record(&format!("Model{index}.find (t)"));
    }
    // Seeing the oldest again makes Model1 the least recently seen.
    record("Model0.find (t)");
    record("Late.find (t)");

    let (total, findings) = collector.findings();
    assert_eq!(total, 66);
    assert_eq!(findings.len(), 64);
    assert_eq!(findings[0].fingerprint, "Late.find (t)");
    assert_eq!(findings[1].fingerprint, "Model0.find (t)");
    assert_eq!(findings[1].requests, 2);
    assert!(
        findings
            .iter()
            .all(|finding| finding.fingerprint != "Model1.find (t)")
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.fingerprint == "Model2.find (t)")
    );
}
