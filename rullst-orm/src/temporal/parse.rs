//! Reading the text forms that SQLx's `chrono` decoders accepted.
//!
//! The decoders of `rullst_orm`'s date/time types must read every value the
//! `chrono` types read, so this module reproduces chrono 0.4's parser for the
//! few format strings those decoders use: the same items (`%Y`, `%m`, `%d`,
//! `%H`, `%M`, `%S`, `%.f`, `%:z`, `%#z`, spaces and literals), the same
//! leniencies (whitespace before a number or an offset, a signed year of any
//! length, fraction digits past the ninth ignored) and the same range checks.
//! A leap second (`:60`) becomes second 59 with its fraction, as jiff's own
//! parser does, and a year outside jiff's -9999 to 9999 is rejected.

// Each pool uses part of this module; a build with one driver leaves the rest
// unused.
#![cfg_attr(not(feature = "drivers-all"), allow(dead_code))]

use jiff::civil;

/// One item of a chrono format string.
#[derive(Clone, Copy)]
pub(super) enum Item {
    /// `%Y`: an optional sign, then up to four digits, or any number of
    /// digits after a sign.
    Year,
    /// `%m`, `%d`, `%H`, `%M`, `%S`: one or two digits.
    Month,
    Day,
    Hour,
    Minute,
    Second,
    /// `%.f`: an optional `.` with at least one digit.
    Fraction,
    /// Text that must follow exactly.
    Lit(&'static str),
    /// A space in the format: any whitespace, or none.
    Space,
    /// `%:z`: `±HH:MM` (a colon or whitespace may separate the minutes).
    Offset,
    /// `%#z`: like `%:z`, and also `Z`, `z` or `±HH` at the end.
    OffsetOrZulu,
}

use Item::{Day, Fraction, Hour, Lit, Minute, Month, Offset, OffsetOrZulu, Second, Space, Year};

const DATE: [Item; 5] = [Year, Lit("-"), Month, Lit("-"), Day];

/// `%Y-%m-%d %H:%M:%S%.f`: PostgreSQL and MySQL text results.
pub(super) const DATE_TIME: &[Item] = &[
    Year,
    Lit("-"),
    Month,
    Lit("-"),
    Day,
    Space,
    Hour,
    Lit(":"),
    Minute,
    Lit(":"),
    Second,
    Fraction,
];

/// `%Y-%m-%d %H:%M:%S%.f%#z`: PostgreSQL `TIMESTAMPTZ` text results.
pub(super) const DATE_TIME_OFFSET: &[Item] = &[
    Year,
    Lit("-"),
    Month,
    Lit("-"),
    Day,
    Space,
    Hour,
    Lit(":"),
    Minute,
    Lit(":"),
    Second,
    Fraction,
    OffsetOrZulu,
];

/// `%H:%M:%S%.f`: PostgreSQL and MySQL text results.
pub(super) const TIME: &[Item] = &[Hour, Lit(":"), Minute, Lit(":"), Second, Fraction];

/// `%F`.
pub(super) const DATE_ONLY: &[Item] = &DATE;

const fn date_time(separator: Item, time: &'static [Item], suffix: &'static [Item]) -> [Item; 13] {
    let mut items = [Lit(""); 13];
    let mut index = 0;
    while index < 5 {
        items[index] = DATE[index];
        index += 1;
    }
    items[5] = separator;
    let mut used = 6;
    let mut part = 0;
    while part < time.len() {
        items[used] = time[part];
        used += 1;
        part += 1;
    }
    part = 0;
    while part < suffix.len() {
        items[used] = suffix[part];
        used += 1;
        part += 1;
    }
    items
}

const HM: &[Item] = &[Hour, Lit(":"), Minute];
const HMS_F: &[Item] = &[Hour, Lit(":"), Minute, Lit(":"), Second, Fraction];
const NONE: &[Item] = &[];
const ZULU: &[Item] = &[Lit("Z")];
const OFFSET: &[Item] = &[Offset];

/// The date-time formats SQLx's SQLite decoder tries after RFC 3339, in its
/// order: `%F %T%.f`, `%F %R`, `%F %RZ`, `%F %R%:z`, `%F %T%.fZ`,
/// `%F %T%.f%:z`, then the same with `T` instead of the space. Unused trailing
/// items are empty literals, which match anything.
const SQLITE_DATE_TIME: [[Item; 13]; 12] = [
    date_time(Space, HMS_F, NONE),
    date_time(Space, HM, NONE),
    date_time(Space, HM, ZULU),
    date_time(Space, HM, OFFSET),
    date_time(Space, HMS_F, ZULU),
    date_time(Space, HMS_F, OFFSET),
    date_time(Lit("T"), HM, NONE),
    date_time(Lit("T"), HM, ZULU),
    date_time(Lit("T"), HM, OFFSET),
    date_time(Lit("T"), HMS_F, NONE),
    date_time(Lit("T"), HMS_F, ZULU),
    date_time(Lit("T"), HMS_F, OFFSET),
];

/// The time formats SQLx's SQLite decoder tries, in its order: `%T.f` (a
/// literal `.f`), `%T%.f`, `%R`, `%RZ`, `%T%.fZ`, `%R%:z`, `%T%.f%:z`.
const SQLITE_TIME: [&[Item]; 7] = [
    &[Hour, Lit(":"), Minute, Lit(":"), Second, Lit(".f")],
    HMS_F,
    HM,
    &[Hour, Lit(":"), Minute, Lit("Z")],
    &[Hour, Lit(":"), Minute, Lit(":"), Second, Fraction, Lit("Z")],
    &[Hour, Lit(":"), Minute, Offset],
    &[Hour, Lit(":"), Minute, Lit(":"), Second, Fraction, Offset],
];

/// What a format matched, before the calendar checks.
#[derive(Default)]
pub(super) struct Parsed {
    year: Option<i32>,
    month: Option<i8>,
    day: Option<i8>,
    hour: Option<i8>,
    minute: Option<i8>,
    second: Option<i8>,
    nanosecond: Option<i32>,
    /// Seconds east of UTC, unchecked (`±99:59` at most).
    pub(super) offset: Option<i32>,
}

impl Parsed {
    pub(super) fn date(&self) -> Option<civil::Date> {
        let year = i16::try_from(self.year?).ok()?;
        civil::Date::new(year, self.month?, self.day?).ok()
    }

    /// Seconds and fraction default to zero; a leap second is second 59.
    pub(super) fn time(&self) -> Option<civil::Time> {
        let second = self.second.unwrap_or(0).min(59);
        let nanosecond = self.nanosecond.unwrap_or(0);
        civil::Time::new(self.hour?, self.minute?, second, nanosecond).ok()
    }

    pub(super) fn date_time(&self) -> Option<civil::DateTime> {
        Some(civil::DateTime::from_parts(self.date()?, self.time()?))
    }

    /// The offset when chrono's `FixedOffset` accepts it (under 24 hours).
    pub(super) fn valid_offset(&self) -> Option<i32> {
        self.offset.filter(|seconds| seconds.abs() < 86_400)
    }
}

/// Matches `text` against `items` as chrono's `parse` does: every item in
/// turn, and nothing may follow the last.
pub(super) fn parse(mut text: &str, items: &[Item]) -> Option<Parsed> {
    let mut parsed = Parsed::default();
    for item in items {
        match *item {
            Lit(literal) => text = text.strip_prefix(literal)?,
            Space => text = text.trim_start(),
            Fraction => {
                if let Some(rest) = text.strip_prefix('.') {
                    let (rest, nanosecond) = fraction(rest)?;
                    parsed.nanosecond = Some(nanosecond);
                    text = rest;
                }
            }
            Offset | OffsetOrZulu => {
                let lenient = matches!(item, OffsetOrZulu);
                let (rest, seconds) = offset(text.trim_start(), lenient, Colon::Optional, lenient)?;
                parsed.offset = Some(seconds);
                text = rest;
            }
            Year => {
                let (rest, year) = year(text.trim_start())?;
                parsed.year = Some(i32::try_from(year).ok()?);
                text = rest;
            }
            field => {
                let (rest, value) = number(text.trim_start(), 2)?;
                let (slot, max) = match field {
                    Month => (&mut parsed.month, 12),
                    Day => (&mut parsed.day, 31),
                    Hour => (&mut parsed.hour, 23),
                    Minute => (&mut parsed.minute, 59),
                    _ => (&mut parsed.second, 60),
                };
                let min = i64::from(matches!(field, Month | Day));
                if !(min..=max).contains(&value) {
                    return None;
                }
                *slot = Some(i8::try_from(value).ok()?);
                text = rest;
            }
        }
    }
    text.is_empty().then_some(parsed)
}

/// Up to `max` ASCII digits, at least one.
fn number(text: &str, max: usize) -> Option<(&str, i64)> {
    let digits = text
        .bytes()
        .take(max)
        .take_while(u8::is_ascii_digit)
        .count();
    if digits == 0 {
        return None;
    }
    let mut value = 0_i64;
    for byte in text[..digits].bytes() {
        value = value.checked_mul(10)?.checked_add(i64::from(byte - b'0'))?;
    }
    Some((&text[digits..], value))
}

/// `%Y`: a sign allows any number of digits, otherwise at most four.
fn year(text: &str) -> Option<(&str, i64)> {
    if let Some(rest) = text.strip_prefix('-') {
        let (rest, value) = number(rest, usize::MAX)?;
        Some((rest, -value))
    } else if let Some(rest) = text.strip_prefix('+') {
        number(rest, usize::MAX)
    } else {
        number(text, 4)
    }
}

/// The digits after a `.`: at least one; past the ninth they are skipped.
fn fraction(text: &str) -> Option<(&str, i32)> {
    let (rest, value) = number(text, 9)?;
    let digits = text.len() - rest.len();
    let scale = 10_i64.pow(9 - u32::try_from(digits).ok()?);
    let nanosecond = i32::try_from(value * scale).ok()?;
    Some((
        rest.trim_start_matches(|c: char| c.is_ascii_digit()),
        nanosecond,
    ))
}

/// How an offset's hours and minutes are separated.
#[derive(Clone, Copy)]
enum Colon {
    /// Exactly one `:` (RFC 3339).
    Required,
    /// Any run of `:` and whitespace, or nothing (`%:z`, `%#z`).
    Optional,
}

/// A UTC offset in seconds: `Z`/`z` when `zulu`, else a sign (`+`, `-` or
/// U+2212), two hour digits, the separator and two minute digits (which may
/// be missing at the end of the text when `missing_minutes`).
fn offset(mut text: &str, zulu: bool, colon: Colon, missing_minutes: bool) -> Option<(&str, i32)> {
    if zulu && let Some(rest) = text.strip_prefix(['Z', 'z']) {
        return Some((rest, 0));
    }
    let negative = if let Some(rest) = text.strip_prefix('+') {
        text = rest;
        false
    } else if let Some(rest) = text.strip_prefix(['-', '\u{2212}']) {
        text = rest;
        true
    } else {
        return None;
    };
    let hours = two_digits(text, b'9')?;
    text = &text[2..];
    text = match colon {
        Colon::Required => text.strip_prefix(':')?,
        Colon::Optional => text.trim_start_matches(|c: char| c == ':' || c.is_whitespace()),
    };
    let minutes = match text.len() {
        0 if missing_minutes => 0,
        0 | 1 => return None,
        _ => {
            let minutes = two_digits(text, b'5')?;
            text = &text[2..];
            minutes
        }
    };
    let seconds = hours * 3600 + minutes * 60;
    Some((text, if negative { -seconds } else { seconds }))
}

/// Two ASCII digits at the start of `text`, the first at most `first_max`.
fn two_digits(text: &str, first_max: u8) -> Option<i32> {
    match text.as_bytes() {
        [tens @ b'0'..=b'9', units @ b'0'..=b'9', ..] if *tens <= first_max => {
            Some(i32::from(tens - b'0') * 10 + i32::from(units - b'0'))
        }
        _ => None,
    }
}

/// chrono's strict RFC 3339 parser: `YYYY-MM-DD`, `T`, `t` or a space,
/// `HH:MM:SS`, an optional fraction and `Z`, `z` or `±HH:MM`, with a valid
/// offset. Returns the local date-time and the offset in seconds.
fn rfc3339(text: &str) -> Option<(civil::DateTime, i32)> {
    let bytes = text.as_bytes();
    let digit = |index: usize| -> Option<i32> {
        let byte = *bytes.get(index)?;
        byte.is_ascii_digit().then(|| i32::from(byte - b'0'))
    };
    let pair = |index: usize| -> Option<i32> { Some(digit(index)? * 10 + digit(index + 1)?) };
    let separators_match = bytes.len() >= 19
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && matches!(bytes[10], b'T' | b't' | b' ')
        && bytes[13] == b':'
        && bytes[16] == b':';
    if !separators_match {
        return None;
    }
    let mut parsed = Parsed {
        year: Some(pair(0)? * 100 + pair(2)?),
        month: Some(i8::try_from(pair(5)?).ok()?),
        day: Some(i8::try_from(pair(8)?).ok()?),
        hour: Some(i8::try_from(pair(11)?).ok()?),
        minute: Some(i8::try_from(pair(14)?).ok()?),
        second: Some(i8::try_from(pair(17)?).ok()?),
        ..Parsed::default()
    };
    // `NaiveTime::from_hms_nano_opt` range checks; 60 is a leap second.
    if parsed.hour? > 23 || parsed.minute? > 59 || parsed.second? > 60 {
        return None;
    }
    let mut rest = &text[19..];
    if let Some(after_dot) = rest.strip_prefix('.') {
        let (after, nanosecond) = fraction(after_dot)?;
        parsed.nanosecond = Some(nanosecond);
        rest = after;
    }
    let (rest, seconds) = offset(rest, true, Colon::Required, false)?;
    if !rest.is_empty() || seconds.abs() >= 86_400 {
        return None;
    }
    Some((parsed.date_time()?, seconds))
}

/// A date-time read from text, with the UTC offset it was written at (zero
/// when it had none).
pub(super) struct Moment {
    pub(super) local: civil::DateTime,
    pub(super) offset: i32,
}

impl Moment {
    /// The instant: the local date-time minus the offset.
    pub(super) fn instant(&self) -> Option<jiff::Timestamp> {
        let offset = jiff::tz::Offset::from_seconds(self.offset).ok()?;
        offset.to_timestamp(self.local).ok()
    }
}

/// A SQLite text date-time, as SQLx's `chrono` decoder reads it: RFC 3339,
/// then each format of [`SQLITE_DATE_TIME`], first with a valid offset and
/// then ignoring the offset (read as UTC).
pub(super) fn sqlite_date_time(text: &str) -> Option<Moment> {
    if let Some((local, offset)) = rfc3339(text) {
        return Some(Moment { local, offset });
    }
    SQLITE_DATE_TIME.iter().find_map(|items| {
        let parsed = parse(text, items)?;
        Some(Moment {
            local: parsed.date_time()?,
            offset: parsed.valid_offset().unwrap_or(0),
        })
    })
}

/// A SQLite text time of day: the first of [`SQLITE_TIME`] that matches; an
/// offset is ignored.
pub(super) fn sqlite_time(text: &str) -> Option<civil::Time> {
    SQLITE_TIME
        .iter()
        .find_map(|items| parse(text, items)?.time())
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
