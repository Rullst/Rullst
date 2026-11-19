#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::field::{DAY_OF_MONTH, HOUR, MINUTE, MONTH, Values};
use super::*;
use jiff::SignedDuration;
use jiff::civil::Weekday;

fn next_days(expression: &str, count: usize) -> Vec<(Weekday, i8)> {
    let schedule = CronSchedule::parse(expression).unwrap();
    // Wednesday 2026-09-30 00:00 UTC.
    let mut cursor = utc(2026, 9, 30, 0, 0);
    let mut days = Vec::new();
    for _ in 0..count {
        cursor = schedule.next_after(&cursor).unwrap();
        let civil = Offset::UTC.to_datetime(cursor);
        days.push((civil.weekday(), civil.day()));
    }
    days
}

fn weekdays(expression: &str, count: usize) -> Vec<Weekday> {
    next_days(expression, count)
        .into_iter()
        .map(|(weekday, _)| weekday)
        .collect()
}

fn utc(year: i16, month: i8, day: i8, hour: i8, minute: i8) -> Timestamp {
    Offset::UTC
        .to_timestamp(jiff::civil::date(year, month, day).at(hour, minute, 0, 0))
        .unwrap()
}

fn next(expression: &str, after: Timestamp) -> Option<Timestamp> {
    CronSchedule::parse(expression).unwrap().next_after(&after)
}

/// The set of `values`.
fn set(values: &[u32]) -> Values {
    Values::from_bits(values.iter().map(|value| 1_u64 << value).sum())
}

#[test]
fn numeric_weekdays_follow_posix_numbering() {
    use Weekday::*;
    assert_eq!(
        weekdays("0 9 * * 1-5", 7),
        [
            Wednesday, Thursday, Friday, Monday, Tuesday, Wednesday, Thursday
        ]
    );
    assert_eq!(weekdays("0 3 * * 0", 2), [Sunday, Sunday]);
    assert_eq!(weekdays("0 3 * * 7", 2), [Sunday, Sunday]);
    assert_eq!(weekdays("0 3 * * 6", 1), [Saturday]);
    assert_eq!(weekdays("0 3 * * 5-7", 3), [Friday, Saturday, Sunday]);
    assert_eq!(weekdays("0 3 * * FRI-SUN", 3), [Friday, Saturday, Sunday]);
    assert_eq!(
        weekdays("0 3 * * 1,3,sat", 4),
        [Wednesday, Saturday, Monday, Wednesday]
    );
    assert_eq!(
        weekdays("0 3 * * 1-5/2", 4),
        [Wednesday, Friday, Monday, Wednesday]
    );
    assert_eq!(weekdays("0 3 * * */3", 3), [Wednesday, Saturday, Sunday]);
    assert_eq!(
        weekdays("0 3 * * mon-fri", 3),
        [Wednesday, Thursday, Friday]
    );
    assert_eq!(weekdays("0 3 * * 0,7", 2), [Sunday, Sunday]);
    assert_eq!(weekdays("0 3 * * 0-7", 3), [Wednesday, Thursday, Friday]);
}

#[test]
fn restricted_day_fields_match_either_day() {
    // POSIX: the 1st of the month OR every Monday.
    let days = next_days("0 0 1 * 1", 4);
    assert_eq!(
        days,
        [
            (Weekday::Thursday, 1),
            (Weekday::Monday, 5),
            (Weekday::Monday, 12),
            (Weekday::Monday, 19)
        ]
    );
    // A star-prefixed day field keeps the intersection (Vixie cron).
    let days = next_days("0 0 */2 * 1", 2);
    assert_eq!(days, [(Weekday::Monday, 5), (Weekday::Monday, 19)]);
    let saturday = utc(2026, 11, 14, 12, 0);
    assert_eq!(next("0 0 13 * 5", saturday), Some(utc(2026, 11, 20, 0, 0)));
    assert_eq!(next("0 0 13 * ?", saturday), Some(utc(2026, 12, 13, 0, 0)));
    assert_eq!(next("0 0 ? * 5", saturday), Some(utc(2026, 11, 20, 0, 0)));
    assert_eq!(next("0 0 */2 * 1", saturday), Some(utc(2026, 11, 23, 0, 0)));
    // Restricting day-of-week to every day still selects every day.
    assert_eq!(
        next("0 0 13 * 0-6", saturday),
        Some(utc(2026, 11, 15, 0, 0))
    );
}

#[test]
fn schedules_are_evaluated_in_utc() {
    let schedule = CronSchedule::parse("30 23 * * *").unwrap();
    let start = utc(2026, 9, 30, 12, 0);
    assert_eq!(
        schedule.next_after(&start).unwrap(),
        utc(2026, 9, 30, 23, 30)
    );
}

#[test]
fn malformed_expressions_are_rejected() {
    for expression in [
        "* * * *",
        "* * * * * *",
        "* * * * 8",
        "* * * * 5-1",
        "* * * * 1/2",
        "* * * * */0",
        "* * * * funday",
        "* * * * 1-",
        "61 * * * *",
    ] {
        assert!(
            CronSchedule::parse(expression).is_err(),
            "accepted `{expression}`"
        );
    }
}

#[test]
fn every_weekday_name_and_single_day_ranges_parse() {
    use Weekday::*;
    assert_eq!(
        weekdays("0 3 * * tue,wednesday,thurs", 3),
        [Wednesday, Thursday, Tuesday]
    );
    assert_eq!(weekdays("0 3 * * TUES", 1), [Tuesday]);
    assert_eq!(weekdays("0 3 * * 3-3", 2), [Wednesday, Wednesday]);
    assert_eq!(weekdays("0 3 * * 0-0", 2), [Sunday, Sunday]);
    let error = CronSchedule::parse("0 3 * * funday").err().unwrap();
    assert!(error.contains("`funday` is not a POSIX weekday"), "{error}");
}

#[test]
fn fields_accept_the_documented_syntax() {
    let cases: &[(&field::Unit, &str, &[u32])] = &[
        (&MINUTE, "*/15", &[0, 15, 30, 45]),
        (&MINUTE, "*/59", &[0, 59]),
        (&MINUTE, "50-59/20", &[50]),
        (&MINUTE, "5/20", &[5, 25, 45]),
        (&MINUTE, "58/5", &[58]),
        (&MINUTE, "1-5,3", &[1, 2, 3, 4, 5]),
        (&MINUTE, "0,30,59", &[0, 30, 59]),
        (&MINUTE, "007", &[7]),
        (&MINUTE, "0-0", &[0]),
        (&HOUR, "*/23", &[0, 23]),
        (&HOUR, "9-17/4", &[9, 13, 17]),
        (&HOUR, "23", &[23]),
        (&DAY_OF_MONTH, "*/31", &[1]),
        (&DAY_OF_MONTH, "?/10", &[1, 11, 21, 31]),
        (&DAY_OF_MONTH, "1,15,31", &[1, 15, 31]),
        (&DAY_OF_MONTH, "28-31", &[28, 29, 30, 31]),
        (&MONTH, "*/12", &[1]),
        (&MONTH, "jan-dec/5", &[1, 6, 11]),
        (&MONTH, "JAN-mar", &[1, 2, 3]),
        (&MONTH, "may,September,DEC", &[5, 9, 12]),
        (&MONTH, "june-july", &[6, 7]),
        (&MONTH, "2/5", &[2, 7, 12]),
        (&MONTH, "12", &[12]),
    ];
    for &(unit, field, expected) in cases {
        assert_eq!(unit.parse(field), Ok(set(expected)), "`{field}`");
    }
    assert_eq!(MINUTE.parse("*"), Ok(Values::from_bits((1 << 60) - 1)));
    assert_eq!(HOUR.parse("*"), Ok(Values::from_bits((1 << 24) - 1)));
    assert_eq!(MONTH.parse("*"), MONTH.parse("1-12"));
    assert_eq!(DAY_OF_MONTH.parse("?"), DAY_OF_MONTH.parse("1-31"));
    assert_eq!(DAY_OF_MONTH.parse("?,5"), DAY_OF_MONTH.parse("1-31"));
}

#[test]
fn fields_reject_what_the_cron_crate_rejected() {
    let cases: &[(&field::Unit, &str)] = &[
        (&MINUTE, "60"),
        (&MINUTE, "*/60"),
        (&MINUTE, "*/0"),
        (&MINUTE, "*/+5"),
        (&MINUTE, "+5"),
        (&MINUTE, "-5"),
        (&MINUTE, "5-"),
        (&MINUTE, "5-1"),
        (&MINUTE, "1-5-7"),
        (&MINUTE, "1,,2"),
        (&MINUTE, "1,"),
        (&MINUTE, ",1"),
        (&MINUTE, "*/5/2"),
        (&MINUTE, "*-5"),
        (&MINUTE, "?"),
        (&MINUTE, "4294967296"),
        (&MINUTE, "jan"),
        (&MINUTE, "@hourly"),
        (&HOUR, "24"),
        (&HOUR, "*/24"),
        (&HOUR, "?"),
        (&DAY_OF_MONTH, "0"),
        (&DAY_OF_MONTH, "32"),
        (&DAY_OF_MONTH, "*/32"),
        (&DAY_OF_MONTH, "L"),
        (&DAY_OF_MONTH, "15W"),
        (&DAY_OF_MONTH, "?-5"),
        (&MONTH, "0"),
        (&MONTH, "13"),
        (&MONTH, "*/13"),
        (&MONTH, "jan/2"),
        (&MONTH, "1-mar"),
        (&MONTH, "mar-jan"),
        (&MONTH, "sept"),
    ];
    for &(unit, field) in cases {
        assert!(unit.parse(field).is_err(), "accepted `{field}`");
    }
    let error = MONTH.parse("jan/2").unwrap_err();
    assert!(error.contains("steps a single name"), "{error}");
    let error = HOUR.parse("1-24").unwrap_err();
    assert_eq!(error, "hour `1-24`: `24` must be 0-23");
}

#[test]
fn a_field_must_be_valid_on_its_own() {
    // The `cron` crate joined these with the next field (`1, 2` read as the
    // minutes 1 and 2) or split them (`5jan`); each field now stands alone.
    for expression in [
        "1, 2 3 4 5",
        "1 ,2 3 4 5",
        "1- 5 * * *",
        "*/ 5 * * *",
        "0 0 * 1, 1",
        "0 0 5jan * *",
        "@daily",
        "0 0 L * *",
        "0 0 * * 1#2",
    ] {
        assert!(
            CronSchedule::parse(expression).is_err(),
            "accepted `{expression}`"
        );
    }
}

#[test]
fn next_occurrences_are_whole_minutes_strictly_after() {
    let every_minute = CronSchedule::parse("* * * * *").unwrap();
    let ten = utc(2026, 11, 14, 10, 0);
    assert_eq!(
        every_minute.next_after(&ten),
        Some(utc(2026, 11, 14, 10, 1))
    );
    let late = ten + SignedDuration::from_millis(59_999);
    assert_eq!(
        every_minute.next_after(&late),
        Some(utc(2026, 11, 14, 10, 1))
    );
    let early = ten - SignedDuration::from_nanos(1);
    assert_eq!(every_minute.next_after(&early), Some(ten));
    // A later minute in the same hour, then the next allowed hour.
    assert_eq!(next("0,45 * * * *", ten), Some(utc(2026, 11, 14, 10, 45)));
    assert_eq!(
        next("0 * * * *", utc(2026, 11, 14, 10, 30)),
        Some(utc(2026, 11, 14, 11, 0))
    );
    assert_eq!(
        next("15 3,20 * * *", utc(2026, 11, 14, 10, 30)),
        Some(utc(2026, 11, 14, 20, 15))
    );
    assert_eq!(
        next("15 3 * * *", utc(2026, 11, 14, 10, 30)),
        Some(utc(2026, 11, 15, 3, 15))
    );
    // The same day of a later month starts that day at midnight.
    assert_eq!(
        next("0 5 15 * *", utc(2026, 1, 15, 10, 30)),
        Some(utc(2026, 2, 15, 5, 0))
    );
    assert_eq!(
        next("30 23 * 3 *", utc(2026, 1, 1, 0, 0)),
        Some(utc(2026, 3, 1, 23, 30))
    );
}

#[test]
fn month_ends_and_leap_days() {
    let march = utc(2025, 3, 1, 0, 0);
    assert_eq!(next("0 0 29 2 *", march), Some(utc(2028, 2, 29, 0, 0)));
    // 2000 is a leap year (divisible by 400), 2100 is not (by 100).
    assert_eq!(
        next("0 0 29 2 *", utc(1999, 3, 1, 0, 0)),
        Some(utc(2000, 2, 29, 0, 0))
    );
    assert_eq!(next("0 0 29 2 *", utc(2096, 2, 29, 0, 0)), None);
    assert_eq!(
        next("0 0 28 2 *", utc(2100, 1, 1, 0, 0)),
        Some(utc(2100, 2, 28, 0, 0))
    );
    // February never has a 30th or 31st, and April no 31st.
    assert_eq!(next("0 0 30 2 *", march), None);
    assert_eq!(next("0 0 31 2 *", march), None);
    assert_eq!(next("0 0 31 4 *", march), None);
    // With day-of-week restricted too, the Mondays of February still run.
    assert_eq!(
        next("0 0 30 2 1", utc(2026, 11, 14, 0, 0)),
        Some(utc(2027, 2, 1, 0, 0))
    );
    assert_eq!(
        next("0 12 31 * *", utc(2026, 4, 1, 0, 0)),
        Some(utc(2026, 5, 31, 12, 0))
    );
    assert_eq!(
        next("0 0 30,31 * *", utc(2027, 2, 1, 0, 0)),
        Some(utc(2027, 3, 30, 0, 0))
    );
}

#[test]
fn year_ends_and_search_bounds() {
    assert_eq!(
        next("59 23 31 12 *", utc(2026, 12, 31, 23, 59)),
        Some(utc(2027, 12, 31, 23, 59))
    );
    let last_moment = utc(2026, 12, 31, 23, 59) + SignedDuration::from_millis(59_999);
    assert_eq!(next("0 0 1 1 *", last_moment), Some(utc(2027, 1, 1, 0, 0)));
    // The search ends with 2100.
    assert_eq!(
        next("59 23 31 12 *", utc(2100, 12, 31, 23, 58)),
        Some(utc(2100, 12, 31, 23, 59))
    );
    assert_eq!(next("59 23 31 12 *", utc(2100, 12, 31, 23, 59)), None);
    assert_eq!(next("* * * * *", utc(2101, 1, 1, 0, 0)), None);
    assert_eq!(next("0 0 1 1 *", utc(2100, 1, 1, 0, 0)), None);
    assert_eq!(next("* * * * *", Timestamp::MAX), None);
    // It starts in 1970, and an instant whose next second is before year 0
    // has no occurrence.
    assert_eq!(
        next("0 0 * * *", utc(1960, 5, 5, 10, 0)),
        Some(utc(1970, 1, 1, 0, 0))
    );
    assert_eq!(
        next("0 0 * * 1", utc(1969, 12, 31, 23, 59)),
        Some(utc(1970, 1, 5, 0, 0))
    );
    assert_eq!(
        next("* * * * *", utc(0, 6, 1, 0, 0)),
        Some(utc(1970, 1, 1, 0, 0))
    );
    let year_zero = utc(-1, 12, 31, 23, 59) + SignedDuration::from_secs(59);
    assert_eq!(next("* * * * *", year_zero), Some(utc(1970, 1, 1, 0, 0)));
    let before_zero = year_zero - SignedDuration::from_secs(1);
    assert_eq!(next("* * * * *", before_zero), None);
    assert_eq!(next("* * * * *", utc(-5, 1, 1, 0, 0)), None);
}

#[test]
fn values_report_membership_and_the_next_member() {
    let values = set(&[3, 10, 59]);
    assert!(values.contains(3));
    assert!(values.contains(59));
    assert!(!values.contains(4));
    assert!(!values.contains(64));
    assert_eq!(values.first_from(0), Some(3));
    assert_eq!(values.first_from(3), Some(3));
    assert_eq!(values.first_from(4), Some(10));
    assert_eq!(values.first_from(11), Some(59));
    assert_eq!(values.first_from(60), None);
    assert_eq!(values.first_from(64), None);
}
