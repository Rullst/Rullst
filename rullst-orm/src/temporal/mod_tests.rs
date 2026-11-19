//! Golden text of the types; `golden_tests.rs` checks `Display`, `Debug`,
//! serde and `FromStr` against the corpus recorded from chrono.

use proptest::prelude::*;

use super::test_support::{date, date_time, instant, time};
use super::{Date, DateTime, Time, Timestamp};
use crate::RullstValue;

fn instant_at(text: &str) -> Timestamp {
    Timestamp::from(text.parse::<jiff::Timestamp>().expect("valid instant"))
}

#[test]
fn golden_text() {
    let at = instant_at("2026-10-15T09:30:00.5Z");
    assert_eq!(at.to_string(), "2026-10-15 09:30:00.500 UTC");
    assert_eq!(format!("{at:?}"), "2026-10-15T09:30:00.500Z");
    assert_eq!(
        serde_json::to_string(&at).unwrap(),
        "\"2026-10-15T09:30:00.500Z\""
    );
    let civil = DateTime::from(jiff::civil::date(2026, 10, 15).at(9, 30, 0, 1_000));
    assert_eq!(civil.to_string(), "2026-10-15 09:30:00.000001");
    assert_eq!(format!("{civil:?}"), "2026-10-15T09:30:00.000001");
    assert_eq!(
        serde_json::to_string(&civil).unwrap(),
        "\"2026-10-15T09:30:00.000001\""
    );
    let ancient = Date::from(jiff::civil::date(-44, 3, 15));
    assert_eq!(ancient.to_string(), "-0044-03-15");
    assert_eq!(format!("{ancient:?}"), "-0044-03-15");
    assert_eq!(serde_json::to_string(&ancient).unwrap(), "\"-0044-03-15\"");
    let clock = Time::from(jiff::civil::time(23, 59, 59, 7));
    assert_eq!(clock.to_string(), "23:59:59.000000007");
    assert_eq!(format!("{clock:?}"), "23:59:59.000000007");
    assert_eq!(
        serde_json::to_string(&clock).unwrap(),
        "\"23:59:59.000000007\""
    );
    // The jiff value behind `Deref` keeps jiff's forms.
    assert_eq!((*at).to_string(), "2026-10-15T09:30:00.5Z");
}

/// Texts chrono read that jiff's parser alone rejects, and jiff's own forms.
#[test]
fn golden_parsing() {
    let read = |text: &str| text.parse::<Timestamp>().map(|at| format!("{at:?}"));
    for (text, expected) in [
        ("2026-10-15T09:30:00 UTC", "2026-10-15T09:30:00Z"),
        ("2026-10-15 09:30:00utc", "2026-10-15T09:30:00Z"),
        ("2026-10-15T09:30:00 +00:00", "2026-10-15T09:30:00Z"),
        ("2026-10-15T9:30:00Z", "2026-10-15T09:30:00Z"),
        ("+2026-10-15T09:30:00Z", "2026-10-15T09:30:00Z"),
        (
            "2026-10-15T09:30:00.1234567891Z",
            "2026-10-15T09:30:00.123456789Z",
        ),
        ("2026-10-15T12:30:00+0300", "2026-10-15T09:30:00Z"),
        ("2026-10-15T12:30:00+03 00", "2026-10-15T09:30:00Z"),
        // jiff's own forms.
        ("2026-10-15T09:30:00Z[UTC]", "2026-10-15T09:30:00Z"),
        ("2026-10-15T09:30:00+24:00", "2026-10-14T09:30:00Z"),
    ] {
        assert_eq!(read(text).ok().as_deref(), Some(expected), "{text}");
    }
    assert!(read("2026-10-15T09:30:00").is_err());
    assert!(read("2026-10-15T09:30:00 UTC junk").is_err());

    let civil = |text: &str| text.parse::<DateTime>().map(|at| at.to_string());
    for (text, expected) in [
        (" 2026-10-15T09:30:00", "2026-10-15 09:30:00"),
        ("2026-10-15T9:30:00", "2026-10-15 09:30:00"),
        ("+2026-10-15T09:30:00.5", "2026-10-15 09:30:00.500"),
        ("2026-10-15 09:30:00", "2026-10-15 09:30:00"),
        ("2026-10-15T09:30", "2026-10-15 09:30:00"),
    ] {
        assert_eq!(civil(text).ok().as_deref(), Some(expected), "{text}");
    }
    assert_eq!(
        "9:30:00"
            .parse::<Time>()
            .map(|at| at.to_string())
            .ok()
            .as_deref(),
        Some("09:30:00")
    );
    assert_eq!(
        "09:30"
            .parse::<Time>()
            .map(|at| at.to_string())
            .ok()
            .as_deref(),
        Some("09:30:00")
    );
    for (text, expected) in [("+2026-10-15", "2026-10-15"), ("2026-1-5", "2026-01-05")] {
        assert_eq!(
            text.parse::<Date>()
                .map(|at| at.to_string())
                .ok()
                .as_deref(),
            Some(expected),
            "{text}"
        );
    }
    assert!("2026-02-30".parse::<Date>().is_err());
    assert!("+10000-01-01".parse::<Date>().is_err());
}

#[test]
fn serde_reads_what_from_str_reads() {
    let at: Timestamp = serde_json::from_str("\"2026-10-15T09:30:00 UTC\"").unwrap();
    assert_eq!(at, instant_at("2026-10-15T09:30:00Z"));
    let civil: DateTime = serde_json::from_str("\"2026-10-15T09:30:00.500\"").unwrap();
    assert_eq!(civil.to_string(), "2026-10-15 09:30:00.500");
    let day: Date = serde_json::from_str("\"2026-1-5\"").unwrap();
    assert_eq!(day.to_string(), "2026-01-05");
    let clock: Time = serde_json::from_str("\"9:30\"").unwrap();
    assert_eq!(clock.to_string(), "09:30:00");
    let error = serde_json::from_str::<Timestamp>("17").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("a formatted date and time string"),
        "{error}"
    );
    assert!(serde_json::from_str::<Date>("\"not a date\"").is_err());
}

#[test]
fn conversions_and_deref_reach_the_jiff_value() {
    let jiff_instant = jiff::Timestamp::from_millisecond(1_792_056_600_500).unwrap();
    assert_eq!(
        Timestamp::from(jiff_instant),
        instant_at("2026-10-15T09:30:00.5Z")
    );
    assert_eq!(
        jiff::Timestamp::from(Timestamp::from(jiff_instant)),
        jiff_instant
    );
    assert_eq!(
        Timestamp::from(jiff_instant).as_millisecond(),
        1_792_056_600_500
    );

    let civil = jiff::civil::date(2026, 10, 15).at(9, 30, 0, 0);
    assert_eq!(DateTime::from(civil).hour(), 9);
    assert_eq!(jiff::civil::DateTime::from(DateTime::from(civil)), civil);
    let day = jiff::civil::date(2026, 10, 15);
    assert_eq!(Date::from(day).day(), 15);
    assert_eq!(jiff::civil::Date::from(Date::from(day)), day);
    let clock = jiff::civil::time(9, 30, 0, 7);
    assert_eq!(Time::from(clock).subsec_nanosecond(), 7);
    assert_eq!(jiff::civil::Time::from(Time::from(clock)), clock);
}

#[test]
fn defaults_are_chronos() {
    assert_eq!(
        format!("{:?}", Timestamp::default()),
        "1970-01-01T00:00:00Z"
    );
    assert_eq!(DateTime::default().to_string(), "1970-01-01 00:00:00");
    assert_eq!(Date::default().to_string(), "1970-01-01");
    assert_eq!(Time::default().to_string(), "00:00:00");
}

#[test]
fn now_reads_the_clock() {
    let before = jiff::Timestamp::now();
    let now = Timestamp::now();
    assert!(*now >= before);
    assert!(*now <= jiff::Timestamp::now());
}

#[test]
fn ordering_follows_time() {
    let at = instant_at("2026-10-15T09:30:00.5Z");
    let later = at.checked_add(jiff::SignedDuration::from_nanos(1)).unwrap();
    assert!(at < later);
    let mut values = [later, at];
    values.sort();
    assert_eq!(values, [at, later]);
}

#[test]
fn each_type_becomes_its_value_variant() {
    let at = instant_at("2026-10-15T09:30:00.5Z");
    assert!(matches!(RullstValue::from(at), RullstValue::Timestamp(value) if value == at));
    let civil = DateTime::from(jiff::civil::date(2026, 10, 15).at(9, 30, 0, 0));
    assert!(
        matches!(RullstValue::from(civil), RullstValue::CivilDateTime(value) if value == civil)
    );
    let day = Date::from(jiff::civil::date(2026, 10, 15));
    assert!(matches!(RullstValue::from(day), RullstValue::CivilDate(value) if value == day));
    let clock = Time::from(jiff::civil::time(9, 30, 0, 0));
    assert!(matches!(RullstValue::from(clock), RullstValue::CivilTime(value) if value == clock));
    assert!(matches!(
        RullstValue::from(None::<Timestamp>),
        RullstValue::Null
    ));
    assert!(matches!(
        RullstValue::from(Some(day)),
        RullstValue::CivilDate(value) if value == day
    ));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    /// Every written text reads back as itself (`golden_tests.rs` checks the
    /// text against chrono's).
    #[test]
    fn written_text_reads_back(at in instant(), civil in date_time(), day in date(), clock in time()) {
        prop_assert_eq!(serde_json::from_str::<Timestamp>(&serde_json::to_string(&at).unwrap()).unwrap(), at);
        prop_assert_eq!(at.to_string().parse::<Timestamp>().unwrap(), at);
        prop_assert_eq!(format!("{civil:?}").parse::<DateTime>().unwrap(), civil);
        prop_assert_eq!(serde_json::from_str::<DateTime>(&serde_json::to_string(&civil).unwrap()).unwrap(), civil);
        prop_assert_eq!(day.to_string().parse::<Date>().unwrap(), day);
        prop_assert_eq!(clock.to_string().parse::<Time>().unwrap(), clock);
    }
}
