//! Date and time types for model fields, query values and [`RullstValue`].
//!
//! [`Timestamp`] is a UTC instant, [`DateTime`] a date and time without a
//! zone, [`Date`] a calendar date and [`Time`] a time of day. Each wraps the
//! [`jiff`] type of the same name (`jiff::Timestamp`,
//! `jiff::civil::{DateTime, Date, Time}`), dereferences to it and converts to
//! and from it, so jiff's arithmetic, rounding and time zones apply; the ORM
//! re-exports the crate as `rullst_orm::jiff`. They replace the `chrono` types
//! `DateTime<Utc>`, `NaiveDateTime`, `NaiveDate` and `NaiveTime`, and behave as
//! those did wherever the ORM or an application observes them:
//!
//! - **Storage.** With a strict driver feature they bind and decode as
//!   `TIMESTAMPTZ`, `TIMESTAMP`, `DATE` and `TIME` on PostgreSQL,
//!   `DATETIME(6)`/`TIMESTAMP`, `DATETIME`, `DATE` and `TIME` on MySQL/MariaDB
//!   and ISO-8601 `TEXT` on SQLite (see `Blueprint::timestamp_tz` and its
//!   siblings), writing byte for byte what the `chrono` types wrote and
//!   reading every form those read, so old and new rows mix in one column.
//! - **Text.** `Display`, `Debug` and serde write chrono's text
//!   (`2026-10-15 09:30:00.500 UTC`, `2026-10-15T09:30:00.500Z`, …; each type
//!   documents its forms), and `FromStr` and serde read every text chrono's
//!   read, giving the same value, and jiff's own forms too.
//! - **`Default`.** chrono's: the Unix epoch (1970-01-01T00:00:00Z),
//!   1970-01-01T00:00:00, 1970-01-01 and midnight.
//!
//! The jiff value reached through `Deref` or `into()` is jiff's, with jiff's
//! text forms (`2026-10-15T09:30:00.5Z`); results of its arithmetic convert
//! back with `into()`:
//!
//! ```
//! use rullst_orm::jiff::SignedDuration;
//! use rullst_orm::{RullstValue, Timestamp};
//!
//! let at: Timestamp = "2026-10-15T09:30:00.5Z".parse().unwrap();
//! assert_eq!(at.to_string(), "2026-10-15 09:30:00.500 UTC");
//! assert_eq!(at.as_millisecond(), 1_792_056_600_500);
//! let later: Timestamp = at.checked_add(SignedDuration::from_hours(1)).unwrap().into();
//! assert_eq!(later.to_string(), "2026-10-15 10:30:00.500 UTC");
//! assert!(matches!(RullstValue::from(later), RullstValue::Timestamp(_)));
//! ```
//!
//! # Limits
//!
//! jiff's range is narrower than chrono's (years up to ±262,142). A value
//! outside it is a decode or parse error, never a panic, and values are never
//! clamped:
//!
//! - [`DateTime`], [`Date`] and [`Time`] (`timestamp`, `date` and `time`
//!   columns) hold years -9999 to 9999.
//! - [`Timestamp`] holds -9999-01-02T01:59:59Z to
//!   9999-12-30T22:00:00.999999999Z, about a day less at each end. The
//!   common end-of-time sentinel `9999-12-31 23:59:59`, which `chrono` read
//!   from a PostgreSQL `timestamptz` or a MySQL/MariaDB `DATETIME`, is outside
//!   it. Find such rows before upgrading, for example with
//!   `SELECT … WHERE col > '9999-12-30 22:00:00+00'` (PostgreSQL) or
//!   `WHERE col > '9999-12-30 22:00:00'` (MySQL/MariaDB); a `timestamp`
//!   column read as [`DateTime`] keeps the full range.
//! - PostgreSQL `infinity` and `-infinity` are errors (`chrono` panicked).
//! - jiff has no leap seconds: a stored leap second, which `chrono` kept as a
//!   value of its own, reads as second 59 with the same fraction, as jiff's
//!   own parser reads it (`23:59:60.5` reads as `23:59:59.5`).
//!
//! The crate root re-exports the four types, and so does the facade's
//! `rullst::db::*`; together with `use chrono::prelude::*` (or another glob
//! that brings a `DateTime`), the name `DateTime` is ambiguous. Import one of
//! them by name.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::str::FromStr;

use crate::RullstValue;

mod calendar;
#[cfg(any(feature = "drivers-all", feature = "strict-mysql"))]
mod mysql;
mod parse;
#[cfg(any(
    feature = "drivers-all",
    feature = "strict-postgres",
    feature = "pgvector"
))]
mod postgres;
#[cfg(any(feature = "drivers-all", feature = "strict-sqlite", feature = "turso"))]
mod sqlite;
pub(crate) mod text;

/// Reads text through chrono's `FromStr` rules first, then jiff's: every text
/// chrono accepted gives chrono's value, and jiff's own forms are accepted too.
fn parse_text<J: FromStr<Err = jiff::Error>>(
    text: &str,
    chrono_rules: fn(&str) -> Option<J>,
) -> Result<J, jiff::Error> {
    match chrono_rules(text) {
        Some(value) => Ok(value),
        None => text.parse(),
    }
}

/// Deserializes any of the four types from a string, as chrono's visitors did.
struct TextVisitor<T>(PhantomData<T>);

impl<T: FromStr<Err = jiff::Error>> serde::de::Visitor<'_> for TextVisitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a formatted date and time string")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<T, E> {
        value.parse().map_err(E::custom)
    }
}

macro_rules! temporal_type {
    (
        $(#[$doc:meta])*
        $name:ident($inner:ty) => $variant:ident,
        display: $display:path,
        debug_and_serde: $debug:path,
        chrono_rules: $rules:path $(,)?
    ) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name($inner);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&$display(*self))
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&$debug(*self))
            }
        }

        impl FromStr for $name {
            type Err = jiff::Error;

            fn from_str(text: &str) -> Result<Self, jiff::Error> {
                parse_text(text, $rules).map(Self)
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str($debug(*self).as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserializer.deserialize_str(TextVisitor(PhantomData))
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $inner {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &$inner {
                &self.0
            }
        }

        impl From<$name> for RullstValue {
            fn from(value: $name) -> Self {
                RullstValue::$variant(value)
            }
        }
    };
}

temporal_type! {
    /// A UTC instant with nanosecond precision, for `timestamp_tz()`
    /// columns: wraps [`jiff::Timestamp`] (replaces `chrono::DateTime<Utc>`).
    ///
    /// Its range is jiff's, -9999-01-02T01:59:59Z to
    /// 9999-12-30T22:00:00.999999999Z, narrower than the civil [`DateTime`]'s.
    /// PostgreSQL stores microseconds and truncates the rest, as before; a
    /// MySQL/MariaDB `DATETIME(6)` likewise. Text: `Display` is
    /// `2026-10-15 09:30:00.500 UTC`, `Debug` and serde
    /// `2026-10-15T09:30:00.500Z`, as chrono's `DateTime<Utc>`.
    Timestamp(jiff::Timestamp) => Timestamp,
    display: text::timestamp_display,
    debug_and_serde: text::timestamp_iso,
    chrono_rules: parse::timestamp_from_str,
}

temporal_type! {
    /// A date and time without a zone, for `timestamp()` columns: wraps
    /// [`jiff::civil::DateTime`] (replaces `chrono::NaiveDateTime`).
    ///
    /// Text: `Display` is `2026-10-15 09:30:00.500`, `Debug` and serde
    /// `2026-10-15T09:30:00.500`, as chrono's `NaiveDateTime`.
    DateTime(jiff::civil::DateTime) => CivilDateTime,
    display: text::date_time,
    debug_and_serde: text::date_time_iso,
    chrono_rules: parse::date_time_from_str,
}

temporal_type! {
    /// A calendar date, for `date()` columns: wraps [`jiff::civil::Date`]
    /// (replaces `chrono::NaiveDate`).
    ///
    /// Text: `2026-10-15` (`-0044-03-15` before year 0), as chrono's
    /// `NaiveDate`.
    Date(jiff::civil::Date) => CivilDate,
    display: text::date,
    debug_and_serde: text::date,
    chrono_rules: parse::date_from_str,
}

temporal_type! {
    /// A time of day, for `time()` columns: wraps [`jiff::civil::Time`]
    /// (replaces `chrono::NaiveTime`).
    ///
    /// Text: `09:30:00.500`, as chrono's `NaiveTime`.
    Time(jiff::civil::Time) => CivilTime,
    display: text::time,
    debug_and_serde: text::time,
    chrono_rules: parse::time_from_str,
}

impl Timestamp {
    /// The current time from the system clock ([`jiff::Timestamp::now`]).
    pub fn now() -> Self {
        Self(jiff::Timestamp::now())
    }
}

// The defaults are chrono's (the Unix epoch), not jiff's 0000-01-01, so a
// model that derives `Default` stores what it stored with the chrono types.

/// The Unix epoch, 1970-01-01T00:00:00Z.
impl Default for Timestamp {
    fn default() -> Self {
        Self(jiff::Timestamp::UNIX_EPOCH)
    }
}

/// 1970-01-01T00:00:00.
impl Default for DateTime {
    fn default() -> Self {
        Self(jiff::civil::DateTime::constant(1970, 1, 1, 0, 0, 0, 0))
    }
}

/// 1970-01-01.
impl Default for Date {
    fn default() -> Self {
        Self(jiff::civil::Date::constant(1970, 1, 1))
    }
}

/// Midnight, 00:00:00.
impl Default for Time {
    fn default() -> Self {
        Self(jiff::civil::Time::midnight())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "support_tests.rs"]
pub(crate) mod test_support;
