//! Instants as RFC 3339 text, byte for byte as chrono 0.4 wrote and read
//! them.
//!
//! Rullst crates used chrono until 13.0.0-alpha.6 and now use jiff. These
//! helpers keep the strings that users, peers and storage saw: chrono printed
//! the fewest of 0, 3, 6 or 9 fractional digits that keep the value, a year
//! outside 0 to 9999 with a sign, and minute offsets; its RFC 3339 parser is
//! stricter than jiff's. Shared by the Rullst crates; not a stable API.

use jiff::Timestamp;

#[path = "rfc3339_parse.rs"]
mod parse;

pub use parse::parse_unix_seconds;

/// Fractional seconds written by [`format_utc`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fraction {
    /// None, or 3, 6 or 9 digits, the fewest that keep the value (chrono's
    /// `SecondsFormat::AutoSi`, also its `serde` form and `to_rfc3339`).
    Auto,
    /// Exactly 3 digits (`SecondsFormat::Millis`).
    Millis,
    /// Exactly 9 digits (`SecondsFormat::Nanos`).
    Nanos,
}

/// The Unix seconds (rounded down) and nanoseconds (0 to 999,999,999) of
/// `timestamp`, as chrono's `timestamp()` and `timestamp_subsec_nanos()`.
pub fn unix_parts(timestamp: Timestamp) -> (i64, u32) {
    let seconds = timestamp.as_second();
    let nanos = timestamp.subsec_nanosecond();
    if nanos < 0 {
        (seconds - 1, nanos.unsigned_abs().abs_diff(1_000_000_000))
    } else {
        (seconds, nanos.unsigned_abs())
    }
}

/// `timestamp` in UTC with a `Z`, as chrono's `to_rfc3339_opts(.., true)`.
pub fn format_utc(timestamp: Timestamp, fraction: Fraction) -> String {
    let (seconds, nanos) = unix_parts(timestamp);
    let mut out = String::with_capacity(32);
    write(&mut out, seconds, nanos, fraction, 0, true);
    out
}

/// `timestamp` at `offset_seconds` from UTC, with an offset such as
/// `+00:00` even for UTC, as chrono's `to_rfc3339()`.
pub fn format_with_offset(timestamp: Timestamp, offset_seconds: i32) -> String {
    let (seconds, nanos) = unix_parts(timestamp);
    let mut out = String::with_capacity(32);
    write(&mut out, seconds, nanos, Fraction::Auto, offset_seconds, false);
    out
}

/// The instant `millis` after the Unix epoch in UTC with a `Z`, or `None`
/// outside the range chrono's `DateTime::from_timestamp_millis` accepted.
pub fn format_unix_millis_utc(millis: i64, fraction: Fraction) -> Option<String> {
    if !(MIN_MILLIS..=MAX_MILLIS).contains(&millis) {
        return None;
    }
    let nanos = u32::try_from(millis.rem_euclid(1_000) * 1_000_000).ok()?;
    let mut out = String::with_capacity(32);
    write(
        &mut out,
        millis.div_euclid(1_000),
        nanos,
        fraction,
        0,
        true,
    );
    Some(out)
}

/// The UTC date of `timestamp` as `YYYY-MM-DD`, as chrono's `%Y-%m-%d`.
pub fn utc_date(timestamp: Timestamp) -> String {
    let (seconds, _) = unix_parts(timestamp);
    let mut out = String::with_capacity(10);
    write_date(&mut out, civil_from_days(seconds.div_euclid(SECONDS_PER_DAY)));
    out
}

/// Reads an instant written by chrono or by [`format_utc`]: RFC 3339 and
/// the other forms jiff parses, then what chrono's RFC 3339 parser also
/// accepted (a U+2212 minus sign in the offset) and years chrono wrote with a
/// sign (`-0001-12-31T23:59:59Z`).
pub fn parse_timestamp(text: &str) -> Option<Timestamp> {
    text.parse::<Timestamp>().ok().or_else(|| {
        let (seconds, nanos) = parse::parse_parts(text, true)?;
        Timestamp::new(seconds, i32::try_from(nanos).ok()?).ok()
    })
}

/// `serde` for a [`Timestamp`] in chrono's `DateTime<Utc>` form:
/// `#[serde(with = "rullst_core::rfc3339::serde_utc")]`.
pub mod serde_utc {
    use super::{Fraction, format_utc, parse_timestamp};
    use jiff::Timestamp;

    /// Writes `2026-10-22T15:45:30.250Z`.
    pub fn serialize<S: serde::Serializer>(
        timestamp: &Timestamp,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format_utc(*timestamp, Fraction::Auto))
    }

    /// Reads what [`super::parse_timestamp`] reads.
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Timestamp, D::Error> {
        deserializer.deserialize_str(Visitor)
    }

    pub(super) struct Visitor;

    impl serde::de::Visitor<'_> for Visitor {
        type Value = Timestamp;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an RFC 3339 date and time with an offset")
        }

        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Timestamp, E> {
            parse_timestamp(value).ok_or_else(|| E::custom("invalid RFC 3339 date and time"))
        }
    }
}

/// [`serde_utc`] for an optional [`Timestamp`]; pair it with
/// `#[serde(default)]` so a missing field stays `None`.
pub mod serde_utc_option {
    use jiff::Timestamp;

    struct Wrapped(Timestamp);

    impl serde::Serialize for Wrapped {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            super::serde_utc::serialize(&self.0, serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for Wrapped {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            super::serde_utc::deserialize(deserializer).map(Wrapped)
        }
    }

    /// Writes `null` or [`super::serde_utc`]'s form.
    pub fn serialize<S: serde::Serializer>(
        timestamp: &Option<Timestamp>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match timestamp {
            Some(timestamp) => serializer.serialize_some(&Wrapped(*timestamp)),
            None => serializer.serialize_none(),
        }
    }

    /// Reads `null` or [`super::serde_utc`]'s form.
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Timestamp>, D::Error> {
        <Option<Wrapped> as serde::Deserialize>::deserialize(deserializer)
            .map(|value| value.map(|wrapped| wrapped.0))
    }
}

/// Milliseconds of chrono's `DateTime::<Utc>::MIN_UTC` and `MAX_UTC`.
const MIN_MILLIS: i64 = -8_334_601_228_800_000;
const MAX_MILLIS: i64 = 8_210_266_876_799_999;

const SECONDS_PER_DAY: i64 = 86_400;

/// Writes chrono's `write_rfc3339` for the instant `seconds` + `nanos` at
/// `offset_seconds` from UTC.
fn write(
    out: &mut String,
    seconds: i64,
    nanos: u32,
    fraction: Fraction,
    offset_seconds: i32,
    zulu: bool,
) {
    let local = seconds.saturating_add(i64::from(offset_seconds));
    write_date(out, civil_from_days(local.div_euclid(SECONDS_PER_DAY)));
    let of_day = local.rem_euclid(SECONDS_PER_DAY);
    out.push('T');
    push_two(out, of_day / 3_600);
    out.push(':');
    push_two(out, of_day / 60 % 60);
    out.push(':');
    push_two(out, of_day % 60);
    match fraction {
        Fraction::Auto if nanos == 0 => {}
        Fraction::Auto if nanos % 1_000_000 == 0 => push_fraction(out, nanos / 1_000_000, 3),
        Fraction::Auto if nanos % 1_000 == 0 => push_fraction(out, nanos / 1_000, 6),
        Fraction::Auto | Fraction::Nanos => push_fraction(out, nanos, 9),
        Fraction::Millis => push_fraction(out, nanos / 1_000_000, 3),
    }
    if zulu && offset_seconds == 0 {
        out.push('Z');
        return;
    }
    out.push(if offset_seconds < 0 { '-' } else { '+' });
    // chrono rounds the offset's seconds to the nearest minute.
    let minutes = (i64::from(offset_seconds).abs() + 30) / 60;
    push_two(out, minutes / 60);
    out.push(':');
    push_two(out, minutes % 60);
}

/// `YYYY-MM-DD`, with a sign and at least four digits for a year outside 0
/// to 9999.
fn write_date(out: &mut String, (year, month, day): (i64, i64, i64)) {
    if (0..=9_999).contains(&year) {
        out.push_str(&format!("{year:04}"));
    } else {
        out.push_str(&format!("{year:+05}"));
    }
    out.push('-');
    push_two(out, month);
    out.push('-');
    push_two(out, day);
}

fn push_two(out: &mut String, value: i64) {
    out.push_str(&format!("{value:02}"));
}

fn push_fraction(out: &mut String, value: u32, digits: usize) {
    out.push_str(&format!(".{value:0digits$}"));
}

/// The proleptic Gregorian date `days` after 1970-01-01.
pub(crate) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (year_of_era + era * 400 + i64::from(month <= 2), month, day)
}

/// Days since 1970-01-01 of a proleptic Gregorian date.
pub(crate) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
#[path = "rfc3339_tests.rs"]
mod tests;
