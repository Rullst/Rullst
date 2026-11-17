//! The text forms of the date/time types: on SQLite, on the SQLx `Any` pool
//! and in Redis query-cache keys. Each is the exact text the `chrono`-era code
//! wrote (SQLx's `chrono` codecs and `RullstValue`'s text path), so stored
//! values and bound filters keep comparing as text with older rows.
//!
//! Years 0 to 9999 have four digits; a negative year has a sign and at least
//! four digits (`-0001`), as chrono's `%Y` writes it.

// Each pool uses part of this module; a build with one driver leaves the rest
// unused.
#![cfg_attr(not(feature = "drivers-all"), allow(dead_code))]

use std::fmt::Write as _;

use jiff::civil;

use super::{Date, DateTime, Time, Timestamp};

/// The UTC wall time of an instant.
pub(crate) fn utc(instant: jiff::Timestamp) -> civil::DateTime {
    jiff::tz::Offset::UTC.to_datetime(instant)
}

/// `2026-10-15T09:30:00.5+00:00` becomes `2026-10-15T09:30:00.500+00:00`:
/// chrono's `to_rfc3339_opts(SecondsFormat::AutoSi, false)`.
pub(crate) fn timestamp(instant: Timestamp) -> String {
    let wall = utc(instant.0);
    let mut text = String::with_capacity(32);
    push_date(&mut text, wall.date());
    text.push('T');
    push_clock(&mut text, wall.time());
    push_auto_fraction(&mut text, wall.subsec_nanosecond());
    text.push_str("+00:00");
    text
}

/// `YYYY-MM-DD HH:MM:SS[.f]`: chrono's `%F %T%.f`.
pub(crate) fn date_time(value: DateTime) -> String {
    let mut text = String::with_capacity(29);
    push_date(&mut text, value.date());
    text.push(' ');
    push_clock(&mut text, value.time());
    push_auto_fraction(&mut text, value.subsec_nanosecond());
    text
}

/// `YYYY-MM-DD`: chrono's `%F`.
pub(crate) fn date(value: Date) -> String {
    let mut text = String::with_capacity(10);
    push_date(&mut text, value.0);
    text
}

/// `HH:MM:SS[.f]`: chrono's `%T%.f`.
pub(crate) fn time(value: Time) -> String {
    let mut text = String::with_capacity(18);
    push_clock(&mut text, value.0);
    push_auto_fraction(&mut text, value.subsec_nanosecond());
    text
}

/// `2026-10-15T09:30:00.500000000Z`: chrono's
/// `to_rfc3339_opts(SecondsFormat::Nanos, true)`, the query-cache key form.
#[cfg(feature = "redis")]
pub(crate) fn timestamp_nanos(instant: Timestamp) -> String {
    let wall = utc(instant.0);
    let mut text = String::with_capacity(30);
    push_date(&mut text, wall.date());
    text.push('T');
    push_clock(&mut text, wall.time());
    push_nine_digits(&mut text, wall.subsec_nanosecond());
    text.push('Z');
    text
}

/// `%F %T%.9f`, the query-cache key form.
#[cfg(feature = "redis")]
pub(crate) fn date_time_nanos(value: DateTime) -> String {
    let mut text = String::with_capacity(29);
    push_date(&mut text, value.date());
    text.push(' ');
    push_clock(&mut text, value.time());
    push_nine_digits(&mut text, value.subsec_nanosecond());
    text
}

/// `%T%.9f`, the query-cache key form.
#[cfg(feature = "redis")]
pub(crate) fn time_nanos(value: Time) -> String {
    let mut text = String::with_capacity(18);
    push_clock(&mut text, value.0);
    push_nine_digits(&mut text, value.subsec_nanosecond());
    text
}

fn push_date(text: &mut String, date: civil::Date) {
    let year = date.year();
    // Writing to a `String` cannot fail.
    let _ = if (0..=9999).contains(&year) {
        write!(text, "{year:04}")
    } else {
        write!(text, "{year:+05}")
    };
    let _ = write!(text, "-{:02}-{:02}", date.month(), date.day());
}

fn push_clock(text: &mut String, time: civil::Time) {
    let _ = write!(
        text,
        "{:02}:{:02}:{:02}",
        time.hour(),
        time.minute(),
        time.second()
    );
}

/// chrono's `%.f`: nothing for a whole second, else 3, 6 or 9 digits,
/// whichever shows every non-zero digit.
fn push_auto_fraction(text: &mut String, nanos: i32) {
    let _ = if nanos == 0 {
        Ok(())
    } else if nanos % 1_000_000 == 0 {
        write!(text, ".{:03}", nanos / 1_000_000)
    } else if nanos % 1_000 == 0 {
        write!(text, ".{:06}", nanos / 1_000)
    } else {
        write!(text, ".{nanos:09}")
    };
}

#[cfg(feature = "redis")]
fn push_nine_digits(text: &mut String, nanos: i32) {
    let _ = write!(text, ".{nanos:09}");
}
