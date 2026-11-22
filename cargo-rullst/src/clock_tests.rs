//! The expected strings are what chrono 0.4 wrote for the same instants
//! (`Local::now().format(..)`, `to_rfc3339()`, `to_rfc3339_opts(Secs, true)`
//! and `format("%Y%m%dT%H%M%SZ")`), so names and reports stay byte-identical.

use super::*;
use jiff::tz::{TimeZone, offset};

fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

#[test]
fn migration_stamps_are_local_wall_time_digits() {
    assert_eq!(
        migration_stamp(at("2026-11-17T09:30:05.999Z"), &TimeZone::UTC),
        "20261117093005"
    );
    // São Paulo's offset moves the stamp across midnight and the year.
    let brt = TimeZone::fixed(offset(-3));
    assert_eq!(
        migration_stamp(at("2026-01-01T01:02:03Z"), &brt),
        "20251231220203"
    );
    let posix = TimeZone::posix("NZST-12").unwrap();
    assert_eq!(
        migration_stamp(at("2026-03-04T12:00:00Z"), &posix),
        "20260305000000"
    );
    assert_eq!(
        migration_stamp(at("0999-02-03T04:05:06Z"), &TimeZone::UTC),
        "09990203040506"
    );
}

#[test]
fn migration_stamps_sort_in_creation_order() {
    let zone = TimeZone::fixed(offset(-3));
    let stamps: Vec<String> = [
        "2026-09-30T23:59:59Z",
        "2026-10-01T00:00:00Z",
        "2026-10-01T09:08:07Z",
        "2026-12-31T23:59:59Z",
        "2027-01-01T00:00:00Z",
    ]
    .into_iter()
    .map(|instant| migration_stamp(at(instant), &zone))
    .collect();
    let mut sorted = stamps.clone();
    sorted.sort();
    assert_eq!(stamps, sorted);
    assert!(stamps.iter().all(|stamp| stamp.len() == 14));
}

#[test]
fn ordered_stamps_sort_after_each_other_across_a_clock_change() {
    assert_eq!(
        ordered_migration_stamps(at("2026-11-17T09:30:59Z"), &TimeZone::UTC).unwrap(),
        ("20261117093059".to_string(), "20261117093100".to_string())
    );
    // New York's clocks went back at 06:00 UTC on 2026-11-01: 01:59:59 EDT
    // is followed by 01:00:00 EST, and the repeated hour ends at 02:00:00.
    let new_york = TimeZone::posix("EST5EDT,M3.2.0,M11.1.0").unwrap();
    assert_eq!(
        ordered_migration_stamps(at("2026-11-01T05:59:59Z"), &new_york).unwrap(),
        ("20261101015959".to_string(), "20261101020000".to_string())
    );
    // The spring change skips an hour; one second later still sorts after.
    assert_eq!(
        ordered_migration_stamps(at("2026-03-08T06:59:59Z"), &new_york).unwrap(),
        ("20260308015959".to_string(), "20260308030000".to_string())
    );
    assert!(ordered_migration_stamps(Timestamp::MAX, &TimeZone::UTC).is_err());
}

#[test]
fn local_stamps_read_the_system_clock() {
    let stamp = local_migration_stamp().unwrap();
    assert_eq!(stamp.len(), 14);
    assert!(stamp.bytes().all(|byte| byte.is_ascii_digit()), "{stamp}");
    assert!(stamp.as_str() >= "20260101000000", "{stamp}");
}

#[test]
fn dates_are_local_calendar_days() {
    assert_eq!(
        date(at("2026-11-17T23:59:59Z"), &TimeZone::UTC),
        "2026-11-17"
    );
    assert_eq!(
        date(at("2026-11-18T02:00:00Z"), &TimeZone::fixed(offset(-3))),
        "2026-11-17"
    );
    assert_eq!(
        date(at("2026-11-17T22:00:00Z"), &TimeZone::fixed(offset(9))),
        "2026-11-18"
    );
}

#[test]
fn rfc3339_writes_chronos_auto_fraction_and_offset() {
    for (instant, expected) in [
        ("2026-11-17T09:30:05Z", "2026-11-17T09:30:05+00:00"),
        ("2026-11-17T09:30:05.25Z", "2026-11-17T09:30:05.250+00:00"),
        ("2026-11-17T09:30:05.001Z", "2026-11-17T09:30:05.001+00:00"),
        (
            "2026-11-17T09:30:05.0012Z",
            "2026-11-17T09:30:05.001200+00:00",
        ),
        (
            "2026-11-17T09:30:05.123456Z",
            "2026-11-17T09:30:05.123456+00:00",
        ),
        (
            "2026-11-17T09:30:05.000001Z",
            "2026-11-17T09:30:05.000001+00:00",
        ),
        (
            "2026-11-17T09:30:05.123456789Z",
            "2026-11-17T09:30:05.123456789+00:00",
        ),
        (
            "2026-11-17T09:30:05.00000001Z",
            "2026-11-17T09:30:05.000000010+00:00",
        ),
        // The instant, not the zone it was written in, is formatted in UTC.
        ("2026-11-17T06:30:05-03:00", "2026-11-17T09:30:05+00:00"),
    ] {
        assert_eq!(rfc3339(at(instant)), expected, "{instant}");
    }
}

#[test]
fn rfc3339_seconds_cuts_the_fraction() {
    assert_eq!(
        rfc3339_seconds(at("2026-11-17T09:30:05.999999999Z")),
        "2026-11-17T09:30:05Z"
    );
    assert_eq!(
        rfc3339_seconds(at("2026-01-02T03:04:05+01:00")),
        "2026-01-02T02:04:05Z"
    );
}

#[test]
fn compact_utc_stamps_have_no_separators() {
    assert_eq!(
        compact_utc(at("2026-11-17T09:30:05.75Z")),
        "20261117T093005Z"
    );
    assert_eq!(
        compact_utc(at("2026-01-01T00:30:00+01:00")),
        "20251231T233000Z"
    );
}
