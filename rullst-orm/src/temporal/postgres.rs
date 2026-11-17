//! PostgreSQL: the binary encoding SQLx's `chrono` codecs used.
//!
//! `TIMESTAMPTZ` and `TIMESTAMP` are microseconds since 2000-01-01 00:00
//! (UTC for an instant), truncated toward 2000 (so a pre-2000 value with a
//! sub-microsecond part moves up to the next microsecond), `DATE` is days
//! since 2000-01-01 and `TIME` microseconds since midnight, truncated. Text
//! results (from the simple query protocol) are read with the `chrono`
//! formats ([`super::parse`]). A stored value outside jiff's range is an
//! error, where `chrono` panicked or accepted it; a `TIME` of `24:00:00`
//! reads as midnight, as before.

use jiff::{SignedDuration, civil};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueFormat};
use sqlx::types::time as sqlx_time;
use sqlx::{Encode, Postgres, Type};

use super::parse::{self, Moment};
use super::{Date, DateTime, Time, Timestamp};

/// 2000-01-01T00:00:00Z, the origin of PostgreSQL's binary forms.
const EPOCH_INSTANT: jiff::Timestamp = jiff::Timestamp::constant(946_684_800, 0);
const EPOCH: civil::DateTime = civil::DateTime::constant(2000, 1, 1, 0, 0, 0, 0);
const EPOCH_DATE: civil::Date = civil::Date::constant(2000, 1, 1);

/// The type descriptors (OIDs) of SQLx's `time` codecs, the same as its
/// `chrono` ones: `PgTypeInfo`'s constants are private to SQLx.
macro_rules! pg_type {
    ($name:ty, $time:ty) => {
        impl Type<Postgres> for $name {
            fn type_info() -> PgTypeInfo {
                <$time as Type<Postgres>>::type_info()
            }
        }

        impl PgHasArrayType for $name {
            fn array_type_info() -> PgTypeInfo {
                <$time as PgHasArrayType>::array_type_info()
            }
        }
    };
}

pg_type!(Timestamp, sqlx_time::OffsetDateTime);
pg_type!(DateTime, sqlx_time::PrimitiveDateTime);
pg_type!(Date, sqlx_time::Date);
pg_type!(Time, sqlx_time::Time);

/// Whole microseconds of `duration`, truncated toward zero.
fn micros(duration: SignedDuration) -> Result<i64, BoxDynError> {
    Ok(i64::try_from(duration.as_micros())?)
}

impl Encode<'_, Postgres> for Timestamp {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Postgres>::encode(micros(self.0.duration_since(EPOCH_INSTANT))?, buf)
    }

    fn size_hint(&self) -> usize {
        size_of::<i64>()
    }
}

impl Encode<'_, Postgres> for DateTime {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        Encode::<Postgres>::encode(micros(self.0.duration_since(EPOCH))?, buf)
    }

    fn size_hint(&self) -> usize {
        size_of::<i64>()
    }
}

impl Encode<'_, Postgres> for Date {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let days = self.0.duration_since(EPOCH_DATE).as_secs() / 86_400;
        Encode::<Postgres>::encode(i32::try_from(days)?, buf)
    }

    fn size_hint(&self) -> usize {
        size_of::<i32>()
    }
}

impl Encode<'_, Postgres> for Time {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let micros = micros(self.0.duration_since(civil::Time::midnight()))?;
        Encode::<Postgres>::encode(micros, buf)
    }

    fn size_hint(&self) -> usize {
        size_of::<u64>()
    }
}

/// Days from 1970-01-01 to 2000-01-01.
const EPOCH_UNIX_DAYS: i64 = 10_957;
const MICROS_PER_DAY: i64 = 86_400_000_000;

/// The date `days` after 1970-01-01. jiff bounds a day count added to a date
/// by its distance from 1970, so the day counts start there.
fn date_from_unix_days(days: i64) -> Option<civil::Date> {
    civil::Date::constant(1970, 1, 1)
        .checked_add(SignedDuration::from_secs(days.checked_mul(86_400)?))
        .ok()
}

/// A binary `TIMESTAMPTZ`: microseconds since 2000 in UTC. (jiff's
/// `from_microsecond` refuses a fraction of its last second, so nanoseconds.)
pub(super) fn instant_from_micros(micros: i64) -> Option<jiff::Timestamp> {
    let since_1970 = i128::from(micros) + i128::from(EPOCH_UNIX_DAYS * MICROS_PER_DAY);
    jiff::Timestamp::from_nanosecond(since_1970 * 1000).ok()
}

/// A binary `TIMESTAMP`.
pub(super) fn date_time_from_micros(micros: i64) -> Option<civil::DateTime> {
    let since_1970 = i128::from(micros) + i128::from(EPOCH_UNIX_DAYS * MICROS_PER_DAY);
    let days = i64::try_from(since_1970.div_euclid(i128::from(MICROS_PER_DAY))).ok()?;
    let time_of_day = i64::try_from(since_1970.rem_euclid(i128::from(MICROS_PER_DAY))).ok()?;
    let time = civil::Time::midnight().wrapping_add(SignedDuration::from_micros(time_of_day));
    Some(date_from_unix_days(days)?.to_datetime(time))
}

/// A binary `DATE`.
pub(super) fn date_from_days(days: i32) -> Option<civil::Date> {
    date_from_unix_days(i64::from(days) + EPOCH_UNIX_DAYS)
}

/// A binary `TIME`, wrapped into one day as `chrono` added it to midnight.
pub(super) fn time_from_micros(micros: i64) -> civil::Time {
    civil::Time::midnight().wrapping_add(SignedDuration::from_micros(micros))
}

/// `TIMESTAMPTZ` text has an offset (`+00`); `chrono` required a valid one.
pub(super) fn instant_from_text(text: &str) -> Option<jiff::Timestamp> {
    if !text.contains(['+', '-']) {
        return None;
    }
    let parsed = parse::parse(text, parse::DATE_TIME_OFFSET)?;
    let moment = Moment {
        local: parsed.date_time()?,
        offset: parsed.valid_offset()?,
    };
    moment.instant()
}

/// `TIMESTAMP` text; an offset after a `+` is read and dropped.
pub(super) fn date_time_from_text(text: &str) -> Option<civil::DateTime> {
    let items = if text.contains('+') {
        parse::DATE_TIME_OFFSET
    } else {
        parse::DATE_TIME
    };
    parse::parse(text, items)?.date_time()
}

/// A binary integer of `N` bytes, big endian; another length is an error.
fn big_endian<const N: usize>(bytes: &[u8]) -> Result<[u8; N], BoxDynError> {
    Ok(bytes.try_into()?)
}

fn decoded<T>(value: Option<T>, what: &str) -> Result<T, BoxDynError> {
    value.ok_or_else(|| format!("invalid or out-of-range PostgreSQL {what}").into())
}

/// A `TIMESTAMPTZ` value in either result format.
pub(super) fn instant_from_value(
    format: PgValueFormat,
    bytes: &[u8],
) -> Result<jiff::Timestamp, BoxDynError> {
    let instant = match format {
        PgValueFormat::Binary => instant_from_micros(i64::from_be_bytes(big_endian(bytes)?)),
        PgValueFormat::Text => instant_from_text(std::str::from_utf8(bytes)?),
    };
    decoded(instant, "TIMESTAMPTZ")
}

/// A `TIMESTAMP` value in either result format.
pub(super) fn date_time_from_value(
    format: PgValueFormat,
    bytes: &[u8],
) -> Result<civil::DateTime, BoxDynError> {
    let date_time = match format {
        PgValueFormat::Binary => date_time_from_micros(i64::from_be_bytes(big_endian(bytes)?)),
        PgValueFormat::Text => date_time_from_text(std::str::from_utf8(bytes)?),
    };
    decoded(date_time, "TIMESTAMP")
}

/// A `DATE` value in either result format.
pub(super) fn date_from_value(
    format: PgValueFormat,
    bytes: &[u8],
) -> Result<civil::Date, BoxDynError> {
    let date = match format {
        PgValueFormat::Binary => date_from_days(i32::from_be_bytes(big_endian(bytes)?)),
        PgValueFormat::Text => parse::parse(std::str::from_utf8(bytes)?, parse::DATE_ONLY)
            .and_then(|parsed| parsed.date()),
    };
    decoded(date, "DATE")
}

/// A `TIME` value in either result format.
pub(super) fn time_from_value(
    format: PgValueFormat,
    bytes: &[u8],
) -> Result<civil::Time, BoxDynError> {
    let time = match format {
        PgValueFormat::Binary => Some(time_from_micros(i64::from_be_bytes(big_endian(bytes)?))),
        PgValueFormat::Text => {
            parse::parse(std::str::from_utf8(bytes)?, parse::TIME).and_then(|parsed| parsed.time())
        }
    };
    decoded(time, "TIME")
}

// The `Decode` implementations only hand the value's format and bytes to the
// functions above; they need a live server.
#[path = "postgres_decode.rs"]
mod decode;

#[cfg(test)]
#[path = "postgres_tests.rs"]
mod tests;
