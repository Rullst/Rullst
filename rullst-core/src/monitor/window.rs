//! Fixed-size request statistics: totals since start and a five-minute
//! window of one-minute slots with a latency histogram.

use serde::Serialize;
use std::time::Duration;

/// Length of one window slot.
pub(super) const SLOT_SECONDS: u64 = 60;
/// Slots in the window: the current minute and the four before it.
pub(super) const SLOTS: usize = 5;
/// Window length reported in the document.
pub(super) const WINDOW_SECONDS: u64 = SLOT_SECONDS * SLOTS as u64;
/// Upper bounds of the latency buckets, in microseconds. A last bucket
/// holds everything slower than 10 s.
pub(super) const BUCKET_BOUNDS_US: [u64; 16] = [
    100, 250, 500, 1_000, 2_500, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000,
    1_000_000, 2_500_000, 5_000_000, 10_000_000,
];
const BUCKETS: usize = BUCKET_BOUNDS_US.len() + 1;

/// Responses per status class: `1xx` to `5xx`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub(super) struct StatusClasses {
    #[serde(rename = "1xx")]
    pub(super) informational: u64,
    #[serde(rename = "2xx")]
    pub(super) success: u64,
    #[serde(rename = "3xx")]
    pub(super) redirection: u64,
    #[serde(rename = "4xx")]
    pub(super) client_error: u64,
    #[serde(rename = "5xx")]
    pub(super) server_error: u64,
}

impl StatusClasses {
    fn count(&mut self, status: u16) {
        let class = match status {
            0..=199 => &mut self.informational,
            200..=299 => &mut self.success,
            300..=399 => &mut self.redirection,
            400..=499 => &mut self.client_error,
            _ => &mut self.server_error,
        };
        *class = class.saturating_add(1);
    }

    pub(super) fn add(&mut self, other: &Self) {
        self.informational = self.informational.saturating_add(other.informational);
        self.success = self.success.saturating_add(other.success);
        self.redirection = self.redirection.saturating_add(other.redirection);
        self.client_error = self.client_error.saturating_add(other.client_error);
        self.server_error = self.server_error.saturating_add(other.server_error);
    }
}

/// A latency histogram over [`BUCKET_BOUNDS_US`] and its maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Histogram {
    buckets: [u64; BUCKETS],
    max_us: u64,
}

impl Default for Histogram {
    fn default() -> Self {
        Self {
            buckets: [0; BUCKETS],
            max_us: 0,
        }
    }
}

impl Histogram {
    fn record(&mut self, micros: u64) {
        let index = BUCKET_BOUNDS_US
            .iter()
            .position(|bound| micros <= *bound)
            .unwrap_or(BUCKET_BOUNDS_US.len());
        if let Some(bucket) = self.buckets.get_mut(index) {
            *bucket = bucket.saturating_add(1);
        }
        self.max_us = self.max_us.max(micros);
    }

    pub(super) fn merge(&mut self, other: &Self) {
        for (bucket, added) in self.buckets.iter_mut().zip(other.buckets) {
            *bucket = bucket.saturating_add(added);
        }
        self.max_us = self.max_us.max(other.max_us);
    }

    fn count(&self) -> u64 {
        self.buckets
            .iter()
            .fold(0_u64, |total, bucket| total.saturating_add(*bucket))
    }

    /// The `quantile` (0 to 1) as the upper bound of the bucket holding it,
    /// never above the observed maximum; `None` without observations.
    pub(super) fn quantile_us(&self, quantile: f64) -> Option<u64> {
        let count = self.count();
        if count == 0 {
            return None;
        }
        // The rank of the observation at `quantile`, from 1.
        let rank = ((quantile * count as f64).ceil() as u64).clamp(1, count);
        let mut seen = 0_u64;
        for (index, bucket) in self.buckets.iter().enumerate() {
            seen = seen.saturating_add(*bucket);
            if seen >= rank {
                let bound = BUCKET_BOUNDS_US.get(index).copied().unwrap_or(u64::MAX);
                return Some(bound.min(self.max_us));
            }
        }
        Some(self.max_us)
    }

    /// p50, p90, p95, p99 and maximum in milliseconds.
    pub(super) fn latency(&self) -> Option<Latency> {
        let millis = |quantile| self.quantile_us(quantile).map(micros_to_millis);
        Some(Latency {
            p50: millis(0.50)?,
            p90: millis(0.90)?,
            p95: millis(0.95)?,
            p99: millis(0.99)?,
            max: micros_to_millis(self.max_us),
        })
    }
}

fn micros_to_millis(micros: u64) -> f64 {
    micros as f64 / 1_000.0
}

/// Latency percentiles in milliseconds; each is the upper bound of its
/// histogram bucket, capped at the maximum.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub(super) struct Latency {
    pub(super) p50: f64,
    pub(super) p90: f64,
    pub(super) p95: f64,
    pub(super) p99: f64,
    pub(super) max: f64,
}

#[derive(Debug, Default, Clone, Copy)]
struct Slot {
    /// Minute since the collector started; `None` until first used.
    minute: Option<u64>,
    requests: u64,
    status: StatusClasses,
    histogram: Histogram,
}

/// What one route (or the whole application) did in the window.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct WindowCounts {
    pub(super) requests: u64,
    pub(super) status: StatusClasses,
    pub(super) histogram: Histogram,
}

impl WindowCounts {
    pub(super) fn add(&mut self, other: &Self) {
        self.requests = self.requests.saturating_add(other.requests);
        self.status.add(&other.status);
        self.histogram.merge(&other.histogram);
    }
}

/// Totals since start and the windowed slots of one route.
#[derive(Debug, Default, Clone)]
pub(super) struct RouteStats {
    pub(super) requests_total: u64,
    pub(super) status_total: StatusClasses,
    slots: [Slot; SLOTS],
}

impl RouteStats {
    /// Records one response `elapsed` after start-up of the collector.
    pub(super) fn record(&mut self, now: Duration, status: u16, latency: Duration) {
        self.requests_total = self.requests_total.saturating_add(1);
        self.status_total.count(status);
        let minute = now.as_secs() / SLOT_SECONDS;
        let index = usize::try_from(minute % SLOTS as u64).unwrap_or_default();
        let Some(slot) = self.slots.get_mut(index) else {
            return;
        };
        if slot.minute != Some(minute) {
            *slot = Slot {
                minute: Some(minute),
                ..Slot::default()
            };
        }
        slot.requests = slot.requests.saturating_add(1);
        slot.status.count(status);
        slot.histogram
            .record(u64::try_from(latency.as_micros()).unwrap_or(u64::MAX));
    }

    /// The slots of the window ending at `now`.
    pub(super) fn window(&self, now: Duration) -> WindowCounts {
        let current = now.as_secs() / SLOT_SECONDS;
        let oldest = current.saturating_sub(SLOTS as u64 - 1);
        let mut counts = WindowCounts::default();
        for slot in &self.slots {
            if slot
                .minute
                .is_some_and(|minute| (oldest..=current).contains(&minute))
            {
                counts.add(&WindowCounts {
                    requests: slot.requests,
                    status: slot.status,
                    histogram: slot.histogram,
                });
            }
        }
        counts
    }
}

/// Seconds the window ending at `now` actually covers: the elapsed part of
/// the current minute plus the four minutes before it, never more than the
/// collector has run, and at least one.
pub(super) fn covered_seconds(now: Duration) -> u64 {
    let elapsed = now.as_secs();
    let covered = (SLOTS as u64 - 1) * SLOT_SECONDS + elapsed % SLOT_SECONDS;
    covered.min(elapsed).max(1)
}
