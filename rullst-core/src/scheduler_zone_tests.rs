#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use chrono::TimeZone as _;

fn utc(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
        .unwrap()
}

fn zoned(expression: &str, zone: &str) -> ZonedCron {
    ZonedCron::new(CronSchedule::parse(expression).unwrap(), zone).unwrap()
}

/// The next `count` runs after `after`, each searched after the previous one.
fn runs(expression: &str, zone: &str, after: DateTime<Utc>, count: usize) -> Vec<DateTime<Utc>> {
    let schedule = zoned(expression, zone);
    let mut cursor = after;
    let mut runs = Vec::new();
    for _ in 0..count {
        cursor = schedule.next_after(&cursor).unwrap();
        runs.push(cursor);
    }
    runs
}

#[test]
fn sao_paulo_runs_at_its_local_time_all_year() {
    // UTC-3 with no daylight saving since 2019, also in its former DST season.
    assert_eq!(
        runs(
            "0 7 * * *",
            "America/Sao_Paulo",
            utc(2026, 11, 14, 12, 0),
            2
        ),
        [utc(2026, 11, 15, 10, 0), utc(2026, 11, 16, 10, 0)]
    );
    assert_eq!(
        runs("0 7 * * *", "America/Sao_Paulo", utc(2026, 1, 10, 0, 0), 1),
        [utc(2026, 1, 10, 10, 0)]
    );
    // Day fields match the local date: Friday 22:30 is Saturday 01:30 UTC.
    assert_eq!(
        runs(
            "30 22 * * 5",
            "America/Sao_Paulo",
            utc(2026, 10, 16, 12, 0),
            2
        ),
        [utc(2026, 10, 17, 1, 30), utc(2026, 10, 24, 1, 30)]
    );
}

#[test]
fn a_run_at_the_search_instant_is_not_repeated() {
    let schedule = zoned("0 7 * * *", "America/Sao_Paulo");
    assert_eq!(
        schedule.next_after(&utc(2026, 11, 15, 10, 0)),
        Some(utc(2026, 11, 16, 10, 0))
    );
    let just_before = utc(2026, 11, 15, 9, 59) + TimeDelta::milliseconds(59_500);
    assert_eq!(
        schedule.next_after(&just_before),
        Some(utc(2026, 11, 15, 10, 0))
    );
}

#[test]
fn a_time_skipped_by_spring_forward_runs_when_the_gap_ends() {
    // New York, 2026-03-08: 02:00 EST jumps to 03:00 EDT (07:00 UTC).
    assert_eq!(
        runs("30 2 * * *", "America/New_York", utc(2026, 3, 7, 0, 0), 3),
        [
            utc(2026, 3, 7, 7, 30), // 02:30 EST
            utc(2026, 3, 8, 7, 0),  // 02:30 does not exist: 03:00 EDT
            utc(2026, 3, 9, 6, 30), // 02:30 EDT
        ]
    );
    // Every skipped time runs once at the end of the gap, then the schedule
    // continues on the new offset.
    assert_eq!(
        runs(
            "*/15 * * * *",
            "America/New_York",
            utc(2026, 3, 8, 6, 40),
            4
        ),
        [
            utc(2026, 3, 8, 6, 45), // 01:45 EST
            utc(2026, 3, 8, 7, 0),  // 02:00-02:45 skipped: 03:00 EDT
            utc(2026, 3, 8, 7, 15), // 03:15 EDT
            utc(2026, 3, 8, 7, 30),
        ]
    );
}

#[test]
fn a_gap_in_a_zone_east_of_utc_also_runs_at_its_end() {
    // Berlin, 2026-03-29: 02:00 CET jumps to 03:00 CEST (01:00 UTC).
    assert_eq!(
        runs("30 2 * * *", "Europe/Berlin", utc(2026, 3, 28, 12, 0), 2),
        [utc(2026, 3, 29, 1, 0), utc(2026, 3, 30, 0, 30)]
    );
}

#[test]
fn a_time_repeated_by_fall_back_runs_once_at_its_first_occurrence() {
    // New York, 2026-11-01: 02:00 EDT falls back to 01:00 EST (06:00 UTC).
    assert_eq!(
        runs("30 1 * * *", "America/New_York", utc(2026, 11, 1, 4, 0), 2),
        [
            utc(2026, 11, 1, 5, 30), // 01:30 EDT
            utc(2026, 11, 2, 6, 30), // next day; never 01:30 EST on the 1st
        ]
    );
    // Searching from inside the repeated hour (01:10 EST) skips its matches.
    assert_eq!(
        runs("30 1 * * *", "America/New_York", utc(2026, 11, 1, 6, 10), 1),
        [utc(2026, 11, 2, 6, 30)]
    );
    // An hourly task runs at 01:00 EDT and then 02:00 EST, not at 01:00 EST.
    assert_eq!(
        runs("0 * * * *", "America/New_York", utc(2026, 11, 1, 4, 30), 3),
        [
            utc(2026, 11, 1, 5, 0),
            utc(2026, 11, 1, 7, 0),
            utc(2026, 11, 1, 8, 0)
        ]
    );
}

#[test]
fn the_utc_zone_matches_the_utc_scheduler() {
    for expression in ["0 7 * * *", "*/20 9-17 * * 1-5", "0 0 1 * 1", "30 2 * * *"] {
        let zoned = zoned(expression, "UTC");
        let plain = CronSchedule::parse(expression).unwrap();
        // Crosses New York's spring-forward day, which UTC ignores.
        let mut cursor = utc(2026, 3, 7, 0, 0);
        for _ in 0..40 {
            let next = zoned.next_after(&cursor).unwrap();
            assert_eq!(Some(next), plain.next_after(&cursor), "{expression}");
            cursor = next;
        }
    }
    assert_eq!(
        runs("30 2 * * *", "UTC", utc(2026, 3, 8, 0, 0), 1),
        [utc(2026, 3, 8, 2, 30)]
    );
}

#[test]
fn task_in_tz_registers_a_labelled_zoned_task() {
    let scheduler = Scheduler::new()
        .task_in_tz("0 7 * * *", "America/Sao_Paulo", || async {})
        .unwrap();
    let [task] = &scheduler.tasks[..] else {
        panic!("expected one task");
    };
    assert_eq!(task.label, "0 7 * * * [America/Sao_Paulo]");
    assert_eq!(
        task.schedule.next_after(&utc(2026, 11, 14, 12, 0)),
        Some(utc(2026, 11, 15, 10, 0))
    );
    assert!(task.deferral.is_none());
}

#[test]
fn unknown_zones_and_bad_expressions_are_typed_registration_errors() {
    for zone in ["Mars/Olympus_Mons", "", "Brasilia"] {
        match Scheduler::new().task_in_tz("0 7 * * *", zone, || async {}) {
            Err(SchedulerError::InvalidTimeZone {
                zone: rejected,
                message,
            }) => {
                assert_eq!(rejected, zone);
                assert!(!message.is_empty());
            }
            Err(other) => panic!("unexpected error for {zone:?}: {other}"),
            Ok(_) => panic!("accepted time zone {zone:?}"),
        }
    }
    let error = Scheduler::new()
        .task_in_tz("0 7 * *", "America/Sao_Paulo", || async {})
        .err()
        .unwrap();
    assert!(
        matches!(&error, SchedulerError::InvalidCron(expression, _) if expression == "0 7 * *")
    );
    let error = SchedulerError::InvalidTimeZone {
        zone: "Brasilia".to_string(),
        message: "not in the database".to_string(),
    };
    assert_eq!(
        error.to_string(),
        "unknown time zone 'Brasilia': not in the database"
    );
}
