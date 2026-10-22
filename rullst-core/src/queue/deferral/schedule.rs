//! Deferral of recurring scheduler tasks by time window.

use super::window::TimeWindow;
use super::{DeferralError, DeferralReason, MAX_DEFERRAL_WINDOWS, planner};
use crate::queue::MAX_SCHEDULE_DELAY;
use std::time::Duration;

/// Lets a scheduled task start up to `max_delay` after its cron tick, at the
/// start of the next allowed window. Added in v13.
///
/// Each tick is placed by the time-window and deadline rules of
/// [`super::Deferral`] with the tick as "now" and `tick + max_delay` as the
/// deadline. Intensity forecasts are not consulted for scheduled tasks; a
/// task can enqueue a job with [`crate::queue::Queue::dispatch_deferred_with`]
/// instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleDeferral {
    max_delay: Duration,
    windows: Vec<TimeWindow>,
}

impl ScheduleDeferral {
    /// Allows each run to start at most `max_delay` after its tick.
    ///
    /// # Errors
    /// Returns [`DeferralError::DeadlineTooFar`] beyond 366 days.
    pub fn within(max_delay: Duration) -> Result<Self, DeferralError> {
        if max_delay > MAX_SCHEDULE_DELAY {
            return Err(DeferralError::DeadlineTooFar);
        }
        Ok(Self {
            max_delay,
            windows: Vec::new(),
        })
    }

    /// Adds an allowed window.
    ///
    /// # Errors
    /// Returns [`DeferralError::TooManyWindows`] beyond
    /// [`MAX_DEFERRAL_WINDOWS`].
    pub fn window(mut self, window: TimeWindow) -> Result<Self, DeferralError> {
        if self.windows.len() >= MAX_DEFERRAL_WINDOWS {
            return Err(DeferralError::TooManyWindows);
        }
        self.windows.push(window);
        Ok(self)
    }

    /// The largest delay after a tick.
    pub fn max_delay(&self) -> Duration {
        self.max_delay
    }

    /// The start, in Unix milliseconds, of the run due at `tick_ms`.
    pub(crate) fn start_for(&self, tick_ms: i64) -> (i64, DeferralReason) {
        planner::window_rule(&self.windows, tick_ms, self.deadline_ms(tick_ms))
    }

    /// Like [`Self::start_for`], with the windows' times read as wall times
    /// of a time zone (see `TimeWindow::zoned_intervals`).
    #[cfg(feature = "scheduler-tz")]
    pub(crate) fn start_for_zoned(
        &self,
        tick_ms: i64,
        local_of: impl Fn(i64) -> Option<i64>,
        instant_of: impl Fn(i64) -> Option<i64>,
    ) -> (i64, DeferralReason) {
        let run_by_ms = self.deadline_ms(tick_ms);
        let intervals: Vec<(i64, i64)> = if self.windows.is_empty() {
            vec![(tick_ms, run_by_ms)]
        } else {
            self.windows
                .iter()
                .flat_map(|window| {
                    window.zoned_intervals(tick_ms, run_by_ms, &local_of, &instant_of)
                })
                .collect()
        };
        planner::earliest_start(&intervals, tick_ms, run_by_ms)
    }

    /// Whether a window sets a UTC offset.
    #[cfg(feature = "scheduler-tz")]
    pub(crate) fn has_utc_offset(&self) -> bool {
        self.windows
            .iter()
            .any(|window| window.utc_offset_minutes() != 0)
    }

    fn deadline_ms(&self, tick_ms: i64) -> i64 {
        let delay = i64::try_from(self.max_delay.as_millis()).unwrap_or(i64::MAX);
        tick_ms.saturating_add(delay)
    }
}
