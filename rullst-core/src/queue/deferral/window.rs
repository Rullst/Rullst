//! Daily time windows and the millisecond arithmetic behind them.
//!
//! All arithmetic uses Unix milliseconds in `i64`, so a window that wraps
//! midnight or a negative UTC offset needs no calendar library. A window is a
//! fixed UTC offset; daylight-saving changes are not applied.

use super::DeferralError;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
const MINUTE_MS: i64 = 60_000;
/// Largest accepted UTC offset (UTC−14:00 to UTC+14:00), in minutes.
const MAX_OFFSET_MINUTES: i32 = 14 * 60;

/// A daily time window during which deferred work may start, such as
/// 00:00–06:00 at UTC−03:00.
///
/// The end is exclusive. A window whose end is earlier than its start wraps
/// midnight (22:00–06:00). Times are local to a fixed UTC offset (UTC unless
/// [`Self::with_utc_offset_minutes`] is used); daylight-saving changes are
/// not applied. Unpublished v13 API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeWindow {
    start_ms: i64,
    end_ms: i64,
    offset_ms: i64,
}

impl TimeWindow {
    /// Creates a daily window from `start_hour:start_minute` (inclusive) to
    /// `end_hour:end_minute` (exclusive), in UTC.
    ///
    /// Hours are 0–23 and minutes 0–59; `24:00` is accepted as the end of the
    /// day.
    ///
    /// # Errors
    /// Returns [`DeferralError::InvalidWindow`] for an out-of-range time or
    /// an empty window (start equal to end). To allow any time, configure no
    /// window at all.
    pub fn daily(
        start_hour: u8,
        start_minute: u8,
        end_hour: u8,
        end_minute: u8,
    ) -> Result<Self, DeferralError> {
        let start_ms = time_of_day(start_hour, start_minute, false)?;
        let end_ms = time_of_day(end_hour, end_minute, true)?;
        if start_ms == end_ms {
            return Err(DeferralError::InvalidWindow(
                "a window needs different start and end times".to_string(),
            ));
        }
        Ok(Self {
            start_ms,
            end_ms,
            offset_ms: 0,
        })
    }

    /// Interprets the window's times at a fixed offset from UTC, in minutes
    /// (`-180` is UTC−03:00, `330` is UTC+05:30).
    ///
    /// # Errors
    /// Returns [`DeferralError::InvalidWindow`] outside −840..=840.
    pub fn with_utc_offset_minutes(mut self, minutes: i32) -> Result<Self, DeferralError> {
        if !(-MAX_OFFSET_MINUTES..=MAX_OFFSET_MINUTES).contains(&minutes) {
            return Err(DeferralError::InvalidWindow(format!(
                "UTC offsets must be between -{MAX_OFFSET_MINUTES} and {MAX_OFFSET_MINUTES} minutes"
            )));
        }
        self.offset_ms = i64::from(minutes) * MINUTE_MS;
        Ok(self)
    }

    /// The UTC offset of the window, in minutes.
    pub fn utc_offset_minutes(&self) -> i32 {
        i32::try_from(self.offset_ms / MINUTE_MS).unwrap_or(0)
    }

    /// Whether `instant` falls inside the window.
    pub fn contains(&self, instant: SystemTime) -> bool {
        self.contains_ms(to_millis(instant))
    }

    pub(super) fn contains_ms(&self, at_ms: i64) -> bool {
        let local = at_ms.saturating_add(self.offset_ms);
        (local - self.start_ms).rem_euclid(DAY_MS) < self.length_ms()
    }

    /// Length of one occurrence; a window that wraps midnight continues on
    /// the next day.
    fn length_ms(&self) -> i64 {
        if self.end_ms > self.start_ms {
            self.end_ms - self.start_ms
        } else {
            self.end_ms + DAY_MS - self.start_ms
        }
    }

    /// Occurrences of the window that overlap `[from_ms, until_ms)`, clipped
    /// to that range, in chronological order.
    pub(super) fn intervals(&self, from_ms: i64, until_ms: i64) -> Vec<(i64, i64)> {
        let mut intervals = Vec::new();
        if from_ms >= until_ms {
            return intervals;
        }
        let length = self.length_ms();
        // UTC instant of local midnight on the day before `from_ms`, so an
        // occurrence that wrapped past midnight is included.
        let local_from = from_ms.saturating_add(self.offset_ms);
        let mut day = local_from - local_from.rem_euclid(DAY_MS) - DAY_MS - self.offset_ms;
        while day < until_ms {
            let start = day.saturating_add(self.start_ms);
            let end = start.saturating_add(length);
            if end > from_ms && start < until_ms {
                intervals.push((start.max(from_ms), end.min(until_ms)));
            }
            day = day.saturating_add(DAY_MS);
        }
        intervals
    }
}

fn time_of_day(hour: u8, minute: u8, end: bool) -> Result<i64, DeferralError> {
    let valid = minute < 60 && (hour < 24 || end && hour == 24 && minute == 0);
    if !valid {
        return Err(DeferralError::InvalidWindow(format!(
            "{hour:02}:{minute:02} is not a valid time of day"
        )));
    }
    Ok(i64::from(hour) * HOUR_MS + i64::from(minute) * MINUTE_MS)
}

/// Unix milliseconds of `instant`, rounded down and saturating.
pub(super) fn to_millis(instant: SystemTime) -> i64 {
    match instant.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_millis()).unwrap_or(i64::MAX),
        Err(before) => {
            let millis = before.duration().as_nanos().div_ceil(1_000_000);
            i64::try_from(millis).map_or(i64::MIN, |millis| -millis)
        }
    }
}

/// The instant `millis` Unix milliseconds after the epoch (saturating at the
/// epoch for negative values, which the planner never produces).
pub(super) fn from_millis(millis: i64) -> SystemTime {
    let millis = u64::try_from(millis).unwrap_or(0);
    UNIX_EPOCH
        .checked_add(Duration::from_millis(millis))
        .unwrap_or(UNIX_EPOCH)
}
