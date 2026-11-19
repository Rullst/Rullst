//! Arithmetic and accessors that return the ORM's types.
//!
//! Each method has the name and parameters of the jiff method it shadows
//! (`Deref` would otherwise reach jiff's and return a jiff value), and returns
//! the ORM type instead: `checked_add`, `checked_sub`, `saturating_add`,
//! `saturating_sub`, `duration_until` and `duration_since` on [`Timestamp`],
//! [`DateTime`] and [`Date`]; `tomorrow` and `yesterday` on `DateTime` and
//! `Date`; `date` and `time` on `DateTime`. The other parameter of a
//! difference may be the ORM type or jiff's. Nothing here panics: there are no
//! `Add`/`Sub` operators (jiff's panic on overflow); a difference is
//! `duration_since`, which cannot overflow.

use jiff::civil::{DateArithmetic, DateTimeArithmetic};
use jiff::{SignedDuration, TimestampArithmetic};

use super::{Date, DateTime, Time, Timestamp, text};

impl Timestamp {
    /// This instant plus `duration` (a `jiff::SignedDuration`, a
    /// `std::time::Duration` or a `jiff::Span` of hours or smaller units); an
    /// error outside the supported range ([`jiff::Timestamp::checked_add`]).
    pub fn checked_add<A: Into<TimestampArithmetic>>(
        self,
        duration: A,
    ) -> Result<Self, jiff::Error> {
        self.0.checked_add(duration).map(Self)
    }

    /// This instant minus `duration` ([`jiff::Timestamp::checked_sub`]).
    pub fn checked_sub<A: Into<TimestampArithmetic>>(
        self,
        duration: A,
    ) -> Result<Self, jiff::Error> {
        self.0.checked_sub(duration).map(Self)
    }

    /// This instant plus `duration`, clamped to the supported range; an error
    /// only for a `Span` with units above hours ([`jiff::Timestamp::saturating_add`]).
    pub fn saturating_add<A: Into<TimestampArithmetic>>(
        self,
        duration: A,
    ) -> Result<Self, jiff::Error> {
        self.0.saturating_add(duration).map(Self)
    }

    /// This instant minus `duration`, clamped to the supported range
    /// ([`jiff::Timestamp::saturating_sub`]).
    pub fn saturating_sub<A: Into<TimestampArithmetic>>(
        self,
        duration: A,
    ) -> Result<Self, jiff::Error> {
        self.0.saturating_sub(duration).map(Self)
    }

    /// The exact time from this instant until `other` (negative when `other`
    /// is earlier).
    pub fn duration_until(self, other: impl Into<jiff::Timestamp>) -> SignedDuration {
        self.0.duration_until(other.into())
    }

    /// The exact time from `other` until this instant.
    pub fn duration_since(self, other: impl Into<jiff::Timestamp>) -> SignedDuration {
        self.0.duration_since(other.into())
    }

    /// RFC 3339 with a `+00:00` offset and the shortest of 0, 3, 6 or 9
    /// fraction digits (`2026-10-15T09:30:00.500+00:00`): chrono's
    /// `DateTime<Utc>::to_rfc3339`, byte for byte.
    pub fn to_rfc3339(self) -> String {
        text::timestamp(self)
    }
}

impl DateTime {
    /// This date-time plus `duration` (a `jiff::Span`, a
    /// `jiff::SignedDuration` or a `std::time::Duration`); an error outside
    /// the supported range ([`jiff::civil::DateTime::checked_add`]).
    pub fn checked_add<A: Into<DateTimeArithmetic>>(
        self,
        duration: A,
    ) -> Result<Self, jiff::Error> {
        self.0.checked_add(duration).map(Self)
    }

    /// This date-time minus `duration` ([`jiff::civil::DateTime::checked_sub`]).
    pub fn checked_sub<A: Into<DateTimeArithmetic>>(
        self,
        duration: A,
    ) -> Result<Self, jiff::Error> {
        self.0.checked_sub(duration).map(Self)
    }

    /// This date-time plus `duration`, clamped to the supported range.
    pub fn saturating_add<A: Into<DateTimeArithmetic>>(self, duration: A) -> Self {
        Self(self.0.saturating_add(duration))
    }

    /// This date-time minus `duration`, clamped to the supported range.
    pub fn saturating_sub<A: Into<DateTimeArithmetic>>(self, duration: A) -> Self {
        Self(self.0.saturating_sub(duration))
    }

    /// The exact time (days of 24 hours) from this date-time until `other`.
    pub fn duration_until(self, other: impl Into<jiff::civil::DateTime>) -> SignedDuration {
        self.0.duration_until(other.into())
    }

    /// The exact time (days of 24 hours) from `other` until this date-time.
    pub fn duration_since(self, other: impl Into<jiff::civil::DateTime>) -> SignedDuration {
        self.0.duration_since(other.into())
    }

    /// The same time on the next day; an error past the supported range.
    pub fn tomorrow(self) -> Result<Self, jiff::Error> {
        self.0.tomorrow().map(Self)
    }

    /// The same time on the previous day; an error before the supported range.
    pub fn yesterday(self) -> Result<Self, jiff::Error> {
        self.0.yesterday().map(Self)
    }

    /// The date part.
    pub fn date(self) -> Date {
        Date(self.0.date())
    }

    /// The time part.
    pub fn time(self) -> Time {
        Time(self.0.time())
    }
}

impl Date {
    /// This date plus `duration` (a `jiff::Span`, a `jiff::SignedDuration`
    /// counted in whole days or a `std::time::Duration`); an error outside the
    /// supported range ([`jiff::civil::Date::checked_add`]).
    pub fn checked_add<A: Into<DateArithmetic>>(self, duration: A) -> Result<Self, jiff::Error> {
        self.0.checked_add(duration).map(Self)
    }

    /// This date minus `duration` ([`jiff::civil::Date::checked_sub`]).
    pub fn checked_sub<A: Into<DateArithmetic>>(self, duration: A) -> Result<Self, jiff::Error> {
        self.0.checked_sub(duration).map(Self)
    }

    /// This date plus `duration`, clamped to the supported range.
    pub fn saturating_add<A: Into<DateArithmetic>>(self, duration: A) -> Self {
        Self(self.0.saturating_add(duration))
    }

    /// This date minus `duration`, clamped to the supported range.
    pub fn saturating_sub<A: Into<DateArithmetic>>(self, duration: A) -> Self {
        Self(self.0.saturating_sub(duration))
    }

    /// The exact time (days of 24 hours) from this date until `other`.
    pub fn duration_until(self, other: impl Into<jiff::civil::Date>) -> SignedDuration {
        self.0.duration_until(other.into())
    }

    /// The exact time (days of 24 hours) from `other` until this date.
    pub fn duration_since(self, other: impl Into<jiff::civil::Date>) -> SignedDuration {
        self.0.duration_since(other.into())
    }

    /// The next day; an error past the supported range.
    pub fn tomorrow(self) -> Result<Self, jiff::Error> {
        self.0.tomorrow().map(Self)
    }

    /// The previous day; an error before the supported range.
    pub fn yesterday(self) -> Result<Self, jiff::Error> {
        self.0.yesterday().map(Self)
    }

    /// The number of days from this date until `other` (negative when
    /// `other` is earlier).
    pub fn days_until(self, other: impl Into<jiff::civil::Date>) -> i64 {
        self.duration_until(other).as_secs() / 86_400
    }

    /// The number of days from `other` until this date.
    pub fn days_since(self, other: impl Into<jiff::civil::Date>) -> i64 {
        self.duration_since(other).as_secs() / 86_400
    }
}

macro_rules! from_reference {
    ($($name:ident => $inner:ty),+ $(,)?) => {
        $(
            impl From<&$name> for $inner {
                fn from(value: &$name) -> Self {
                    value.0
                }
            }
        )+
    };
}

from_reference! {
    Timestamp => jiff::Timestamp,
    DateTime => jiff::civil::DateTime,
    Date => jiff::civil::Date,
    Time => jiff::civil::Time,
}

#[cfg(test)]
#[path = "arithmetic_tests.rs"]
mod tests;
