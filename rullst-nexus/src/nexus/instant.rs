//! The UTC instant of a form or stored date-time value, and the text Nexus
//! writes for dates and times.
//!
//! A wall time of years 1 to 9999 at an offset under 24 hours names an
//! instant from 0000-12-31 to 10000-01-01 UTC. Nexus has accepted all of
//! them since `DateTimeUtc` fields were added, but a `jiff::Timestamp` ends
//! at 9999-12-30T22:00Z and jiff's civil types at 9999-12-31. An [`Instant`]
//! is therefore the time elapsed since the Unix epoch, which reaches every
//! one, and becomes a `jiff::Timestamp` only where a value needs one.

use std::fmt::Write as _;

use jiff::SignedDuration;
use jiff::civil::{Date, DateTime, Time};

/// The wall time of the Unix epoch.
const UNIX_EPOCH: DateTime = DateTime::constant(1970, 1, 1, 0, 0, 0, 0);
const ONE_DAY: SignedDuration = SignedDuration::from_hours(24);

/// A UTC instant, as the time elapsed since 1970-01-01T00:00:00Z.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Instant(SignedDuration);

impl Instant {
    /// The instant of the wall time `local` at `offset` seconds east of UTC.
    pub(crate) fn at_offset(local: DateTime, offset: i32) -> Option<Self> {
        local
            .duration_since(UNIX_EPOCH)
            .checked_sub(SignedDuration::from_secs(i64::from(offset)))
            .map(Self)
    }

    /// The wall time of this instant at `offset` seconds east of UTC, when
    /// it lies in jiff's years -9999 to 9999.
    pub(crate) fn wall_time(self, offset: i32) -> Option<DateTime> {
        let elapsed = self
            .0
            .checked_add(SignedDuration::from_secs(i64::from(offset)))?;
        UNIX_EPOCH.checked_add(elapsed).ok()
    }

    /// This instant as a `jiff::Timestamp`, when its range holds it.
    pub(crate) fn timestamp(self) -> Option<jiff::Timestamp> {
        jiff::Timestamp::from_duration(self.0).ok()
    }

    /// The UTC date as text and the UTC time of day.
    fn utc_parts(self) -> Option<(String, Time)> {
        if let Some(wall) = self.wall_time(0) {
            return Some((date_text(wall.date()), wall.time()));
        }
        // Beyond jiff's last date, the instant is on 10000-01-01 exactly
        // when the day before it is on 9999-12-31, jiff's last date.
        let day_before = Self(self.0.checked_sub(ONE_DAY)?).wall_time(0)?;
        Some(("+10000-01-01".to_owned(), day_before.time()))
    }

    /// RFC 3339 text in UTC ending in `Z` or `+00:00`, as chrono's
    /// `to_rfc3339_opts(SecondsFormat::AutoSi, use_z)` wrote it:
    /// `2026-10-15T09:30:00.500+00:00`, and `+10000-01-01T…` in year 10000.
    pub(crate) fn rfc3339(self, use_z: bool) -> Option<String> {
        let (date, time) = self.utc_parts()?;
        let zone = if use_z { "Z" } else { "+00:00" };
        Some(format!("{date}T{}{zone}", time_text(time)))
    }

    /// The UTC wall time as `YYYY-MM-DD HH:MM:SS[.f]` (chrono's
    /// `%F %T%.f`), and `+10000-01-01 …` in year 10000.
    pub(crate) fn utc_wall_text(self) -> Option<String> {
        let (date, time) = self.utc_parts()?;
        Some(format!("{date} {}", time_text(time)))
    }
}

/// `YYYY-MM-DD` (chrono's `%Y-%m-%d` for years 0 to 9999).
pub(crate) fn date_text(date: Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

/// `HH:MM:SS` and no fraction or 3, 6 or 9 fraction digits, whichever
/// shows every non-zero digit (chrono's `%H:%M:%S%.f`).
pub(crate) fn time_text(time: Time) -> String {
    let mut text = format!(
        "{:02}:{:02}:{:02}",
        time.hour(),
        time.minute(),
        time.second()
    );
    let nanos = time.subsec_nanosecond();
    // Writing to a `String` cannot fail.
    let _ = if nanos == 0 {
        Ok(())
    } else if nanos % 1_000_000 == 0 {
        write!(text, ".{:03}", nanos / 1_000_000)
    } else if nanos % 1_000 == 0 {
        write!(text, ".{:06}", nanos / 1_000)
    } else {
        write!(text, ".{nanos:09}")
    };
    text
}

/// A date and time joined by `separator` (chrono's `%Y-%m-%d`, then
/// `%H:%M:%S%.f`).
pub(crate) fn date_time_text(value: DateTime, separator: char) -> String {
    format!(
        "{}{separator}{}",
        date_text(value.date()),
        time_text(value.time())
    )
}

#[cfg(test)]
#[path = "instant_tests.rs"]
mod tests;
