//! The value bound for each typed date or time, per driver.

use jiff::civil::{DateTime, date, time};
use rullst_orm::RullstValue;

use super::driver_value;
use crate::nexus::crud::input::TemporalValue;
use crate::nexus::instant::Instant;

fn instant(local: &str, offset: i32) -> TemporalValue {
    let local: DateTime = local.parse().unwrap();
    TemporalValue::Instant(Instant::at_offset(local, offset).unwrap())
}

fn bound(value: TemporalValue, driver: &str) -> RullstValue {
    driver_value(value, driver).unwrap()
}

#[test]
fn dates_times_and_local_date_times_bind_their_jiff_type_everywhere() {
    for driver in ["sqlite", "postgres", "mysql"] {
        match bound(TemporalValue::Date(date(2026, 10, 15)), driver) {
            RullstValue::CivilDate(value) => assert_eq!(*value, date(2026, 10, 15)),
            other => panic!("{driver}: {other:?}"),
        }
        match bound(TemporalValue::Time(time(9, 30, 0, 500_000_000)), driver) {
            RullstValue::CivilTime(value) => assert_eq!(*value, time(9, 30, 0, 500_000_000)),
            other => panic!("{driver}: {other:?}"),
        }
        let local = date(2026, 10, 15).at(9, 30, 0, 0);
        match bound(TemporalValue::DateTime(local), driver) {
            RullstValue::CivilDateTime(value) => assert_eq!(*value, local),
            other => panic!("{driver}: {other:?}"),
        }
    }
}

#[test]
fn mysql_binds_a_utc_instant_as_its_utc_wall_time() {
    let value = instant("2026-10-15T06:30:00.5", -3 * 3600);
    match bound(value, "mysql") {
        RullstValue::CivilDateTime(wall) => {
            assert_eq!(*wall, date(2026, 10, 15).at(9, 30, 0, 500_000_000));
        }
        other => panic!("{other:?}"),
    }
    for driver in ["postgres", "sqlite"] {
        match bound(value, driver) {
            RullstValue::Timestamp(at) => {
                assert_eq!(at.as_second(), 1_792_056_600);
                assert_eq!(at.subsec_nanosecond(), 500_000_000);
            }
            other => panic!("{driver}: {other:?}"),
        }
    }
}

#[test]
fn instants_past_jiff_timestamps_bind_their_chrono_era_text() {
    // Later than 9999-12-30T22:00Z: no `jiff::Timestamp`, but MySQL's UTC
    // wall time is still a jiff date-time.
    let late = instant("9999-12-31T23:30:00", 0);
    for driver in ["postgres", "sqlite"] {
        match bound(late, driver) {
            RullstValue::String(text) => assert_eq!(text, "9999-12-31T23:30:00+00:00"),
            other => panic!("{driver}: {other:?}"),
        }
    }
    match bound(late, "mysql") {
        RullstValue::CivilDateTime(wall) => {
            assert_eq!(*wall, date(9999, 12, 31).at(23, 30, 0, 0));
        }
        other => panic!("{other:?}"),
    }

    let year_10000 = instant("9999-12-31T23:00:00.25", -5 * 3600);
    for driver in ["postgres", "sqlite"] {
        match bound(year_10000, driver) {
            RullstValue::String(text) => assert_eq!(text, "+10000-01-01T04:00:00.250+00:00"),
            other => panic!("{driver}: {other:?}"),
        }
    }
    match bound(year_10000, "mysql") {
        RullstValue::String(text) => assert_eq!(text, "+10000-01-01 04:00:00.250"),
        other => panic!("{other:?}"),
    }
}
