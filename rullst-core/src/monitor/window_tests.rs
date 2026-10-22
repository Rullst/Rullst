#![allow(clippy::unwrap_used, clippy::float_cmp)]

use super::window::{Histogram, Latency, RouteStats, StatusClasses, covered_seconds};
use std::time::Duration;

fn seconds(value: u64) -> Duration {
    Duration::from_secs(value)
}

fn micros(value: u64) -> Duration {
    Duration::from_micros(value)
}

/// Records `latencies` (microseconds) in one minute and returns the window.
fn histogram(latencies: &[u64]) -> Histogram {
    let mut stats = RouteStats::default();
    for latency in latencies {
        stats.record(seconds(1), 200, micros(*latency));
    }
    stats.window(seconds(1)).histogram
}

#[test]
fn status_codes_are_counted_by_class() {
    let mut stats = RouteStats::default();
    for status in [100, 199, 200, 299, 300, 399, 400, 499, 500, 599, 600] {
        stats.record(seconds(0), status, micros(1));
    }
    assert_eq!(
        stats.status_total,
        StatusClasses {
            informational: 2,
            success: 2,
            redirection: 2,
            client_error: 2,
            server_error: 3,
        }
    );
    assert_eq!(stats.requests_total, 11);
}

#[test]
fn status_classes_add_field_by_field() {
    let mut total = StatusClasses {
        informational: 1,
        success: 2,
        redirection: 3,
        client_error: 4,
        server_error: 5,
    };
    total.add(&StatusClasses {
        informational: 10,
        success: 20,
        redirection: 30,
        client_error: 40,
        server_error: 50,
    });
    assert_eq!(
        total,
        StatusClasses {
            informational: 11,
            success: 22,
            redirection: 33,
            client_error: 44,
            server_error: 55,
        }
    );
}

#[test]
fn a_percentile_is_its_bucket_bound_capped_at_the_maximum() {
    assert_eq!(histogram(&[]).quantile_us(0.5), None);
    assert_eq!(histogram(&[]).latency(), None);
    // 3 ms falls in the 5 ms bucket; the maximum caps the estimate.
    assert_eq!(histogram(&[3_000]).quantile_us(0.5), Some(3_000));

    let mut latencies = vec![1_000; 90];
    latencies.extend([100_000; 9]);
    latencies.push(20_000_000);
    let observed = histogram(&latencies);
    assert_eq!(observed.quantile_us(0.50), Some(1_000));
    assert_eq!(observed.quantile_us(0.90), Some(1_000));
    assert_eq!(observed.quantile_us(0.91), Some(100_000));
    assert_eq!(observed.quantile_us(0.99), Some(100_000));
    // Slower than the last bound: reported as the maximum.
    assert_eq!(observed.quantile_us(1.0), Some(20_000_000));
    assert_eq!(
        observed.latency(),
        Some(Latency {
            p50: 1.0,
            p90: 1.0,
            p95: 100.0,
            p99: 100.0,
            max: 20_000.0,
        })
    );
}

#[test]
fn bucket_bounds_are_inclusive() {
    let at = |latency: u64| histogram(&[latency, 1_000_000_000]).quantile_us(0.5);
    assert_eq!(at(100), Some(100));
    assert_eq!(at(101), Some(250));
    assert_eq!(at(1_000), Some(1_000));
    assert_eq!(at(1_001), Some(2_500));
    assert_eq!(at(10_000_000), Some(10_000_000));
    // The open last bucket reports the maximum.
    assert_eq!(at(10_000_001), Some(1_000_000_000));
    assert_eq!(histogram(&[0]).quantile_us(0.5), Some(0));
}

#[test]
fn histograms_merge_buckets_and_maxima() {
    let mut merged = histogram(&[1_000, 1_000]);
    merged.merge(&histogram(&[50_000, 70_000]));
    assert_eq!(merged.quantile_us(0.5), Some(1_000));
    // 50 ms is the bound of its own bucket; 70 ms falls in the 100 ms one.
    assert_eq!(merged.quantile_us(0.75), Some(50_000));
    assert_eq!(merged.quantile_us(1.0), Some(70_000));
    assert_eq!(merged.latency().unwrap().max, 70.0);
}

#[test]
fn the_window_holds_the_current_minute_and_the_four_before_it() {
    let mut stats = RouteStats::default();
    stats.record(seconds(0), 200, micros(10));
    stats.record(seconds(61), 404, micros(10));
    stats.record(seconds(125), 500, micros(10));
    stats.record(seconds(250), 200, micros(10));

    let window = stats.window(seconds(250));
    assert_eq!(window.requests, 4);
    assert_eq!(window.status.success, 2);
    assert_eq!(window.status.client_error, 1);
    assert_eq!(window.status.server_error, 1);

    // Minute 5: minute 0 has left the window.
    let window = stats.window(seconds(300));
    assert_eq!(window.requests, 3);
    assert_eq!(window.status.success, 1);
    // Minute 8: only minute 4 remains.
    assert_eq!(stats.window(seconds(480)).requests, 1);
    assert_eq!(stats.window(seconds(540)).requests, 0);
    assert_eq!(stats.requests_total, 4);
}

#[test]
fn a_reused_slot_starts_empty() {
    let mut stats = RouteStats::default();
    stats.record(seconds(10), 500, micros(5_000_000));
    // Minute 5 reuses minute 0's slot.
    stats.record(seconds(310), 200, micros(1_000));
    let window = stats.window(seconds(310));
    assert_eq!(window.requests, 1);
    assert_eq!(window.status.server_error, 0);
    assert_eq!(window.histogram.latency().unwrap().max, 1.0);
    assert_eq!(stats.status_total.server_error, 1);
}

#[test]
fn the_covered_time_grows_to_the_window_and_follows_the_current_minute() {
    assert_eq!(covered_seconds(seconds(0)), 1);
    assert_eq!(covered_seconds(seconds(30)), 30);
    assert_eq!(covered_seconds(seconds(250)), 250);
    assert_eq!(covered_seconds(seconds(299)), 299);
    assert_eq!(covered_seconds(seconds(300)), 240);
    assert_eq!(covered_seconds(seconds(359)), 299);
    assert_eq!(covered_seconds(seconds(3_600)), 240);
}
