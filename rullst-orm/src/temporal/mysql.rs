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
use sqlx::mysql::types::MySqlTime;
use sqlx::mysql::{MySqlTypeInfo, MySqlValueRef};
use sqlx::types::time as sqlx_time;
use sqlx::{Decode, Encode, MySql, Type, TypeInfo};

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

fn raw(value: MySqlValueRef<'_>) -> Result<Raw<'_>, BoxDynError> {
    let bytes = <&[u8] as Decode<MySql>>::decode(value)?;
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

fn date_time(value: MySqlValueRef<'_>) -> Result<civil::DateTime, BoxDynError> {
    match raw(value)? {
        Raw::Binary(bytes) => date_time_from_binary(bytes),
        Raw::Text(text) => parse::parse(text, parse::DATE_TIME)
            .and_then(|parsed| parsed.date_time())
            .ok_or_else(|| format!("invalid MySQL DATETIME: {text}").into()),
    }
}

impl<'r> Decode<'r, MySql> for Timestamp {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        let instant = jiff::tz::Offset::UTC.to_timestamp(date_time(value)?)?;
        Ok(Self(instant))
    }
}

impl<'r> Decode<'r, MySql> for DateTime {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        date_time(value).map(Self)
    }
}

impl<'r> Decode<'r, MySql> for Date {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        let date = match raw(value)? {
            Raw::Binary(bytes) => date_from_binary(&bytes[1..])?.ok_or("unexpected zero date")?,
            Raw::Text(text) => parse::parse(text, parse::DATE_ONLY)
                .and_then(|parsed| parsed.date())
                .ok_or_else(|| format!("invalid MySQL DATE: {text}"))?,
        };
        Ok(Self(date))
    }
}

/// A binary `TIME` is read by SQLx's `MySqlTime` and must be a time of day;
/// its sign is ignored, as `chrono` ignored it.
pub(super) fn time_from_mysql_time(time: &MySqlTime) -> Option<civil::Time> {
    clock(
        u8::try_from(time.hours()).ok()?,
        time.minutes(),
        time.seconds(),
        time.microseconds(),
    )
}

impl<'r> Decode<'r, MySql> for Time {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        let time = match raw(value.clone())? {
            Raw::Binary(_) => {
                let time = MySqlTime::decode(value)?;
                time_from_mysql_time(&time)
                    .ok_or_else(|| format!("Cannot convert `MySqlTime` value to a time: {time}"))?
            }
            Raw::Text(text) => parse::parse(text, parse::TIME)
                .and_then(|parsed| parsed.time())
                .ok_or_else(|| format!("invalid MySQL TIME: {text}"))?,
        };
        Ok(Self(time))
    }
}

#[cfg(test)]
#[path = "mysql_tests.rs"]
mod tests;
