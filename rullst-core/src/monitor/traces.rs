//! Sampled summaries of recent requests, for the version 2 document.
//!
//! A summary holds only what the statistics already key on: the normalized
//! method, the matched route pattern, the status, the duration and the ORM
//! operations the request started. At most one ordinary request is kept every
//! [`SAMPLE_INTERVAL`], and at most one outlier (a server error or a request
//! slower than [`SLOW_REQUEST`]) every [`OUTLIER_INTERVAL`], so a burst takes
//! the buffer's lock a bounded number of times per second. The newest
//! [`MAX_TRACES`] summaries are kept.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// Summaries kept, newest first.
pub(super) const MAX_TRACES: usize = 50;
/// Shortest gap between two kept ordinary requests.
pub(super) const SAMPLE_INTERVAL: Duration = Duration::from_millis(100);
/// Shortest gap between two kept outliers.
pub(super) const OUTLIER_INTERVAL: Duration = Duration::from_millis(10);
/// Requests at least this slow are outliers.
pub(super) const SLOW_REQUEST: Duration = Duration::from_secs(1);

/// One kept request.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RequestTrace {
    /// Unix time the response was recorded, in seconds.
    pub(super) at: u64,
    pub(super) method: &'static str,
    pub(super) route: Arc<str>,
    pub(super) status: u16,
    pub(super) latency: Duration,
    /// Outermost ORM operations the request started.
    pub(super) db_operations: u64,
    /// Whether the request repeated an ORM operation at least the N+1
    /// threshold.
    pub(super) n_plus_one: bool,
}

/// Whether a response is an outlier: a server error or a slow request.
pub(super) fn is_outlier(status: u16, latency: Duration) -> bool {
    status >= 500 || latency >= SLOW_REQUEST
}

/// The sampled buffer.
#[derive(Debug, Default)]
pub(super) struct Sampler {
    /// Earliest time, in microseconds since the collector started, at which
    /// the next ordinary request may be kept.
    next_regular: AtomicU64,
    /// The same for outliers.
    next_outlier: AtomicU64,
    kept_total: AtomicU64,
    recent: Mutex<VecDeque<RequestTrace>>,
}

fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

impl Sampler {
    /// Whether a request recorded at `now` (since the collector started) is
    /// kept. Concurrent requests race for the same slot; one wins.
    pub(super) fn admit(&self, now: Duration, outlier: bool) -> bool {
        let (gate, interval) = if outlier {
            (&self.next_outlier, OUTLIER_INTERVAL)
        } else {
            (&self.next_regular, SAMPLE_INTERVAL)
        };
        let now = micros(now);
        let next = gate.load(Ordering::Relaxed);
        now >= next
            && gate
                .compare_exchange(
                    next,
                    now.saturating_add(micros(interval)),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
    }

    /// Keeps `trace`, dropping the oldest beyond [`MAX_TRACES`].
    pub(super) fn keep(&self, trace: RequestTrace) {
        self.kept_total.fetch_add(1, Ordering::Relaxed);
        let mut recent = self.recent.lock().unwrap_or_else(PoisonError::into_inner);
        recent.push_front(trace);
        recent.truncate(MAX_TRACES);
    }

    /// Summaries kept since start, and the kept ones, newest first.
    pub(super) fn snapshot(&self) -> (u64, Vec<RequestTrace>) {
        let recent = self.recent.lock().unwrap_or_else(PoisonError::into_inner);
        (
            self.kept_total.load(Ordering::Relaxed),
            recent.iter().cloned().collect(),
        )
    }
}
