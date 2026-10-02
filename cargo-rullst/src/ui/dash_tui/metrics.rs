//! Aggregates telemetry snapshots into the dashboard's live metrics.
//!
//! Rates come from the application's exact counters; latency percentiles and
//! the sparkline come from the per-request samples the dashboard observed.
//! Every history is bounded, and nothing is shown that the application did
//! not report.

use super::telemetry::{
    DatabaseReport, PollOutcome, QueueReport, RequestSample, SlowQuery, TelemetrySnapshot,
};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Window of the requests-per-second figure.
pub(super) const RATE_WINDOW: Duration = Duration::from_secs(10);
/// Window of the error rate and the latency percentiles.
pub(super) const LATENCY_WINDOW: Duration = Duration::from_secs(60);
const RATE_SAMPLES: usize = 90;
const LATENCY_SAMPLES: usize = 4_096;
/// Points of the per-poll p95 history (two minutes at one poll per second).
pub(super) const HISTORY_POINTS: usize = 120;
const RECENT_REQUESTS: usize = 50;
const SLOW_QUERIES: usize = 16;
const SPARK_LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Where the dashboard's metrics currently come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    /// No poll has completed yet.
    Waiting,
    Live,
    /// The application answered `404`: telemetry is not enabled.
    NotServed,
    /// The application did not answer (starting, restarting or stopped).
    Unreachable,
    Rejected(&'static str),
}

/// What a newly ingested poll changed that deserves a system-log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Notice {
    Connected,
    Restarted,
    Lost(Source),
}

#[derive(Debug)]
struct Baseline {
    at: Instant,
    generation: String,
    requests: u64,
    server_errors: u64,
    last_request_seq: u64,
    last_slow_seq: u64,
}

#[derive(Debug, Clone, Copy)]
struct RateSample {
    at: Instant,
    elapsed: Duration,
    requests: u64,
    server_errors: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Percentiles {
    pub p50_us: u64,
    pub p95_us: u64,
    pub samples: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ErrorRate {
    pub server_errors: u64,
    pub requests: u64,
    pub percent: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Totals {
    pub requests: u64,
    pub client_errors: u64,
    pub server_errors: u64,
}

#[derive(Debug)]
pub(super) struct Metrics {
    pub source: Source,
    pub totals: Option<Totals>,
    pub database: Option<DatabaseReport>,
    pub queue: Option<QueueReport>,
    baseline: Option<Baseline>,
    rates: VecDeque<RateSample>,
    latencies: VecDeque<(Instant, u64)>,
    history: VecDeque<Option<u64>>,
    recent: VecDeque<RequestSample>,
    slow: VecDeque<SlowQuery>,
    sampled_at: Option<Instant>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            source: Source::Waiting,
            totals: None,
            database: None,
            queue: None,
            baseline: None,
            rates: VecDeque::new(),
            latencies: VecDeque::new(),
            history: VecDeque::new(),
            recent: VecDeque::new(),
            slow: VecDeque::new(),
            sampled_at: None,
        }
    }

    /// Whether any telemetry snapshot has been received.
    pub fn has_data(&self) -> bool {
        self.totals.is_some()
    }

    pub fn ingest(&mut self, outcome: PollOutcome, now: Instant) -> Option<Notice> {
        let previous = self.source;
        let snapshot = match outcome {
            PollOutcome::Snapshot(snapshot) => snapshot,
            other => {
                self.source = match other {
                    PollOutcome::NotServed => Source::NotServed,
                    PollOutcome::Rejected(reason) => Source::Rejected(reason),
                    _ => Source::Unreachable,
                };
                push_bounded(&mut self.history, None, HISTORY_POINTS);
                // Startup polls before the port opens are expected silence.
                let notable = previous == Source::Live || self.source != Source::Unreachable;
                return (previous != self.source && notable).then_some(Notice::Lost(self.source));
            }
        };
        self.source = Source::Live;
        let restarted = self.baseline.as_ref().is_some_and(|baseline| {
            baseline.generation != snapshot.generation
                || snapshot.http.requests_total < baseline.requests
        });
        if restarted {
            // Per-process figures restart; the p95 timeline and the request
            // list keep the previous process's entries.
            self.baseline = None;
            self.rates.clear();
            self.latencies.clear();
            self.slow.clear();
            self.sampled_at = None;
        }
        let new_requests = self.observe(&snapshot, now);
        let durations = new_requests
            .iter()
            .map(|sample| sample.duration_us)
            .collect::<Vec<_>>();
        push_bounded(
            &mut self.history,
            percentile_of(&durations, 95),
            HISTORY_POINTS,
        );
        for duration in durations {
            push_bounded(&mut self.latencies, (now, duration), LATENCY_SAMPLES);
        }
        for sample in new_requests {
            push_bounded(&mut self.recent, sample, RECENT_REQUESTS);
        }
        self.totals = Some(Totals {
            requests: snapshot.http.requests_total,
            client_errors: snapshot.http.client_errors_total,
            server_errors: snapshot.http.server_errors_total,
        });
        self.database = Some(snapshot.database);
        self.queue = Some(snapshot.queue);
        if restarted {
            Some(Notice::Restarted)
        } else {
            (previous != Source::Live).then_some(Notice::Connected)
        }
    }

    /// Updates the baseline and rate window; returns the requests not seen yet.
    fn observe(&mut self, snapshot: &TelemetrySnapshot, now: Instant) -> Vec<RequestSample> {
        let http = &snapshot.http;
        let (last_request_seq, last_slow_seq) = match &self.baseline {
            Some(baseline) => {
                self.rates.push_back(RateSample {
                    at: now,
                    elapsed: now.saturating_duration_since(baseline.at),
                    requests: http.requests_total.saturating_sub(baseline.requests),
                    server_errors: http
                        .server_errors_total
                        .saturating_sub(baseline.server_errors),
                });
                while self.rates.len() > RATE_SAMPLES {
                    self.rates.pop_front();
                }
                (baseline.last_request_seq, baseline.last_slow_seq)
            }
            // The first poll of a process covers every request it served.
            None => (0, 0),
        };
        let expected = http.requests_total.saturating_sub(last_request_seq);
        let observed = http
            .recent
            .iter()
            .filter(|sample| sample.seq > last_request_seq)
            .count() as u64;
        if observed < expected {
            self.sampled_at = Some(now);
        }
        let new_requests = http
            .recent
            .iter()
            .filter(|sample| sample.seq > last_request_seq)
            .cloned()
            .collect::<Vec<_>>();
        let mut newest_slow = last_slow_seq;
        if let DatabaseReport::Observed { recent_slow, .. } = &snapshot.database {
            for slow in recent_slow.iter().filter(|slow| slow.seq > last_slow_seq) {
                newest_slow = newest_slow.max(slow.seq);
                push_bounded(&mut self.slow, slow.clone(), SLOW_QUERIES);
            }
        }
        self.baseline = Some(Baseline {
            at: now,
            generation: snapshot.generation.clone(),
            requests: http.requests_total,
            server_errors: http.server_errors_total,
            last_request_seq: http.requests_total,
            last_slow_seq: newest_slow,
        });
        new_requests
    }

    /// Requests per second over [`RATE_WINDOW`], from exact counters.
    pub fn requests_per_second(&self, now: Instant) -> Option<f64> {
        let (requests, _, elapsed) = self.window(now, RATE_WINDOW)?;
        Some(requests as f64 / elapsed.as_secs_f64())
    }

    /// Server errors (5xx) as a share of requests over [`LATENCY_WINDOW`].
    pub fn error_rate(&self, now: Instant) -> Option<ErrorRate> {
        let (requests, server_errors, _) = self.window(now, LATENCY_WINDOW)?;
        let percent = if requests == 0 {
            0.0
        } else {
            server_errors as f64 * 100.0 / requests as f64
        };
        Some(ErrorRate {
            server_errors,
            requests,
            percent,
        })
    }

    fn window(&self, now: Instant, window: Duration) -> Option<(u64, u64, Duration)> {
        let mut requests = 0_u64;
        let mut errors = 0_u64;
        let mut elapsed = Duration::ZERO;
        for sample in self
            .rates
            .iter()
            .rev()
            .take_while(|sample| now.saturating_duration_since(sample.at) <= window)
        {
            requests = requests.saturating_add(sample.requests);
            errors = errors.saturating_add(sample.server_errors);
            elapsed = elapsed.saturating_add(sample.elapsed);
        }
        (!elapsed.is_zero()).then_some((requests, errors, elapsed))
    }

    /// p50/p95 of the request durations observed in [`LATENCY_WINDOW`].
    pub fn latency(&self, now: Instant) -> Option<Percentiles> {
        let mut durations = self
            .latencies
            .iter()
            .filter(|(at, _)| now.saturating_duration_since(*at) <= LATENCY_WINDOW)
            .map(|(_, duration)| *duration)
            .collect::<Vec<_>>();
        durations.sort_unstable();
        Some(Percentiles {
            p50_us: percentile(&durations, 50)?,
            p95_us: percentile(&durations, 95)?,
            samples: durations.len(),
        })
    }

    /// Whether more requests arrived in [`LATENCY_WINDOW`] than the
    /// application's recent list could return, so percentiles are sampled.
    pub fn sampled(&self, now: Instant) -> bool {
        self.sampled_at
            .is_some_and(|at| now.saturating_duration_since(at) <= LATENCY_WINDOW)
    }

    /// The p95 history as `rows` lines of `width` characters, top line first.
    pub fn sparkline(&self, width: usize, rows: usize) -> Vec<String> {
        let points = self.history.iter().copied().collect::<Vec<_>>();
        sparkline_rows(&points, width, rows)
    }

    /// Observed requests, newest first.
    pub fn recent(&self) -> impl Iterator<Item = &RequestSample> {
        self.recent.iter().rev()
    }

    /// Observed slow ORM operations, newest first.
    pub fn slow_queries(&self) -> impl Iterator<Item = &SlowQuery> {
        self.slow.iter().rev()
    }
}

fn push_bounded<T>(entries: &mut VecDeque<T>, entry: T, capacity: usize) {
    while entries.len() >= capacity {
        entries.pop_front();
    }
    entries.push_back(entry);
}

/// Nearest-rank percentile of an ascending slice.
pub(super) fn percentile(sorted: &[u64], percentile: u32) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let percentile = usize::try_from(percentile.clamp(1, 100)).unwrap_or(100);
    let rank = (percentile * sorted.len()).div_ceil(100);
    sorted.get(rank.saturating_sub(1)).copied()
}

fn percentile_of(values: &[u64], rank: u32) -> Option<u64> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    percentile(&sorted, rank)
}

/// One-line [`sparkline_rows`].
#[cfg(test)]
pub(super) fn sparkline(points: &[Option<u64>], width: usize) -> String {
    sparkline_rows(points, width, 1).concat()
}

/// Scales the newest `width` points against their maximum over `rows`
/// stacked lines (eight levels per line), top line first: the largest point
/// fills every line, zero only the bottom `▁`, and a point without requests
/// is blank. Older points are dropped; a short history is right-aligned.
pub(super) fn sparkline_rows(points: &[Option<u64>], width: usize, rows: usize) -> Vec<String> {
    let rows = rows.max(1);
    let visible = &points[points.len().saturating_sub(width)..];
    let max = u128::from(visible.iter().flatten().copied().max().unwrap_or(0));
    let steps = SPARK_LEVELS.len() as u128;
    let top = steps * rows as u128;
    let levels = visible
        .iter()
        .map(|point| {
            point.map(|value| match max {
                0 => 1,
                max => (u128::from(value) * (top - 1) + max / 2) / max + 1,
            })
        })
        .collect::<Vec<_>>();
    (0..rows)
        .map(|row| {
            let below = (rows - 1 - row) as u128 * steps;
            let mut line = " ".repeat(width - visible.len());
            for level in &levels {
                let filled = level.map_or(0, |level| level.saturating_sub(below).min(steps));
                line.push(match usize::try_from(filled).unwrap_or(0) {
                    0 => ' ',
                    filled => SPARK_LEVELS[filled - 1],
                });
            }
            line
        })
        .collect()
}

/// `850 µs`, `12.4 ms`, `152 ms`, `1.52 s`.
pub(super) fn format_duration(micros: u64) -> String {
    match micros {
        0..=999 => format!("{micros} µs"),
        1_000..=99_999 => format!("{:.1} ms", micros as f64 / 1_000.0),
        100_000..=999_999 => format!("{} ms", micros / 1_000),
        _ => format!("{:.2} s", micros as f64 / 1_000_000.0),
    }
}
