//! Date and time form values: parsing, the canonical text, the typed value
//! bound for native `DATE`, `TIME`, `TIMESTAMP` and `TIMESTAMPTZ` columns,
//! and the value a `time` or `datetime-local` input can show.

use rullst_orm::RullstValue;
use rullst_orm::chrono::{
    DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, Timelike,
    Utc,
};

use super::{canonical_datetime, parse_date_part};
use crate::nexus::types::FieldKind;

/// A calendar date `YYYY-MM-DD` (year 0001 to 9999, as HTML date inputs).
pub(super) fn date(value: &str) -> Option<NaiveDate> {
    let mut parts = value.split('-');
    let year = parse_date_part(parts.next(), 4)?;
    let month = parse_date_part(parts.next(), 2)?;
    let day = parse_date_part(parts.next(), 2)?;
    if parts.next().is_some() || year == 0 {
        return None;
    }
    NaiveDate::from_ymd_opt(i32::try_from(year).ok()?, month, day)
}

/// A time of day `HH:MM`, `HH:MM:SS` or `HH:MM:SS.fraction` (1 to 9 digits).
pub(super) fn time(value: &str) -> Option<NaiveTime> {
    let mut parts = value.split(':');
    let hour = parse_date_part(parts.next(), 2)?;
    let minute = parse_date_part(parts.next(), 2)?;
    let (second, nanos) = match parts.next() {
        Some(seconds) => seconds_and_nanos(seconds)?,
        None => (0, 0),
    };
    if parts.next().is_some() {
        return None;
    }
    // Rejects hour 24, minute or second 60; with at most nine fraction
    // digits there is no leap second.
    NaiveTime::from_hms_nano_opt(hour, minute, second, nanos)
}

fn seconds_and_nanos(seconds: &str) -> Option<(u32, u32)> {
    let (whole, fraction) = match seconds.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (seconds, None),
    };
    let second = parse_date_part(Some(whole), 2)?;
    let nanos = match fraction {
        None => 0,
        Some(digits)
            if (1..=9).contains(&digits.len())
                && digits.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            format!("{digits:0<9}").parse().ok()?
        }
        Some(_) => return None,
    };
    Some((second, nanos))
}

/// Splits a date-time at its `T` or space into the date and the time part.
fn date_and_time(value: &str) -> Option<(NaiveDate, &str)> {
    let (date_part, time_part) = value.split_once(['T', ' '])?;
    Some((date(date_part)?, time_part))
}

/// A local date-time without an offset, as a `datetime-local` input submits.
pub(super) fn local_date_time(value: &str) -> Option<NaiveDateTime> {
    let (date, time_part) = date_and_time(value)?;
    Some(date.and_time(time(time_part)?))
}

/// The offset in seconds east of UTC of `Z`, `±HH:MM` or `±HH` (PostgreSQL
/// writes `+00`), and the time before it.
fn split_offset(time_part: &str) -> Option<(&str, i32)> {
    if let Some(local) = time_part.strip_suffix(['Z', 'z']) {
        return Some((local, 0));
    }
    let index = time_part.rfind(['+', '-'])?;
    let (local, offset) = time_part.split_at(index);
    let sign = if offset.starts_with('-') { -1 } else { 1 };
    let mut fields = offset.get(1..)?.split(':');
    let hours = parse_date_part(fields.next(), 2)?;
    let minutes = match fields.next() {
        Some(minutes) => parse_date_part(Some(minutes), 2)?,
        None => 0,
    };
    // `FixedOffset::east_opt` rejects 24 hours or more.
    if fields.next().is_some() || minutes > 59 {
        return None;
    }
    let seconds = i32::try_from(hours * 3600 + minutes * 60).ok()?;
    Some((local, sign * seconds))
}

/// A UTC instant: a date-time with `Z`, `±HH:MM` or `±HH` is converted to
/// UTC, and one without an offset, as a `datetime-local` input submits, is
/// already UTC.
pub(super) fn utc(value: &str) -> Option<DateTime<Utc>> {
    let (date, time_part) = date_and_time(value)?;
    let (local, offset) = match split_offset(time_part) {
        Some((local, offset)) => (local, offset),
        None => (time_part, 0),
    };
    let local = date.and_time(time(local)?);
    let zone = FixedOffset::east_opt(offset)?;
    Some(local.and_local_timezone(zone).single()?.with_timezone(&Utc))
}

/// The canonical text and the typed value of a validated form value. A
/// date-time with an offset in a [`FieldKind::DateTime`] field (a text
/// column) keeps its text and binds no typed value; other kinds are not
/// temporal. `None` means `value` is not a valid value of its kind.
pub(super) fn normalize(kind: &FieldKind, value: String) -> Option<(String, Option<RullstValue>)> {
    Some(match kind {
        FieldKind::Date => {
            let date = date(&value)?;
            (value, Some(RullstValue::Date(date)))
        }
        FieldKind::Time => {
            let time = time(&value)?;
            (
                time.format("%H:%M:%S%.f").to_string(),
                Some(RullstValue::Time(time)),
            )
        }
        FieldKind::DateTime => match local_date_time(&value) {
            Some(local) => (
                canonical_datetime(value),
                Some(RullstValue::NaiveDateTime(local)),
            ),
            None => (value, None),
        },
        FieldKind::DateTimeUtc => {
            let instant = utc(&value)?;
            (
                instant.to_rfc3339_opts(SecondsFormat::AutoSi, false),
                Some(RullstValue::DateTime(instant)),
            )
        }
        _ => (value, None),
    })
}

/// `HH:MM:SS[.mmm]`: what `time` and `datetime-local` inputs can show, at
/// most millisecond precision; `None` for a finer value.
fn input_time(time: NaiveTime) -> Option<String> {
    let nanos = time.nanosecond();
    if !nanos.is_multiple_of(1_000_000) {
        return None;
    }
    let seconds = time.format("%H:%M:%S").to_string();
    Some(match nanos / 1_000_000 {
        0 => seconds,
        millis => format!("{seconds}.{millis:03}"),
    })
}

/// The value of a `time` input showing the stored `text`, when it can.
pub(crate) fn time_input_value(text: &str) -> Option<String> {
    input_time(time(text)?)
}

/// The UTC `datetime-local` value of the stored instant `text`: an RFC 3339
/// or PostgreSQL value is converted to UTC, and a MySQL `DATETIME` value
/// without an offset already holds UTC.
pub(crate) fn utc_input_value(text: &str) -> Option<String> {
    let instant = utc(text)?;
    if !(1..=9999).contains(&instant.year()) {
        return None;
    }
    let time = input_time(instant.time())?;
    Some(format!("{}T{time}", instant.format("%Y-%m-%d")))
}

/// Whether the time part of a date-time carries an offset (`Z`, `+`, `-`).
pub(crate) fn has_offset(value: &str) -> bool {
    value
        .split_once(['T', ' '])
        .is_some_and(|(_, time)| time.contains(['Z', 'z', '+', '-']))
}

#[cfg(test)]
#[path = "temporal_tests.rs"]
mod tests;
