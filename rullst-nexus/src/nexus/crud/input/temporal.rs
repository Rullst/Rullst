//! Date and time form values: parsing, the canonical text, the typed value
//! bound for native `DATE`, `TIME`, `TIMESTAMP` and `TIMESTAMPTZ` columns,
//! and the value a `time` or `datetime-local` input can show.

use jiff::civil::{Date, DateTime, Time};

use super::{FormInputError, canonical_datetime, parse_date_part};
use crate::nexus::NexusTimeZone;
use crate::nexus::instant::{Instant, date_text, time_text};
use crate::nexus::types::{FieldKind, RegistryEntry};

/// The typed value of a validated date or time, bound instead of its text
/// so native date/time columns accept it (see `dialect::driver_value`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemporalValue {
    /// A [`FieldKind::Date`] value.
    Date(Date),
    /// A [`FieldKind::Time`] value.
    Time(Time),
    /// A [`FieldKind::DateTime`] value without an offset.
    DateTime(DateTime),
    /// A [`FieldKind::DateTimeUtc`] value.
    Instant(Instant),
}

/// A calendar date `YYYY-MM-DD` (year 0001 to 9999, as HTML date inputs).
pub(super) fn date(value: &str) -> Option<Date> {
    let mut parts = value.split('-');
    let year = parse_date_part(parts.next(), 4)?;
    let month = parse_date_part(parts.next(), 2)?;
    let day = parse_date_part(parts.next(), 2)?;
    if parts.next().is_some() || year == 0 {
        return None;
    }
    Date::new(
        i16::try_from(year).ok()?,
        i8::try_from(month).ok()?,
        i8::try_from(day).ok()?,
    )
    .ok()
}

/// A time of day `HH:MM`, `HH:MM:SS` or `HH:MM:SS.fraction` (1 to 9 digits).
pub(super) fn time(value: &str) -> Option<Time> {
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
    // Rejects hour 24, minute or second 60 (jiff has no leap second).
    Time::new(
        i8::try_from(hour).ok()?,
        i8::try_from(minute).ok()?,
        i8::try_from(second).ok()?,
        i32::try_from(nanos).ok()?,
    )
    .ok()
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
fn date_and_time(value: &str) -> Option<(Date, &str)> {
    let (date_part, time_part) = value.split_once(['T', ' '])?;
    Some((date(date_part)?, time_part))
}

/// A local date-time without an offset, as a `datetime-local` input submits.
pub(super) fn local_date_time(value: &str) -> Option<DateTime> {
    let (date, time_part) = date_and_time(value)?;
    Some(date.to_datetime(time(time_part)?))
}

/// The offset in seconds east of UTC of `Z`, `±HH:MM` or `±HH` (PostgreSQL
/// writes `+00`) under 24 hours, and the time before it.
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
    if fields.next().is_some() || hours > 23 || minutes > 59 {
        return None;
    }
    let seconds = i32::try_from(hours * 3600 + minutes * 60).ok()?;
    Some((local, sign * seconds))
}

/// A UTC instant: a date-time with `Z`, `±HH:MM` or `±HH` is converted to
/// UTC, and one without an offset, as a `datetime-local` input submits, is
/// already UTC.
pub(super) fn utc(value: &str) -> Option<Instant> {
    let (date, time_part) = date_and_time(value)?;
    let (local, offset) = match split_offset(time_part) {
        Some((local, offset)) => (local, offset),
        None => (time_part, 0),
    };
    Instant::at_offset(date.to_datetime(time(local)?), offset)
}

/// The canonical text and the typed value of a validated form value. A
/// date-time with an offset in a [`FieldKind::DateTime`] field (a text
/// column) keeps its text and binds no typed value; other kinds are not
/// temporal. `None` means `value` is not a valid value of its kind.
pub(super) fn normalize(
    kind: &FieldKind,
    value: String,
) -> Option<(String, Option<TemporalValue>)> {
    Some(match kind {
        FieldKind::Date => {
            let date = date(&value)?;
            (value, Some(TemporalValue::Date(date)))
        }
        FieldKind::Time => {
            let time = time(&value)?;
            (time_text(time), Some(TemporalValue::Time(time)))
        }
        FieldKind::DateTime => match local_date_time(&value) {
            Some(local) => (
                canonical_datetime(value),
                Some(TemporalValue::DateTime(local)),
            ),
            None => (value, None),
        },
        FieldKind::DateTimeUtc => {
            let instant = utc(&value)?;
            (
                instant.rfc3339(false)?,
                Some(TemporalValue::Instant(instant)),
            )
        }
        _ => (value, None),
    })
}

/// `HH:MM:SS[.mmm]`: what `time` and `datetime-local` inputs can show, at
/// most millisecond precision; `None` for a finer value.
fn input_time(time: Time) -> Option<String> {
    (time.subsec_nanosecond() % 1_000_000 == 0).then(|| time_text(time))
}

/// The value of a `time` input showing the stored `text`, when it can.
pub(crate) fn time_input_value(text: &str) -> Option<String> {
    input_time(time(text)?)
}

/// The input type and value that show the stored instant `text` (RFC 3339,
/// PostgreSQL, or a MySQL `DATETIME` without an offset, which holds UTC) in
/// `zone`: its wall time in a `datetime-local` input, or its UTC value (`Z`)
/// in a text input when `zone` would read that wall time as another instant,
/// the later occurrence of a time a fold repeats. `None` when the wall time
/// is outside years 1 to 9999 or finer than milliseconds.
pub(crate) fn zoned_input_value(
    text: &str,
    zone: &NexusTimeZone,
) -> Option<(&'static str, String)> {
    let instant = utc(text)?;
    let local = instant.wall_time(zone.offset_at(&instant)?)?;
    if !(1..=9999).contains(&local.year()) {
        return None;
    }
    let time = input_time(local.time())?;
    if zone.instant_of(&local).ok()? != instant {
        return Some(("text", instant.rfc3339(true)?));
    }
    Some((
        "datetime-local",
        format!("{}T{time}", date_text(local.date())),
    ))
}

/// Rewrites each submitted `DateTimeUtc` value without an offset, a wall
/// time in the administrator's `zone`, as its UTC instant (`Z`); validation
/// then reads it like a value typed with an offset. Other values are kept
/// for [`super::validate_form_values`] to check.
pub(crate) fn localize_form_values(
    entry: &RegistryEntry,
    pairs: Vec<(String, String)>,
    zone: &NexusTimeZone,
) -> Result<Vec<(String, String)>, FormInputError> {
    pairs
        .into_iter()
        .map(|(name, value)| {
            let field = entry
                .fields
                .iter()
                .find(|field| field.name == name && matches!(field.kind, FieldKind::DateTimeUtc));
            let Some(field) = field else {
                return Ok((name, value));
            };
            match zoned_submission(&value, zone) {
                Ok(Some(instant)) => Ok((name, instant)),
                Ok(None) => Ok((name, value)),
                Err(_) => Err(FormInputError::NonexistentLocalTime {
                    field: field.name,
                    zone: zone.name().to_string(),
                }),
            }
        })
        .collect()
}

/// The UTC instant (`Z`) of a submitted value without an offset, read as a
/// wall time in `zone`; `Ok(None)` for any other value, which validation
/// then reads as before.
fn zoned_submission(
    value: &str,
    zone: &NexusTimeZone,
) -> Result<Option<String>, crate::nexus::NexusTimeZoneError> {
    let Some(local) = local_date_time(value) else {
        return Ok(None);
    };
    Ok(zone.instant_of(&local)?.rfc3339(true))
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

#[cfg(test)]
#[path = "zone_tests.rs"]
mod zone_tests;
