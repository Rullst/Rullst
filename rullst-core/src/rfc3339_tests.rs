#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The expected strings are what chrono 0.4.45 printed and read. Until
//! chrono left Rullst, property tests (commit fa34caf89,
//! `rfc3339_differential_tests.rs`) also compared these helpers with it.

use super::*;

fn at(seconds: i64, nanos: i32) -> Timestamp {
    Timestamp::new(seconds, nanos).unwrap()
}

#[test]
fn utc_text_keeps_chrono_fraction_digits() {
    let base = 1_761_147_930; // 2025-10-22T15:45:30Z
    assert_eq!(
        format_utc(at(base, 0), Fraction::Auto),
        "2025-10-22T15:45:30Z"
    );
    assert_eq!(
        format_utc(at(base, 250_000_000), Fraction::Auto),
        "2025-10-22T15:45:30.250Z"
    );
    assert_eq!(
        format_utc(at(base, 250_100_000), Fraction::Auto),
        "2025-10-22T15:45:30.250100Z"
    );
    assert_eq!(
        format_utc(at(base, 1), Fraction::Auto),
        "2025-10-22T15:45:30.000000001Z"
    );
    assert_eq!(
        format_utc(at(base, 120), Fraction::Auto),
        "2025-10-22T15:45:30.000000120Z"
    );
    assert_eq!(
        format_utc(at(base, 0), Fraction::Millis),
        "2025-10-22T15:45:30.000Z"
    );
    assert_eq!(
        format_utc(at(base, 999_999_999), Fraction::Millis),
        "2025-10-22T15:45:30.999Z"
    );
    assert_eq!(
        format_utc(at(base, 7_000), Fraction::Nanos),
        "2025-10-22T15:45:30.000007000Z"
    );
}

#[test]
fn negative_instants_round_down_like_chrono() {
    assert_eq!(unix_parts(at(-5, -123_456_789)), (-6, 876_543_211));
    assert_eq!(unix_parts(at(-5, 0)), (-5, 0));
    assert_eq!(unix_parts(at(5, 1)), (5, 1));
    assert_eq!(
        format_utc(at(0, -1), Fraction::Auto),
        "1969-12-31T23:59:59.999999999Z"
    );
    assert_eq!(
        format_utc(at(-1, 0), Fraction::Millis),
        "1969-12-31T23:59:59.000Z"
    );
}

#[test]
fn years_outside_four_digits_take_a_sign() {
    let year_zero = -62_167_219_200;
    assert_eq!(
        format_utc(at(year_zero, 0), Fraction::Auto),
        "0000-01-01T00:00:00Z"
    );
    assert_eq!(
        format_utc(at(year_zero - 1, 0), Fraction::Auto),
        "-0001-12-31T23:59:59Z"
    );
    assert_eq!(utc_date(at(year_zero - 1, 0)), "-0001-12-31");
    assert_eq!(utc_date(Timestamp::MAX), "9999-12-30");
    assert_eq!(utc_date(Timestamp::MIN), "-9999-01-02");
    assert_eq!(
        format_unix_millis_utc(253_402_300_800_000, Fraction::Millis).as_deref(),
        Some("+10000-01-01T00:00:00.000Z")
    );
    assert_eq!(
        format_unix_millis_utc(MAX_MILLIS, Fraction::Millis).as_deref(),
        Some("+262142-12-31T23:59:59.999Z")
    );
    assert_eq!(
        format_unix_millis_utc(MIN_MILLIS, Fraction::Auto).as_deref(),
        Some("-262143-01-01T00:00:00Z")
    );
    assert_eq!(
        format_unix_millis_utc(MAX_MILLIS + 1, Fraction::Millis),
        None
    );
    assert_eq!(
        format_unix_millis_utc(MIN_MILLIS - 1, Fraction::Millis),
        None
    );
    assert_eq!(
        format_unix_millis_utc(-1, Fraction::Millis).as_deref(),
        Some("1969-12-31T23:59:59.999Z")
    );
}

#[test]
fn offsets_round_to_minutes_and_utc_keeps_its_offset() {
    let instant = at(1_761_147_930, 5_000_000);
    assert_eq!(
        format_with_offset(instant, 0),
        "2025-10-22T15:45:30.005+00:00"
    );
    assert_eq!(
        format_with_offset(instant, -10_800),
        "2025-10-22T12:45:30.005-03:00"
    );
    assert_eq!(
        format_with_offset(instant, 19_800),
        "2025-10-22T21:15:30.005+05:30"
    );
    assert_eq!(
        format_with_offset(instant, 49_500),
        "2025-10-23T05:30:30.005+13:45"
    );
    // Local mean time offsets have seconds; the wall time keeps them.
    assert_eq!(
        format_with_offset(instant, -11_188),
        "2025-10-22T12:39:02.005-03:06"
    );
    assert_eq!(
        format_with_offset(instant, 29),
        "2025-10-22T15:45:59.005+00:00"
    );
    assert_eq!(
        format_with_offset(instant, -30),
        "2025-10-22T15:45:00.005-00:01"
    );
}

#[test]
fn strict_parser_accepts_what_chrono_accepted() {
    let base = 1_761_147_930;
    for (text, expected) in [
        ("2025-10-22T15:45:30Z", base),
        ("2025-10-22t15:45:30z", base),
        ("2025-10-22 15:45:30Z", base),
        ("2025-10-22T15:45:30.999999999999Z", base),
        ("2025-10-22T12:45:30-03:00", base),
        ("2025-10-22T12:45:30\u{2212}03:00", base),
        ("2025-10-23T15:44:30+23:59", base),
        ("2025-10-22T15:45:60Z", base + 29),
        ("2024-02-29T00:00:00+00:00", 1_709_164_800),
        ("2000-02-29T00:00:00Z", 951_782_400),
        ("0000-01-01T00:00:00Z", -62_167_219_200),
        ("1969-12-31T23:59:59.5Z", -1),
    ] {
        assert_eq!(parse_unix_seconds(text), Some(expected), "{text}");
    }
    for text in [
        "",
        "2025-10-22T15:45:30",
        "2025-10-22T15:45Z",
        "2025-10-22T15:45:30.Z",
        "2025-10-22T15:45:30+0300",
        "2025-10-22T15:45:30+03",
        "2025-10-22T15:45:30+24:00",
        "2025-10-22T15:45:30+03:60",
        "2025-10-22T15:45:30Z ",
        "2025-10-22T24:00:00Z",
        "2025-10-22T15:60:00Z",
        "2025-10-22T15:45:61Z",
        "2025-02-29T00:00:00Z",
        "1900-02-29T00:00:00Z",
        "2100-02-29T00:00:00Z",
        "2025-06-31T00:00:00Z",
        "2025-13-01T00:00:00Z",
        "2025-00-01T00:00:00Z",
        "2025-04-31T00:00:00Z",
        "2025-10-22_15:45:30Z",
        "2025/10/22T15:45:30Z",
        "+2025-10-22T15:45:30Z",
        "20251022T154530Z",
        "2025-10-22T15:45:30Z[UTC]",
        "2025-10-22T15:45:30.5+03:00x",
        "２025-10-22T15:45:30Z",
        "-0001-12-31T23:59:59Z",
    ] {
        assert_eq!(parse_unix_seconds(text), None, "{text}");
    }
}

#[test]
fn lenient_parser_reads_chrono_and_jiff_forms() {
    let instant = at(1_761_147_930, 250_000_000);
    for text in [
        "2025-10-22T15:45:30.250Z",
        "2025-10-22T15:45:30.25Z",
        "2025-10-22T12:45:30.250-03:00",
        "2025-10-22 15:45:30.250+00:00",
        "2025-10-22T12:45:30.250\u{2212}03:00",
    ] {
        assert_eq!(parse_timestamp(text), Some(instant), "{text}");
    }
    // jiff reads a leap second as the second before it.
    assert_eq!(
        parse_timestamp("2025-10-22T15:45:60Z"),
        Some(at(1_761_147_959, 0))
    );
    assert_eq!(
        parse_timestamp("-0001-12-31T23:59:59Z"),
        Some(at(-62_167_219_201, 0))
    );
    assert_eq!(
        parse_timestamp("+0001-01-01T00:00:00.5\u{2212}00:30"),
        Some(at(-62_135_595_000, 500_000_000))
    );
    assert_eq!(parse_timestamp("-12345-01-01T00:00:00Z"), None);
    assert_eq!(parse_timestamp("2025-10-22T15:45:30"), None);
    assert_eq!(parse_timestamp("yesterday"), None);
}

#[test]
fn serde_round_trips_in_chrono_form() {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Record {
        #[serde(with = "serde_utc")]
        at: Timestamp,
        #[serde(default, with = "serde_utc_option")]
        later: Option<Timestamp>,
    }
    let record = Record {
        at: at(1_761_147_930, 250_000_000),
        later: Some(at(1_761_147_930, 1_000)),
    };
    let json = serde_json::to_string(&record).unwrap();
    assert_eq!(
        json,
        r#"{"at":"2025-10-22T15:45:30.250Z","later":"2025-10-22T15:45:30.000001Z"}"#
    );
    assert_eq!(serde_json::from_str::<Record>(&json).unwrap(), record);
    let none = Record {
        later: None,
        ..record
    };
    assert_eq!(
        serde_json::to_string(&none).unwrap(),
        r#"{"at":"2025-10-22T15:45:30.250Z","later":null}"#
    );
    let missing: Record = serde_json::from_str(r#"{"at":"2025-10-22T15:45:30.250Z"}"#).unwrap();
    assert_eq!(missing, none);
    let error = serde_json::from_str::<Record>(r#"{"at":"soon"}"#).unwrap_err();
    assert!(error.to_string().contains("invalid RFC 3339"), "{error}");
    assert!(serde_json::from_str::<Record>(r#"{"at":5}"#).is_err());
}

#[test]
fn civil_days_round_trip() {
    for days in [
        -1_000_000, -719_529, -1, 0, 59, 60, 11_016, 47_846, 2_932_896,
    ] {
        let (year, month, day) = civil_from_days(days);
        assert_eq!(days_from_civil(year, month, day), days);
    }
    assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    assert_eq!(days_from_civil(2100, 12, 31), 47_846);
}
