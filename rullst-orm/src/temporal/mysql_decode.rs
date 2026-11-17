//! `Decode` for MySQL/MariaDB: the value's bytes go to the parent's functions,
//! which the unit tests cover; the live MySQL and MariaDB matrices
//! (`matrix_mysql`, `matrix_mariadb`, `tests/driver_contract/jiff_temporal.rs`)
//! cover these.

use sqlx::error::BoxDynError;
use sqlx::mysql::MySqlValueRef;
use sqlx::{Decode, MySql};

use super::super::{Date, DateTime, Time, Timestamp};

/// The value's bytes as the row holds them (with the length prefix of the
/// binary protocol).
fn bytes(value: MySqlValueRef<'_>) -> Result<&[u8], BoxDynError> {
    <&[u8] as Decode<MySql>>::decode(value)
}

impl<'r> Decode<'r, MySql> for Timestamp {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        super::instant_from_value(bytes(value)?).map(Self)
    }
}

impl<'r> Decode<'r, MySql> for DateTime {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        super::date_time_from_value(bytes(value)?).map(Self)
    }
}

impl<'r> Decode<'r, MySql> for Date {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        super::date_from_value(bytes(value)?).map(Self)
    }
}

impl<'r> Decode<'r, MySql> for Time {
    fn decode(value: MySqlValueRef<'r>) -> Result<Self, BoxDynError> {
        super::time_from_value(bytes(value)?).map(Self)
    }
}
