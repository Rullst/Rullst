//! Shared by the date/time tests: conversions between the chrono values SQLx's
//! `chrono` codecs take and the jiff values ours take, and value strategies
//! over jiff's whole range.

use chrono::{
    DateTime as ChronoDateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc,
};
use jiff::civil;
use proptest::prelude::*;

use super::{Date, DateTime, Time, Timestamp};

/// A chrono time as jiff reads it: a leap second is second 59.
pub(crate) fn jiff_time(time: NaiveTime) -> Option<civil::Time> {
    let nanos = time.nanosecond() % 1_000_000_000;
    civil::Time::new(
        i8::try_from(time.hour()).ok()?,
        i8::try_from(time.minute()).ok()?,
        i8::try_from(time.second()).ok()?,
        i32::try_from(nanos).ok()?,
    )
    .ok()
}

pub(crate) fn jiff_date(date: NaiveDate) -> Option<civil::Date> {
    civil::Date::new(
        i16::try_from(date.year()).ok()?,
        i8::try_from(date.month()).ok()?,
        i8::try_from(date.day()).ok()?,
    )
    .ok()
}

pub(crate) fn jiff_date_time(value: NaiveDateTime) -> Option<civil::DateTime> {
    Some(jiff_date(value.date())?.to_datetime(jiff_time(value.time())?))
}

pub(crate) fn jiff_instant(value: ChronoDateTime<Utc>) -> Option<jiff::Timestamp> {
    jiff::tz::Offset::UTC
        .to_timestamp(jiff_date_time(value.naive_utc())?)
        .ok()
}

pub(crate) fn chrono_date(value: civil::Date) -> NaiveDate {
    NaiveDate::from_ymd_opt(
        i32::from(value.year()),
        value.month().unsigned_abs().into(),
        value.day().unsigned_abs().into(),
    )
    .expect("every jiff date is a chrono date")
}

pub(crate) fn chrono_time(value: civil::Time) -> NaiveTime {
    NaiveTime::from_hms_nano_opt(
        value.hour().unsigned_abs().into(),
        value.minute().unsigned_abs().into(),
        value.second().unsigned_abs().into(),
        value.subsec_nanosecond().unsigned_abs(),
    )
    .expect("every jiff time is a chrono time")
}

pub(crate) fn chrono_date_time(value: impl Into<civil::DateTime>) -> NaiveDateTime {
    let value = value.into();
    chrono_date(value.date()).and_time(chrono_time(value.time()))
}

pub(crate) fn chrono_instant(value: impl Into<jiff::Timestamp>) -> ChronoDateTime<Utc> {
    chrono_date_time(jiff::tz::Offset::UTC.to_datetime(value.into())).and_utc()
}

/// Nanoseconds, often whole seconds, milliseconds or microseconds, so every
/// fraction width is written.
fn nanos() -> impl Strategy<Value = i32> {
    prop_oneof![
        Just(0),
        (0..1_000).prop_map(|millis| millis * 1_000_000),
        (0..1_000_000).prop_map(|micros| micros * 1_000),
        0..1_000_000_000,
        Just(999_999_999),
        1..1_000,
    ]
}

/// Any instant jiff holds, with the edges and 2000-01-01 (PostgreSQL's
/// origin) weighted in.
pub(crate) fn instant() -> impl Strategy<Value = Timestamp> {
    let min = jiff::Timestamp::MIN.as_second();
    let max = jiff::Timestamp::MAX.as_second();
    prop_oneof![
        (min..=max, nanos()),
        (946_684_790_i64..946_684_810, nanos()),
        (-10_i64..10, nanos()),
        (
            prop_oneof![Just(min), Just(max), Just(min + 1), Just(max - 1)],
            nanos()
        ),
    ]
    .prop_filter_map("in range", |(second, nanos)| {
        let nanos = if second < 0 && nanos > 0 {
            -nanos
        } else {
            nanos
        };
        jiff::Timestamp::new(second, nanos)
            .ok()
            .map(Timestamp::from)
    })
}

/// Any date-time jiff holds.
pub(crate) fn date_time() -> impl Strategy<Value = DateTime> {
    (
        prop_oneof![
            -9999_i16..=9999,
            1999_i16..=2001,
            Just(-9999),
            Just(9999),
            0_i16..=1
        ],
        1_i8..=12,
        1_i8..=31,
        0_i8..=23,
        0_i8..=59,
        0_i8..=59,
        nanos(),
    )
        .prop_filter_map(
            "a real day",
            |(year, month, day, hour, minute, second, nanos)| {
                civil::DateTime::new(year, month, day, hour, minute, second, nanos)
                    .ok()
                    .map(DateTime::from)
            },
        )
}

pub(crate) fn date() -> impl Strategy<Value = Date> {
    date_time().prop_map(|value| Date::from(value.date()))
}

pub(crate) fn time() -> impl Strategy<Value = Time> {
    date_time().prop_map(|value| Time::from(value.time()))
}

/// Near-valid date-times: every part varies in width, range and spelling.
pub(crate) fn date_time_text() -> impl Strategy<Value = String> {
    let year = prop_oneof![
        (0_i32..=9999).prop_map(|year| format!("{year:04}")),
        (-12_000_i32..=12_000).prop_map(|year| format!("{year:+05}")),
        (0_i32..100_000).prop_map(|year| year.to_string()),
        Just("-0".to_string()),
    ];
    let two = |max: u32| {
        prop_oneof![
            (0..=max).prop_map(|value| format!("{value:02}")),
            (0..=max).prop_map(|value| value.to_string()),
            Just(" 7".to_string()),
            Just("123".to_string()),
        ]
    };
    let fraction = prop_oneof![
        Just(String::new()),
        "\\.[0-9]{0,12}",
        Just(".f".to_string()),
    ];
    let separator = prop_oneof![
        Just("T"),
        Just(" "),
        Just("t"),
        Just("  "),
        Just("\t"),
        Just(""),
    ];
    let offset = prop_oneof![
        Just(String::new()),
        Just("Z".to_string()),
        Just("z".to_string()),
        "[ ]?[+\\-\u{2212}][0-9]{2}(:?[0-9]{2})?",
        "[+-][0-9]{1,2}(:| |::)?[0-9]{0,3}",
    ];
    (
        year,
        two(14),
        two(33),
        separator,
        two(25),
        two(62),
        prop::option::of(two(62)),
        fraction,
        offset,
        prop_oneof![Just(""), Just(" "), Just("x")],
    )
        .prop_map(
            |(year, month, day, separator, hour, minute, second, fraction, offset, tail)| {
                let seconds = match second {
                    Some(second) => format!(":{second}{fraction}"),
                    None => String::new(),
                };
                format!("{year}-{month}-{day}{separator}{hour}:{minute}{seconds}{offset}{tail}")
            },
        )
}

pub(crate) fn time_text() -> impl Strategy<Value = String> {
    (
        0_u32..=25,
        0_u32..=61,
        prop::option::of(0_u32..=61),
        prop_oneof![
            Just(String::new()),
            "\\.[0-9]{0,12}",
            Just(".f".to_string())
        ],
        prop_oneof![
            Just(String::new()),
            Just("Z".to_string()),
            "[ ]?[+-][0-9]{2}(:?[0-9]{2})?"
        ],
    )
        .prop_map(|(hour, minute, second, fraction, offset)| {
            let seconds = second
                .map(|second| format!(":{second:02}{fraction}"))
                .unwrap_or_default();
            format!("{hour:02}:{minute:02}{seconds}{offset}")
        })
}
