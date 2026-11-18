//! The types' text against chrono's: `Display`, `Debug` and serde write
//! chrono's text for every value, and `FromStr` and serde read every text
//! chrono read, with chrono's value, falling back to jiff's parser.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use proptest::prelude::*;

use super::test_support::{
    chrono_date, chrono_date_time, chrono_instant, chrono_time, date, date_time, date_time_text,
    instant, jiff_date, jiff_date_time, jiff_instant, jiff_time, time, time_text,
};
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
    assert_eq!(
        chrono_instant(Timestamp::default()),
        chrono::DateTime::<Utc>::default()
    );
    assert_eq!(
        chrono_date_time(DateTime::default()),
        NaiveDateTime::default()
    );
    assert_eq!(chrono_date(*Date::default()), NaiveDate::default());
    assert_eq!(chrono_time(*Time::default()), NaiveTime::default());
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
    let later = Timestamp::from(at.checked_add(jiff::SignedDuration::from_nanos(1)).unwrap());
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

/// Near-valid dates.
pub(super) fn date_text() -> impl Strategy<Value = String> {
    (
        prop_oneof![
            (0_i32..=9999).prop_map(|year| format!("{year:04}")),
            (-12_000_i32..=12_000).prop_map(|year| format!("{year:+05}")),
            (0_i32..100_000).prop_map(|year| year.to_string()),
        ],
        0_u32..=13,
        0_u32..=32,
        prop_oneof![Just(""), Just(" "), Just(" -"), Just("x"), Just("T")],
        prop_oneof![Just(""), Just(" "), Just("\t")],
    )
        .prop_map(|(year, month, day, tail, lead)| format!("{lead}{year}-{month}-{day:02}{tail}"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn written_text_is_chronos(at in instant(), civil in date_time(), day in date(), clock in time()) {
        let theirs = chrono_instant(at);
        prop_assert_eq!(at.to_string(), theirs.to_string());
        prop_assert_eq!(format!("{at:?}"), format!("{theirs:?}"));
        prop_assert_eq!(serde_json::to_string(&at).unwrap(), serde_json::to_string(&theirs).unwrap());
        let naive = chrono_date_time(civil);
        prop_assert_eq!(civil.to_string(), naive.to_string());
        prop_assert_eq!(format!("{civil:?}"), format!("{naive:?}"));
        prop_assert_eq!(serde_json::to_string(&civil).unwrap(), serde_json::to_string(&naive).unwrap());
        let naive_date = chrono_date(*day);
        prop_assert_eq!(day.to_string(), naive_date.to_string());
        prop_assert_eq!(format!("{day:?}"), format!("{naive_date:?}"));
        prop_assert_eq!(serde_json::to_string(&day).unwrap(), serde_json::to_string(&naive_date).unwrap());
        let naive_time = chrono_time(*clock);
        prop_assert_eq!(clock.to_string(), naive_time.to_string());
        prop_assert_eq!(format!("{clock:?}"), format!("{naive_time:?}"));
        prop_assert_eq!(serde_json::to_string(&clock).unwrap(), serde_json::to_string(&naive_time).unwrap());
        // And it reads back as itself.
        prop_assert_eq!(serde_json::from_str::<Timestamp>(&serde_json::to_string(&at).unwrap()).unwrap(), at);
        prop_assert_eq!(at.to_string().parse::<Timestamp>().unwrap(), at);
        prop_assert_eq!(format!("{civil:?}").parse::<DateTime>().unwrap(), civil);
        prop_assert_eq!(day.to_string().parse::<Date>().unwrap(), day);
        prop_assert_eq!(clock.to_string().parse::<Time>().unwrap(), clock);
    }

    /// chrono's value whenever chrono reads the text, else jiff's.
    #[test]
    fn instants_parse_as_chrono_then_jiff(text in prop_oneof![date_time_text(), "[0-9T:+\\-. ZzUTCu]{0,40}"]) {
        let chrono_value = text.parse::<chrono::DateTime<Utc>>().ok().map(jiff_instant);
        let expected = match chrono_value {
            Some(Some(value)) => Some(value),
            _ => text.parse::<jiff::Timestamp>().ok(),
        };
        prop_assert_eq!(text.parse::<Timestamp>().ok().map(|at| *at), expected, "{:?}", text);
    }

    #[test]
    fn date_times_parse_as_chrono_then_jiff(text in prop_oneof![date_time_text(), "[0-9T:+\\-. ]{0,30}"]) {
        let chrono_value = text.parse::<NaiveDateTime>().ok().map(jiff_date_time);
        let expected = match chrono_value {
            Some(Some(value)) => Some(value),
            _ => text.parse::<jiff::civil::DateTime>().ok(),
        };
        prop_assert_eq!(text.parse::<DateTime>().ok().map(|at| *at), expected, "{:?}", text);
    }

    #[test]
    fn dates_parse_as_chrono_then_jiff(text in prop_oneof![date_text(), "[0-9+\\- T]{0,16}"]) {
        let chrono_value = text.parse::<NaiveDate>().ok().map(jiff_date);
        let expected = match chrono_value {
            Some(Some(value)) => Some(value),
            _ => text.parse::<jiff::civil::Date>().ok(),
        };
        prop_assert_eq!(text.parse::<Date>().ok().map(|at| *at), expected, "{:?}", text);
    }

    #[test]
    fn times_parse_as_chrono_then_jiff(text in prop_oneof![time_text(), "[0-9:. ]{0,16}"]) {
        let chrono_value = text.parse::<NaiveTime>().ok().map(jiff_time);
        let expected = match chrono_value {
            Some(Some(value)) => Some(value),
            _ => text.parse::<jiff::civil::Time>().ok(),
        };
        prop_assert_eq!(text.parse::<Time>().ok().map(|at| *at), expected, "{:?}", text);
    }
}
