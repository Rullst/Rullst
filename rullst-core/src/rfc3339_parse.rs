//! chrono 0.4's `DateTime::parse_from_rfc3339`, which validated security
//! event times and read webhook times until 13.0.0-alpha.6.
//!
//! It takes `YYYY-MM-DD`, `T`, `t` or a space, `hh:mm:ss`, an optional `.`
//! and one or more digits (past nine ignored), then `Z`, `z` or an offset
//! `+hh:mm` (also `-` or U+2212) below 24 hours. Second `60` is a leap second.

use super::{SECONDS_PER_DAY, days_from_civil};

/// The Unix seconds (rounded down, a leap second counted as the second
/// before it) of an RFC 3339 date and time, as chrono's
/// `DateTime::parse_from_rfc3339(text)?.timestamp()`.
pub fn parse_unix_seconds(text: &str) -> Option<i64> {
    parse_parts(text, false).map(|(seconds, _)| seconds)
}

/// The Unix seconds and nanoseconds of `text`; a leap second is the second
/// before it with 999,999,999 nanoseconds. `signed_years` also reads the
/// years chrono wrote with a sign (`-0001`, `+10000`).
pub(super) fn parse_parts(text: &str, signed_years: bool) -> Option<(i64, u32)> {
    let (year, rest) = match text.as_bytes().first() {
        Some(sign @ (b'+' | b'-')) if signed_years => {
            let digits = text.get(1..)?;
            let length = digits.bytes().take_while(u8::is_ascii_digit).count();
            if !(4..=9).contains(&length) {
                return None;
            }
            let year: i64 = digits.get(..length)?.parse().ok()?;
            let year = if *sign == b'-' { -year } else { year };
            (year, digits.get(length..)?)
        }
        _ => (digits(text, 0..4)?, text.get(4..)?),
    };
    // `-MM-DD`, a separator, `hh:mm:ss`: 15 bytes.
    let bytes = rest.as_bytes();
    if bytes.len() < 15 {
        return None;
    }
    let month = digits(rest, 1..3)?;
    let day = digits(rest, 4..6)?;
    let hour = digits(rest, 7..9)?;
    let minute = digits(rest, 10..12)?;
    let second = digits(rest, 13..15)?;
    let separators = [(0, b'-'), (3, b'-'), (9, b':'), (12, b':')];
    if separators
        .iter()
        .any(|(index, separator)| bytes.get(*index) != Some(separator))
        || !matches!(bytes.get(6), Some(b'T' | b't' | b' '))
        || !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    let (rest, nanos) = match rest.get(15..)?.strip_prefix('.') {
        Some(fraction) => {
            let count = fraction.bytes().take_while(u8::is_ascii_digit).count();
            if count == 0 {
                return None;
            }
            let kept = fraction.get(..count.min(9))?;
            let scale = 10_u32.pow(u32::try_from(9 - kept.len()).ok()?);
            (fraction.get(count..)?, kept.parse::<u32>().ok()? * scale)
        }
        None => (rest.get(15..)?, 0),
    };
    let offset = offset_seconds(rest)?;
    let (second, nanos) = if second == 60 {
        // A leap second lasts until the next minute.
        (59, 999_999_999)
    } else {
        (second, nanos)
    };
    let local = days_from_civil(year, month, day) * SECONDS_PER_DAY
        + hour * 3_600
        + minute * 60
        + second;
    Some((local - offset, nanos))
}

/// The number written with the ASCII digits at `range` of `text`.
fn digits(text: &str, range: std::ops::Range<usize>) -> Option<i64> {
    text.as_bytes()
        .get(range)?
        .iter()
        .try_fold(0_i64, |value, byte| {
            byte.is_ascii_digit()
                .then(|| value * 10 + i64::from(byte - b'0'))
        })
}

/// `Z`, `z` or `±hh:mm` (`-` or U+2212 for minus), and nothing after it.
fn offset_seconds(text: &str) -> Option<i64> {
    if text == "Z" || text == "z" {
        return Some(0);
    }
    let (negative, rest) = if let Some(rest) = text.strip_prefix('+') {
        (false, rest)
    } else if let Some(rest) = text.strip_prefix('-').or_else(|| text.strip_prefix('\u{2212}')) {
        (true, rest)
    } else {
        return None;
    };
    let bytes = rest.as_bytes();
    if bytes.len() != 5
        || bytes.get(2) != Some(&b':')
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 2 || byte.is_ascii_digit())
    {
        return None;
    }
    let hours = i64::from(bytes[0] - b'0') * 10 + i64::from(bytes[1] - b'0');
    let minutes = i64::from(bytes[3] - b'0') * 10 + i64::from(bytes[4] - b'0');
    if hours > 23 || minutes > 59 {
        return None;
    }
    let seconds = hours * 3_600 + minutes * 60;
    Some(if negative { -seconds } else { seconds })
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 31,
    }
}
