//! `Decode` for PostgreSQL: the value's format and bytes go to the parent's
//! functions, which the unit tests cover; the live PostgreSQL matrix
//! (`matrix_postgres`, `tests/driver_contract/jiff_temporal.rs`) covers these.

use sqlx::error::BoxDynError;
use sqlx::postgres::PgValueRef;
use sqlx::{Decode, Postgres};

use super::super::{Date, DateTime, Time, Timestamp};

impl<'r> Decode<'r, Postgres> for Timestamp {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        super::instant_from_value(value.format(), value.as_bytes()?).map(Self)
    }
}

impl<'r> Decode<'r, Postgres> for DateTime {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        super::date_time_from_value(value.format(), value.as_bytes()?).map(Self)
    }
}

impl<'r> Decode<'r, Postgres> for Date {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        super::date_from_value(value.format(), value.as_bytes()?).map(Self)
    }
}

impl<'r> Decode<'r, Postgres> for Time {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        super::time_from_value(value.format(), value.as_bytes()?).map(Self)
    }
}
