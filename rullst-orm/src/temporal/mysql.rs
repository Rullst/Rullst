//! MySQL/MariaDB: the binary protocol values SQLx's `chrono` codecs used.
//!
//! A date-time is its length (4, 7 or 11), the year (two bytes, little
//! endian), month and day, then hour, minute and second when the time is not
//! midnight, then the microseconds (four bytes) when the fraction is not zero;
//! a time is its length (8 or 12), a zero sign byte, four zero day bytes, the
//! hour, minute and second and the microseconds when the fraction is not zero.
//! A `Timestamp` is sent as its UTC wall time with SQLx's `TIMESTAMP` type
//! (SQLx sets the session time zone to `+00:00`); generated writes, filters
//! and `Orm::raw` bindings send a `DateTime` instead (`__bind_field`), because
//! MariaDB nulls a `TIMESTAMP` parameter outside 1970-2038.
//!
//! SQLx does not tell a decoder whether a value came in the binary or the
//! text protocol. The binary row format starts each date/time value with its
//! length (at most 12), while the text format is printable (`2026-…`,
//! `-12:00:00`), so the first byte tells them apart. Text values are read with
//! the `chrono` formats ([`super::parse`]).

use jiff::civil;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::mysql::MySqlTypeInfo;
use sqlx::mysql::types::{MySqlTime, MySqlTimeSign};
use sqlx::types::time as sqlx_time;
use sqlx::{Encode, MySql, Type, TypeInfo};

use super::parse;
use super::{Date, DateTime, Time, Timestamp, text};

impl Type<MySql> for Timestamp {
    fn type_info() -> MySqlTypeInfo {
        <sqlx_time::OffsetDateTime as Type<MySql>>::type_info()
    }

    fn compatible(type_info: &MySqlTypeInfo) -> bool {
        matches!(type_info.name(), "DATETIME" | "TIMESTAMP")
    }
}

impl Type<MySql> for DateTime {
    fn type_info() -> MySqlTypeInfo {
        <sqlx_time::PrimitiveDateTime as Type<MySql>>::type_info()
    }
}

impl Type<MySql> for Date {
    fn type_info() -> MySqlTypeInfo {
        <sqlx_time::Date as Type<MySql>>::type_info()
    }
}

impl Type<MySql> for Time {
    fn type_info() -> MySqlTypeInfo {
        <MySqlTime as Type<MySql>>::type_info()
    }
}

/// The year (MySQL's range is 1000 to 9999; a negative year cannot be sent),
/// month and day.
fn push_date(date: civil::Date, buf: &mut Vec<u8>) -> Result<(), BoxDynError> {
    let year =
        u16::try_from(date.year()).map_err(|_| format!("date out of range for MySQL: {date}"))?;
    buf.extend_from_slice(&year.to_le_bytes());
    buf.extend_from_slice(&[date.month().unsigned_abs(), date.day().unsigned_abs()]);
    Ok(())
}

/// Hour, minute and second, then the microseconds when `micros`.
fn push_clock(time: civil::Time, micros: bool, buf: &mut Vec<u8>) {
    buf.extend_from_slice(&[
        time.hour().unsigned_abs(),
        time.minute().unsigned_abs(),
        time.second().unsigned_abs(),
    ]);
    if micros {
        buf.extend_from_slice(&(time.subsec_nanosecond().unsigned_abs() / 1000).to_le_bytes());
    }
}

/// The encoded length: 4 at midnight, 7 without a fraction, else 11. A
/// sub-microsecond fraction counts, and sends zero microseconds.
fn date_time_len(value: civil::DateTime) -> u8 {
    if value.subsec_nanosecond() != 0 {
        11
    } else if value.time() != civil::Time::midnight() {
        7
    } else {
        4
    }
}

fn encode_date_time(value: civil::DateTime, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
    let len = date_time_len(value);
    buf.push(len);
    push_date(value.date(), buf)?;
    if len > 4 {
        push_clock(value.time(), len > 7, buf);
    }
    Ok(IsNull::No)
}

impl Encode<'_, MySql> for Timestamp {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        encode_date_time(text::utc(self.0), buf)
    }
}

impl Encode<'_, MySql> for DateTime {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        encode_date_time(self.0, buf)
    }

    fn size_hint(&self) -> usize {
        usize::from(date_time_len(self.0)) + 1
    }
}

impl Encode<'_, MySql> for Date {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        buf.push(4);
        push_date(self.0, buf)?;
        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        5
    }
}

impl Encode<'_, MySql> for Time {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> Result<IsNull, BoxDynError> {
        let micros = self.0.subsec_nanosecond() != 0;
        buf.push(if micros { 12 } else { 8 });
        // Not negative, and no days.
        buf.extend_from_slice(&[0; 5]);
        push_clock(self.0, micros, buf);
        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        if self.0.subsec_nanosecond() != 0 {
            13
        } else {
            9
        }
    }
}

/// A value as the row holds it.
enum Raw<'r> {
    /// Binary protocol: the length byte, then the fields.
    Binary(&'r [u8]),
    /// Text protocol.
    Text(&'r str),
}

/// The binary row format starts a date/time value with its length (at most
/// 12); the text format is printable.
fn classify(bytes: &[u8]) -> Result<Raw<'_>, BoxDynError> {
    Ok(match bytes.first() {
        Some(length) if *length < b' ' => Raw::Binary(bytes),
        _ => Raw::Text(std::str::from_utf8(bytes)?),
    })
}

/// Year, month and day after the length byte; `None` inside `Ok` for the
/// zero date, which has no bytes.
pub(super) fn date_from_binary(fields: &[u8]) -> Result<Option<civil::Date>, BoxDynError> {
    match fields {
        [] => Ok(None),
        [year_low, year_high, month, day, ..] => {
            let year = i16::try_from(u16::from_le_bytes([*year_low, *year_high]))?;
            let date = civil::Date::new(year, i8::try_from(*month)?, i8::try_from(*day)?)
                .map_err(|_| format!("server returned invalid date: {year}/{month}/{day}"))?;
            Ok(Some(date))
        }
        _ => Err(format!("expected at least 4 bytes for date, got {}", fields.len()).into()),
    }
}

/// A binary date-time: its length byte decides whether a time (above 4) and
/// microseconds (above 7, every remaining byte, little endian) follow.
pub(super) fn date_time_from_binary(bytes: &[u8]) -> Result<civil::DateTime, BoxDynError> {
    let (&len, fields) = bytes.split_first().ok_or("empty buffer")?;
    let date = date_from_binary(fields)?.ok_or("unexpected zero date")?;
    if len <= 4 {
        return Ok(date.to_datetime(civil::Time::midnight()));
    }
    let Some([hour, minute, second, micros @ ..]) = fields.get(4..) else {
        return Err("missing time bytes".into());
    };
    let micros = if len > 7 {
        if micros.len() > 8 {
            return Err("too many microsecond bytes".into());
        }
        let mut le = [0; 8];
        le[..micros.len()].copy_from_slice(micros);
        u32::try_from(u64::from_le_bytes(le))?
    } else {
        0
    };
    let time = clock(*hour, *minute, *second, micros)
        .ok_or_else(|| format!("server returned invalid time: {hour}:{minute}:{second}"))?;
    Ok(date.to_datetime(time))
}

/// `chrono`'s `from_hms_micro_opt`: a second `59` may carry a leap second
/// (1,000,000 to 1,999,999 microseconds), read as second 59.
fn clock(hour: u8, minute: u8, second: u8, micros: u32) -> Option<civil::Time> {
    let micros = match micros {
        0..=999_999 => micros,
        1_000_000..=1_999_999 if second == 59 => micros - 1_000_000,
        _ => return None,
    };
    civil::Time::new(
        i8::try_from(hour).ok()?,
        i8::try_from(minute).ok()?,
        i8::try_from(second).ok()?,
        i32::try_from(micros * 1000).ok()?,
    )
    .ok()
}

/// A `DATETIME` or `TIMESTAMP` value in either protocol.
pub(super) fn date_time_from_value(bytes: &[u8]) -> Result<civil::DateTime, BoxDynError> {
    match classify(bytes)? {
        Raw::Binary(bytes) => date_time_from_binary(bytes),
        Raw::Text(text) => parse::parse(text, parse::DATE_TIME)
            .and_then(|parsed| parsed.date_time())
            .ok_or_else(|| format!("invalid MySQL DATETIME: {text}").into()),
    }
}

/// An instant: the UTC wall time of [`date_time_from_value`].
pub(super) fn instant_from_value(bytes: &[u8]) -> Result<jiff::Timestamp, BoxDynError> {
    Ok(jiff::tz::Offset::UTC.to_timestamp(date_time_from_value(bytes)?)?)
}

/// A `DATE` value in either protocol.
pub(super) fn date_from_value(bytes: &[u8]) -> Result<civil::Date, BoxDynError> {
    match classify(bytes)? {
        Raw::Binary(bytes) => Ok(date_from_binary(&bytes[1..])?.ok_or("unexpected zero date")?),
        Raw::Text(text) => parse::parse(text, parse::DATE_ONLY)
            .and_then(|parsed| parsed.date())
            .ok_or_else(|| format!("invalid MySQL DATE: {text}").into()),
    }
}

/// A binary `TIME` as SQLx's `MySqlTime` decoder reads it: no fields for
/// zero, else a sign byte, four day bytes, hour, minute, second and optional
/// microseconds, validated by `MySqlTime::new`.
pub(super) fn mysql_time_from_binary(bytes: &[u8]) -> Result<MySqlTime, BoxDynError> {
    let (&length, fields) = bytes.split_first().ok_or("empty buffer")?;
    if length == 0 {
        return Ok(MySqlTime::ZERO);
    }
    let (sign, days, hours, minutes, seconds, micros) = match fields {
        [sign, d0, d1, d2, d3, hours, minutes, seconds] => (
            *sign,
            [*d0, *d1, *d2, *d3],
            *hours,
            *minutes,
            *seconds,
            [0; 4],
        ),
        [
            sign,
            d0,
            d1,
            d2,
            d3,
            hours,
            minutes,
            seconds,
            m0,
            m1,
            m2,
            m3,
        ] => (
            *sign,
            [*d0, *d1, *d2, *d3],
            *hours,
            *minutes,
            *seconds,
            [*m0, *m1, *m2, *m3],
        ),
        _ => {
            return Err(format!(
                "expected 8 or 12 bytes for TIME value, got {}",
                fields.len()
            )
            .into());
        }
    };
    let sign = match sign {
        0 => MySqlTimeSign::Positive,
        1 => MySqlTimeSign::Negative,
        other => return Err(format!("expected 0 or 1 for TIME sign byte, got {other}").into()),
    };
    let whole_hours = u32::from_le_bytes(days)
        .checked_mul(24)
        .and_then(|hours_of_days| hours_of_days.checked_add(u32::from(hours)))
        .ok_or("overflow calculating whole hours from `days * 24 + hours`")?;
    Ok(MySqlTime::new(
        sign,
        whole_hours,
        minutes,
        seconds,
        u32::from_le_bytes(micros),
    )?)
}

/// A `MySqlTime` must be a time of day; its sign is ignored, as `chrono`
/// ignored it.
pub(super) fn time_from_mysql_time(time: &MySqlTime) -> Option<civil::Time> {
    clock(
        u8::try_from(time.hours()).ok()?,
        time.minutes(),
        time.seconds(),
        time.microseconds(),
    )
}

/// A `TIME` value in either protocol.
pub(super) fn time_from_value(bytes: &[u8]) -> Result<civil::Time, BoxDynError> {
    match classify(bytes)? {
        Raw::Binary(bytes) => {
            let time = mysql_time_from_binary(bytes)?;
            time_from_mysql_time(&time)
                .ok_or_else(|| format!("Cannot convert `MySqlTime` value to a time: {time}").into())
        }
        Raw::Text(text) => parse::parse(text, parse::TIME)
            .and_then(|parsed| parsed.time())
            .ok_or_else(|| format!("invalid MySQL TIME: {text}").into()),
    }
}

// The `Decode` implementations only hand the value's bytes to the functions
// above; they need a live server.
#[path = "mysql_decode.rs"]
mod decode;

#[cfg(test)]
#[path = "mysql_tests.rs"]
mod tests;
