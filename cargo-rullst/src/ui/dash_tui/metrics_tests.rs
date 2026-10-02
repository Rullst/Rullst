#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::metrics::{
    HISTORY_POINTS, Metrics, Notice, Source, format_duration, percentile, sparkline, sparkline_rows,
};
use super::telemetry::{
    DatabaseReport, HttpReport, PollOutcome, QueueReport, RequestSample, SlowQuery,
    TelemetrySnapshot,
};
use std::time::{Duration, Instant};

const FIRST: &str = "0123456789abcdef0123456789abcdef";
const SECOND: &str = "fedcba9876543210fedcba9876543210";

fn sample(seq: u64, status: u16, duration_us: u64) -> RequestSample {
    RequestSample {
        seq,
        method: "GET".to_string(),
        path: format!("/r/{seq}"),
        status,
        duration_us,
    }
}

/// A snapshot whose recent list holds the newest `limit` of `total` requests.
fn snapshot(generation: &str, total: u64, server_errors: u64, limit: u64) -> PollOutcome {
    let recent = (total.saturating_sub(limit) + 1..=total)
        .map(|seq| {
            sample(
                seq,
                if seq <= server_errors { 500 } else { 200 },
                seq * 1_000,
            )
        })
        .collect();
    PollOutcome::Snapshot(TelemetrySnapshot {
        generation: generation.to_string(),
        http: HttpReport {
            requests_total: total,
            client_errors_total: 0,
            server_errors_total: server_errors,
            recent,
        },
        database: DatabaseReport::Observed {
            queries_total: total * 2,
            slow_total: 1,
            slow_threshold_ms: 100,
            recent_slow: vec![SlowQuery {
                seq: 1,
                operation: "select_many".to_string(),
                model: Some("Post".to_string()),
                table: None,
                duration_us: 150_000,
            }],
        },
        queue: QueueReport::Observed { pending: 2 },
    })
}

#[test]
fn nearest_rank_percentiles_match_their_definition() {
    assert_eq!(percentile(&[], 50), None);
    assert_eq!(percentile(&[7], 50), Some(7));
    assert_eq!(percentile(&[7], 95), Some(7));
    assert_eq!(percentile(&[1, 2, 3, 4], 50), Some(2));
    assert_eq!(percentile(&[1, 2, 3, 4], 95), Some(4));
    let hundred = (1..=100).collect::<Vec<u64>>();
    assert_eq!(percentile(&hundred, 50), Some(50));
    assert_eq!(percentile(&hundred, 95), Some(95));
    assert_eq!(percentile(&hundred, 100), Some(100));
    assert_eq!(percentile(&hundred, 0), Some(1));
    assert_eq!(percentile(&hundred, 500), Some(100));
}

#[test]
fn sparklines_scale_to_the_visible_maximum_and_keep_gaps() {
    assert_eq!(sparkline(&[], 4), "    ");
    assert_eq!(sparkline(&[Some(0), Some(0)], 2), "▁▁");
    assert_eq!(sparkline(&[Some(1), Some(4), Some(8)], 3), "▂▅█");
    assert_eq!(sparkline(&[Some(8), None, Some(4)], 3), "█ ▅");
    // Only the newest `width` points are drawn and they set the scale.
    assert_eq!(sparkline(&[Some(1_000), Some(2), Some(4)], 2), "▅█");
    // Scaling cannot overflow at the extremes.
    assert_eq!(sparkline(&[Some(u64::MAX), Some(0)], 2), "█▁");
    // A short history is right-aligned.
    assert_eq!(sparkline(&[Some(3)], 3), "  █");
}

#[test]
fn stacked_sparklines_split_sixteen_levels_over_two_lines() {
    let points = [Some(0), Some(4), Some(8), Some(16), None];
    // Levels 1, 5, 9 and 16 of 16: the top line holds what exceeds eight.
    assert_eq!(sparkline_rows(&points, 5, 2), ["  ▁█ ", "▁▅██ "]);
    assert_eq!(sparkline_rows(&points, 5, 0), [sparkline(&points, 5)]);
}

#[test]
fn durations_are_formatted_with_stable_units() {
    assert_eq!(format_duration(850), "850 µs");
    assert_eq!(format_duration(12_400), "12.4 ms");
    assert_eq!(format_duration(152_400), "152 ms");
    assert_eq!(format_duration(1_520_000), "1.52 s");
}

#[test]
fn rates_and_error_rates_come_from_counter_deltas() {
    let start = Instant::now();
    let mut metrics = Metrics::new();
    assert_eq!(
        metrics.ingest(snapshot(FIRST, 10, 0, 10), start),
        Some(Notice::Connected)
    );
    // One snapshot is only a baseline.
    assert_eq!(metrics.requests_per_second(start), None);
    assert_eq!(metrics.error_rate(start), None);

    metrics.ingest(snapshot(FIRST, 30, 2, 20), start + Duration::from_secs(1));
    metrics.ingest(snapshot(FIRST, 30, 2, 20), start + Duration::from_secs(2));
    let now = start + Duration::from_secs(2);
    assert_eq!(metrics.requests_per_second(now), Some(10.0));
    let errors = metrics.error_rate(now).unwrap();
    assert_eq!((errors.server_errors, errors.requests), (2, 20));
    assert!((errors.percent - 10.0).abs() < f64::EPSILON);
    assert_eq!(metrics.totals.unwrap().requests, 30);

    // Old samples leave the 10 s window but stay in the 60 s window.
    let later = start + Duration::from_secs(13);
    assert_eq!(metrics.requests_per_second(later), None);
    assert_eq!(metrics.error_rate(later).unwrap().requests, 20);
    assert_eq!(metrics.error_rate(start + Duration::from_secs(70)), None);
}

#[test]
fn latency_percentiles_use_each_observed_request_once() {
    let start = Instant::now();
    let mut metrics = Metrics::new();
    metrics.ingest(snapshot(FIRST, 4, 0, 4), start);
    // The same requests again are not counted twice.
    metrics.ingest(snapshot(FIRST, 4, 0, 4), start + Duration::from_secs(1));
    let latency = metrics.latency(start + Duration::from_secs(1)).unwrap();
    assert_eq!(latency.samples, 4);
    assert_eq!((latency.p50_us, latency.p95_us), (2_000, 4_000));
    assert!(!metrics.sampled(start));
    assert_eq!(
        metrics.recent().map(|s| s.seq).collect::<Vec<_>>(),
        [4, 3, 2, 1]
    );
    assert!(metrics.latency(start + Duration::from_secs(62)).is_none());
}

#[test]
fn a_burst_beyond_the_recent_list_is_marked_as_sampled() {
    let start = Instant::now();
    let mut metrics = Metrics::new();
    metrics.ingest(snapshot(FIRST, 1, 0, 1), start);
    metrics.ingest(snapshot(FIRST, 101, 0, 64), start + Duration::from_secs(1));
    let now = start + Duration::from_secs(1);
    assert!(metrics.sampled(now));
    assert_eq!(metrics.latency(now).unwrap().samples, 65);
    assert_eq!(metrics.requests_per_second(now), Some(100.0));
    assert!(!metrics.sampled(start + Duration::from_secs(70)));
}

#[test]
fn a_new_generation_restarts_baselines_without_negative_rates() {
    let start = Instant::now();
    let mut metrics = Metrics::new();
    metrics.ingest(snapshot(FIRST, 50, 0, 10), start);
    metrics.ingest(snapshot(FIRST, 60, 0, 10), start + Duration::from_secs(1));
    assert_eq!(
        metrics.ingest(snapshot(SECOND, 3, 0, 3), start + Duration::from_secs(2)),
        Some(Notice::Restarted)
    );
    // The previous process's window is gone; the new one starts as a baseline.
    assert_eq!(
        metrics.requests_per_second(start + Duration::from_secs(2)),
        None
    );
    assert_eq!(metrics.totals.unwrap().requests, 3);
    assert_eq!(metrics.recent().next().unwrap().seq, 3);
    // Latency and slow operations describe the new process only.
    let latency = metrics.latency(start + Duration::from_secs(2)).unwrap();
    assert_eq!(latency.samples, 3);
    assert_eq!(metrics.slow_queries().count(), 1);
    assert!(
        metrics.recent().count() > 3,
        "the request list keeps earlier entries"
    );
    metrics.ingest(snapshot(SECOND, 5, 0, 5), start + Duration::from_secs(3));
    assert_eq!(
        metrics.requests_per_second(start + Duration::from_secs(3)),
        Some(2.0)
    );
    // A counter that goes backwards in the same generation also resets.
    assert_eq!(
        metrics.ingest(snapshot(SECOND, 1, 0, 1), start + Duration::from_secs(4)),
        Some(Notice::Restarted)
    );
}

#[test]
fn connection_changes_are_reported_once_and_data_is_kept() {
    let start = Instant::now();
    let mut metrics = Metrics::new();
    // Startup silence before the port opens is not news.
    assert_eq!(metrics.ingest(PollOutcome::Unreachable, start), None);
    assert_eq!(metrics.source, Source::Unreachable);
    assert!(!metrics.has_data());
    assert_eq!(
        metrics.ingest(PollOutcome::NotServed, start),
        Some(Notice::Lost(Source::NotServed))
    );
    assert_eq!(metrics.ingest(PollOutcome::NotServed, start), None);
    assert_eq!(
        metrics.ingest(snapshot(FIRST, 1, 0, 1), start),
        Some(Notice::Connected)
    );
    assert_eq!(metrics.ingest(snapshot(FIRST, 1, 0, 1), start), None);
    assert_eq!(
        metrics.ingest(PollOutcome::Unreachable, start),
        Some(Notice::Lost(Source::Unreachable))
    );
    assert_eq!(metrics.ingest(PollOutcome::Unreachable, start), None);
    assert!(metrics.has_data());
    assert_eq!(
        metrics.ingest(PollOutcome::Rejected("malformed JSON"), start),
        Some(Notice::Lost(Source::Rejected("malformed JSON")))
    );
    assert_eq!(metrics.queue, Some(QueueReport::Observed { pending: 2 }));
}

#[test]
fn every_history_stays_bounded() {
    let start = Instant::now();
    let mut metrics = Metrics::new();
    for second in 0..400_u64 {
        let total = (second + 1) * 70;
        metrics.ingest(
            snapshot(FIRST, total, 0, 64),
            start + Duration::from_secs(second),
        );
        metrics.ingest(
            PollOutcome::Unreachable,
            start + Duration::from_secs(second),
        );
    }
    let now = start + Duration::from_secs(399);
    assert_eq!(metrics.recent().count(), 50);
    assert_eq!(metrics.slow_queries().count(), 1);
    let line = metrics.sparkline(HISTORY_POINTS * 2, 1).concat();
    assert_eq!(line.chars().count(), HISTORY_POINTS * 2);
    assert!(line.starts_with(&" ".repeat(HISTORY_POINTS)));
    // 60 s of samples at 64 observed requests per poll.
    assert!(metrics.latency(now).unwrap().samples <= 4_096);
    assert_eq!(metrics.requests_per_second(now), Some(70.0));
}
