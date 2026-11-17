//! SQLite: ISO-8601 text, as SQLx's `chrono` codecs wrote and read it.
//!
//! Writes are the text forms of [`super::text`]. Reads accept what the
//! `chrono` decoders accepted: for an instant or a date-time, RFC 3339 and the
//! other SQLite text forms ([`super::parse`]), an integer (Unix seconds) or a
//! real (a Julian day number); for a date, `YYYY-MM-DD`; for a time, the
//! SQLite time forms. Column compatibility is the same too.

use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::sqlite::{SqliteArgumentsBuffer, SqliteTypeInfo, SqliteValueRef};
use sqlx::types::time as sqlx_time;
use sqlx::{Decode, Encode, Sqlite, Type, TypeInfo, ValueRef};

use super::parse::{self, Moment};
use super::{Date, DateTime, Time, Timestamp, text};

/// A value's storage class or a column's declared type, by SQLx's name.
fn named(type_info: &SqliteTypeInfo, names: &[&str]) -> bool {
    names.contains(&type_info.name())
}

/// `DATETIME`, `TEXT`, `INTEGER` or `REAL`: the `chrono` date-time set.
const DATE_TIME_TYPES: &[&str] = &["DATETIME", "TEXT", "INTEGER", "REAL"];

impl Type<Sqlite> for Timestamp {
    fn type_info() -> SqliteTypeInfo {
        <sqlx_time::OffsetDateTime as Type<Sqlite>>::type_info()
    }

    fn compatible(type_info: &SqliteTypeInfo) -> bool {
        named(type_info, DATE_TIME_TYPES)
    }
}

impl Type<Sqlite> for DateTime {
    fn type_info() -> SqliteTypeInfo {
        <sqlx_time::PrimitiveDateTime as Type<Sqlite>>::type_info()
    }

    fn compatible(type_info: &SqliteTypeInfo) -> bool {
        named(type_info, DATE_TIME_TYPES)
    }
}

impl Type<Sqlite> for Date {
    fn type_info() -> SqliteTypeInfo {
        <sqlx_time::Date as Type<Sqlite>>::type_info()
    }

    fn compatible(type_info: &SqliteTypeInfo) -> bool {
        named(type_info, &["DATE", "TEXT"])
    }
}

impl Type<Sqlite> for Time {
    fn type_info() -> SqliteTypeInfo {
        <sqlx_time::Time as Type<Sqlite>>::type_info()
    }

    fn compatible(type_info: &SqliteTypeInfo) -> bool {
        named(type_info, &["TIME", "TEXT"])
    }
}

impl Encode<'_, Sqlite> for Timestamp {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(text::timestamp(*self), buf)
    }
}

impl Encode<'_, Sqlite> for DateTime {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(text::date_time(*self), buf)
    }
}

impl Encode<'_, Sqlite> for Date {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(text::date(*self), buf)
    }
}

impl Encode<'_, Sqlite> for Time {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(text::time(*self), buf)
    }
}

/// Text, an integer or a real, by the value's storage class.
fn moment(value: SqliteValueRef<'_>) -> Result<Moment, BoxDynError> {
    let storage = value.type_info().name().to_owned();
    let moment = match storage.as_str() {
        "TEXT" => parse::sqlite_date_time(<&str as Decode<Sqlite>>::decode(value)?),
        "INTEGER" => from_unix_seconds(<i64 as Decode<Sqlite>>::decode(value)?),
        "REAL" => from_julian_day(<f64 as Decode<Sqlite>>::decode(value)?),
        _ => None,
    };
    moment.ok_or_else(|| "invalid datetime".into())
}

/// An integer is Unix seconds.
pub(super) fn from_unix_seconds(seconds: i64) -> Option<Moment> {
    utc_moment(jiff::Timestamp::from_second(seconds).ok()?)
}

/// A real is a Julian day number. The whole and fractional seconds are
/// combined as SQLx's `chrono` decoder combined them: the fraction's absolute
/// value is added to the truncated seconds.
pub(super) fn from_julian_day(day: f64) -> Option<Moment> {
    let seconds = (day - 2_440_587.5) * 86_400.0;
    if !seconds.is_finite() {
        return None;
    }
    // The saturating casts are the decoder's; out-of-range values fail below.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (whole, nanos) = (seconds.trunc() as i64, (seconds.fract() * 1E9).abs() as u32);
    let total = i128::from(whole) * 1_000_000_000 + i128::from(nanos);
    utc_moment(jiff::Timestamp::from_nanosecond(total).ok()?)
}

fn utc_moment(instant: jiff::Timestamp) -> Option<Moment> {
    Some(Moment {
        local: text::utc(instant),
        offset: 0,
    })
}

impl<'r> Decode<'r, Sqlite> for Timestamp {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let instant = moment(value)?.instant();
        instant
            .map(Self)
            .ok_or_else(|| "datetime out of range".into())
    }
}

/// The date and time as written: an offset in the text is dropped, not
/// applied, as `NaiveDateTime` read it.
impl<'r> Decode<'r, Sqlite> for DateTime {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(Self(moment(value)?.local))
    }
}

impl<'r> Decode<'r, Sqlite> for Date {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let text = <&str as Decode<Sqlite>>::decode(value)?;
        parse::parse(text, parse::DATE_ONLY)
            .and_then(|parsed| parsed.date())
            .map(Self)
            .ok_or_else(|| format!("invalid date: {text}").into())
    }
}

impl<'r> Decode<'r, Sqlite> for Time {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let text = <&str as Decode<Sqlite>>::decode(value)?;
        parse::sqlite_time(text)
            .map(Self)
            .ok_or_else(|| format!("invalid time: {text}").into())
    }
}

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod tests;
