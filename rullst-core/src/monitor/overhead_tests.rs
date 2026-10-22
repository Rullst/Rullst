//! Measures the recorder's cost per request and its memory bound. Ignored by
//! default (timing depends on the machine); the production monitoring guide
//! publishes a run:
//!
//! `cargo test --release -p rullst-core --lib monitor::overhead_tests -- --ignored --nocapture --test-threads=1`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::collector::{Collector, MAX_ROUTES};
use super::test_support::BASE64_TOKEN;
use super::window::RouteStats;
use super::{Installed, Monitor};
use crate::config::Environment;
use crate::server::dev_telemetry::request_scope::RequestQueries;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request};
use std::time::{Duration, Instant};
use tower::{Service, ServiceExt};

const REQUESTS: u32 = 200_000;

fn application() -> Router {
    Router::new()
        // rullst-access: public — benchmark fixture without data.
        .route("/items/{id}", axum::routing::get(|| async { "item" }))
        .route("/", axum::routing::get(|| async { "home" }))
}

async fn per_request(router: Router, requests: u32) -> Duration {
    let mut service = router.into_service::<Body>();
    let started = Instant::now();
    for index in 0..requests {
        let uri = if index % 2 == 0 { "/items/42" } else { "/" };
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let response = service.ready().await.unwrap().call(request).await.unwrap();
        drop(response);
    }
    started.elapsed() / requests
}

fn installed() -> Installed {
    Installed::prepare(
        Monitor::new().with_token(BASE64_TOKEN),
        Environment::Production,
    )
    .unwrap()
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "timing measurement; run explicitly in release mode"]
async fn measure_recording_overhead_per_request() {
    // Warm both paths, then alternate to share cache and frequency effects.
    let monitor = installed();
    per_request(application(), 20_000).await;
    per_request(monitor.record(application()), 20_000).await;
    let mut plain = Vec::new();
    let mut recorded = Vec::new();
    for _ in 0..5 {
        plain.push(per_request(application(), REQUESTS).await);
        recorded.push(per_request(monitor.record(application()), REQUESTS).await);
    }
    plain.sort();
    recorded.sort();
    let (plain, recorded) = (plain[2], recorded[2]);
    println!(
        "router without recorder: {} ns/request; with recorder: {} ns/request; overhead: {} ns",
        plain.as_nanos(),
        recorded.as_nanos(),
        recorded.as_nanos().saturating_sub(plain.as_nanos())
    );
}

#[test]
#[ignore = "timing measurement; run explicitly in release mode"]
fn measure_collector_record_under_contention() {
    let collector = std::sync::Arc::new(Collector::new());
    let none = RequestQueries::default();
    let started = Instant::now();
    for _ in 0..1_000_000 {
        collector.record(
            &Method::GET,
            Some("/items/{id}"),
            200,
            Duration::from_micros(800),
            &none,
        );
    }
    println!(
        "collector.record, one thread: {} ns/call",
        (started.elapsed() / 1_000_000).as_nanos()
    );

    for threads in [4_usize, 8] {
        let started = Instant::now();
        std::thread::scope(|scope| {
            for thread in 0..threads {
                let collector = std::sync::Arc::clone(&collector);
                scope.spawn(move || {
                    let none = RequestQueries::default();
                    // Half of the threads share one route, the others use their own.
                    let route = if thread % 2 == 0 {
                        "/items/{id}".to_string()
                    } else {
                        format!("/thread/{thread}")
                    };
                    for _ in 0..250_000 {
                        collector.record(
                            &Method::GET,
                            Some(&route),
                            200,
                            Duration::from_micros(800),
                            &none,
                        );
                    }
                });
            }
        });
        let calls = u32::try_from(threads * 250_000).unwrap();
        println!(
            "collector.record, {threads} threads: {} ns/call (wall time / calls)",
            (started.elapsed() / calls).as_nanos()
        );
    }
}

#[test]
#[ignore = "memory report; run explicitly"]
fn report_the_memory_bound() {
    let route = std::mem::size_of::<RouteStats>();
    println!(
        "RouteStats: {route} bytes; {MAX_ROUTES} patterns plus (other): about {} KiB of statistics",
        route * (MAX_ROUTES + 1) / 1024
    );
}
