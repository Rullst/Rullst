//! Date and time types for model fields, query values and [`RullstValue`].
//!
//! [`Timestamp`] is a UTC instant, [`DateTime`] a date and time without a
//! zone, [`Date`] a calendar date and [`Time`] a time of day. Each wraps the
//! [`jiff`] type of the same name (`jiff::Timestamp`,
//! `jiff::civil::{DateTime, Date, Time}`), dereferences to it and converts to
//! and from it, so jiff's arithmetic, rounding and time zones apply; the ORM
//! re-exports the crate as `rullst_orm::jiff`.
//!
//! The wrappers exist because SQLx has no jiff codecs and Rust's orphan rule
//! keeps the ORM from adding them to jiff's types. They bind and decode with
//! the strict driver features: `TIMESTAMPTZ`, `TIMESTAMP`, `DATE` and `TIME`
//! on PostgreSQL, `DATETIME(6)`/`TIMESTAMP`, `DATETIME`, `DATE` and `TIME` on
//! MySQL/MariaDB, and ISO-8601 `TEXT` on SQLite (see `Blueprint::timestamp_tz`
//! and its siblings). What they write is byte for byte what the ORM wrote for
//! the `chrono` types they replace (`DateTime<Utc>`, `NaiveDateTime`,
//! `NaiveDate`, `NaiveTime`), and they read every form those types read, so
//! existing rows and new ones mix in one column. Three limits come from jiff:
//! years run from -9999 to 9999 (a `Timestamp` from -9999-01-02T01:59:59Z to
//! 9999-12-30T22:00:00.999999999Z), so a stored value outside that range is a
//! decode error; a leap second (`:60`) reads as second 59 with the same
//! fraction, as jiff parses it; and `Default` is jiff's (the Unix epoch for
//! `Timestamp`, midnight for `Time`, 0000-01-01 for `Date` and `DateTime`).
//!
//! `Display`, `FromStr` and serde use jiff's ISO 8601 / RFC 3339 forms
//! (`2026-10-15T09:30:00.5Z` for a `Timestamp`), which are not the database
//! text forms.
//!
//! ```
//! use rullst_orm::{RullstValue, Timestamp};
//!
//! let at: Timestamp = "2026-10-15T09:30:00.5Z".parse().unwrap();
//! assert_eq!(at.as_millisecond(), 1_792_056_600_500);
//! assert_eq!(at.to_string(), "2026-10-15T09:30:00.5Z");
//! assert!(matches!(RullstValue::from(at), RullstValue::Timestamp(_)));
//! ```

use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use crate::RullstValue;

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

macro_rules! temporal_type {
    ($(#[$doc:meta])* $name:ident($inner:ty) => $variant:ident) => {
        $(#[$doc])*
        #[derive(
            Clone,
            Copy,
            Default,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        #[repr(transparent)]
        pub struct $name($inner);

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.0, f)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl FromStr for $name {
            type Err = jiff::Error;

            fn from_str(text: &str) -> Result<Self, jiff::Error> {
                text.parse().map(Self)
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
    /// PostgreSQL stores microseconds and truncates the rest, as before; a
    /// MySQL/MariaDB `DATETIME(6)` likewise.
    Timestamp(jiff::Timestamp) => Timestamp
}

temporal_type! {
    /// A date and time without a zone, for `timestamp()` columns: wraps
    /// [`jiff::civil::DateTime`] (replaces `chrono::NaiveDateTime`).
    DateTime(jiff::civil::DateTime) => CivilDateTime
}

temporal_type! {
    /// A calendar date, for `date()` columns: wraps [`jiff::civil::Date`]
    /// (replaces `chrono::NaiveDate`).
    Date(jiff::civil::Date) => CivilDate
}

temporal_type! {
    /// A time of day, for `time()` columns: wraps [`jiff::civil::Time`]
    /// (replaces `chrono::NaiveTime`).
    Time(jiff::civil::Time) => CivilTime
}

impl Timestamp {
    /// The current time from the system clock ([`jiff::Timestamp::now`]).
    pub fn now() -> Self {
        Self(jiff::Timestamp::now())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
