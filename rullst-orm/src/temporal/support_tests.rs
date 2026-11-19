//! Shared by the date/time tests: value strategies over jiff's whole range.

use jiff::civil;
use proptest::prelude::*;

use super::{Date, DateTime, Time, Timestamp};

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
