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
use sqlx::postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueFormat, PgValueRef};
use sqlx::types::time as sqlx_time;
use sqlx::{Decode, Encode, Postgres, Type};

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

/// A binary `TIMESTAMPTZ`: microseconds since 2000 in UTC.
pub(super) fn instant_from_micros(micros: i64) -> Option<jiff::Timestamp> {
    EPOCH_INSTANT
        .checked_add(SignedDuration::from_micros(micros))
        .ok()
}

/// A binary `TIMESTAMP`.
pub(super) fn date_time_from_micros(micros: i64) -> Option<civil::DateTime> {
    EPOCH
        .checked_add(SignedDuration::from_micros(micros))
        .ok()
}

/// A binary `DATE`.
pub(super) fn date_from_days(days: i32) -> Option<civil::Date> {
    EPOCH_DATE
        .checked_add(SignedDuration::from_secs(i64::from(days) * 86_400))
        .ok()
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

fn decoded<T>(value: Option<T>, what: &str) -> Result<T, BoxDynError> {
    value.ok_or_else(|| format!("invalid or out-of-range PostgreSQL {what}").into())
}

impl<'r> Decode<'r, Postgres> for Timestamp {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let instant = match value.format() {
            PgValueFormat::Binary => instant_from_micros(Decode::<Postgres>::decode(value)?),
            PgValueFormat::Text => instant_from_text(value.as_str()?),
        };
        decoded(instant, "TIMESTAMPTZ").map(Self)
    }
}

impl<'r> Decode<'r, Postgres> for DateTime {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let date_time = match value.format() {
            PgValueFormat::Binary => date_time_from_micros(Decode::<Postgres>::decode(value)?),
            PgValueFormat::Text => date_time_from_text(value.as_str()?),
        };
        decoded(date_time, "TIMESTAMP").map(Self)
    }
}

impl<'r> Decode<'r, Postgres> for Date {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let date = match value.format() {
            PgValueFormat::Binary => date_from_days(Decode::<Postgres>::decode(value)?),
            PgValueFormat::Text => {
                parse::parse(value.as_str()?, parse::DATE_ONLY).and_then(|parsed| parsed.date())
            }
        };
        decoded(date, "DATE").map(Self)
    }
}

impl<'r> Decode<'r, Postgres> for Time {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let time = match value.format() {
            PgValueFormat::Binary => Some(time_from_micros(Decode::<Postgres>::decode(value)?)),
            PgValueFormat::Text => {
                parse::parse(value.as_str()?, parse::TIME).and_then(|parsed| parsed.time())
            }
        };
        decoded(time, "TIME").map(Self)
    }
}

#[cfg(test)]
#[path = "postgres_tests.rs"]
mod tests;
