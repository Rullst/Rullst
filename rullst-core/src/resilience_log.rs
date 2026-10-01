//! Rate-limited diagnostics for the Traffic Shield request path.
//!
//! Under overload the shield rejects many requests per second. Writing one
//! synchronous stderr line for each would block Tokio workers on a full log
//! pipe and prolong the overload, so at most one line per interval is written
//! and it reports how many similar events were suppressed.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Shortest time between two Traffic Shield diagnostic lines of one kind.
pub(super) const LOG_INTERVAL: Duration = Duration::from_secs(1);

/// Admits one event per interval and counts the ones it suppresses.
pub(super) struct LogThrottle {
    origin: Instant,
    /// Milliseconds after `origin` before which events are suppressed.
    next_allowed_ms: AtomicU64,
    suppressed: AtomicU64,
}

impl LogThrottle {
    pub(super) fn new() -> Self {
        Self {
            origin: Instant::now(),
            next_allowed_ms: AtomicU64::new(0),
            suppressed: AtomicU64::new(0),
        }
    }

    /// Returns the number of events suppressed since the previous admitted
    /// one when an event at `now` may be logged, or `None` (and counts it)
    /// while the interval has not elapsed.
    pub(super) fn admit(&self, now: Instant, interval: Duration) -> Option<u64> {
        let now_ms = millis(now.saturating_duration_since(self.origin));
        let next_allowed = self.next_allowed_ms.load(Ordering::Acquire);
        let claimed = now_ms >= next_allowed
            && self
                .next_allowed_ms
                .compare_exchange(
                    next_allowed,
                    now_ms.saturating_add(millis(interval)),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok();
        if claimed {
            Some(self.suppressed.swap(0, Ordering::AcqRel))
        } else {
            self.suppressed.fetch_add(1, Ordering::AcqRel);
            None
        }
    }

    #[cfg(test)]
    pub(super) fn suppressed(&self) -> u64 {
        self.suppressed.load(Ordering::Acquire)
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_event_per_interval_is_admitted_with_the_suppressed_count() {
        let throttle = LogThrottle::new();
        let start = Instant::now();
        let interval = Duration::from_secs(1);

        assert_eq!(throttle.admit(start, interval), Some(0));
        for offset in [1, 200, 900] {
            assert_eq!(
                throttle.admit(start + Duration::from_millis(offset), interval),
                None
            );
        }
        assert_eq!(
            throttle.admit(start + Duration::from_millis(1_100), interval),
            Some(3)
        );
        assert_eq!(
            throttle.admit(start + Duration::from_millis(1_200), interval),
            None
        );
    }
}
