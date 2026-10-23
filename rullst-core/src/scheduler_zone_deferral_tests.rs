//! Deferrable tasks whose ticks and windows are wall times of a time zone,
//! checked with fixed instants around daylight-saving changes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::queue::TimeWindow;
use std::time::Duration;

const NEW_YORK: &str = "America/New_York";

fn at(text: &str) -> i64 {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .timestamp_millis()
}

fn deferral(hours: u64, windows: &[(u8, u8, u8, u8)]) -> ScheduleDeferral {
    windows.iter().fold(
        ScheduleDeferral::within(Duration::from_secs(hours * 3_600)).unwrap(),
        |deferral, (start_hour, start_minute, end_hour, end_minute)| {
            deferral
                .window(
                    TimeWindow::daily(*start_hour, *start_minute, *end_hour, *end_minute).unwrap(),
                )
                .unwrap()
        },
    )
}

fn zoned(deferral: ScheduleDeferral, zone: &str) -> TaskDeferral {
    TaskDeferral::Zoned(deferral, TimeZone::get(zone).unwrap())
}

/// The start of the run due at `tick`, as RFC 3339 UTC, and its reason.
fn start(deferral: &TaskDeferral, tick: &str) -> (String, &'static str) {
    let (start_ms, reason) = deferral.start_for(at(tick));
    let start = DateTime::from_timestamp_millis(start_ms).unwrap();
    (start.to_rfc3339(), reason.as_str())
}

fn expect(start: &str, reason: &'static str) -> (String, &'static str) {
    (start.to_string(), reason)
}

#[test]
fn a_night_window_follows_the_zone_across_the_spring_change() {
    let night = zoned(deferral(12, &[(22, 0, 6, 0)]), NEW_YORK);
    // 18:00 EST on 7 March 2026 waits for 22:00 EST.
    assert_eq!(
        start(&night, "2026-03-07T23:00:00Z"),
        expect("2026-03-08T03:00:00+00:00", "window")
    );
    // A day later clocks are on EDT: 22:00 is 02:00 UTC, an hour earlier
    // than the fixed UTC−05:00 offset of a plain deferrable task.
    assert_eq!(
        start(&night, "2026-03-08T22:00:00Z"),
        expect("2026-03-09T02:00:00+00:00", "window")
    );
    let fixed = TaskDeferral::Utc(
        ScheduleDeferral::within(Duration::from_secs(12 * 3_600))
            .unwrap()
            .window(
                TimeWindow::daily(22, 0, 6, 0)
                    .unwrap()
                    .with_utc_offset_minutes(-300)
                    .unwrap(),
            )
            .unwrap(),
    );
    assert_eq!(
        start(&fixed, "2026-03-08T22:00:00Z"),
        expect("2026-03-09T03:00:00+00:00", "window")
    );
    // Inside an open window the run starts at its tick.
    assert_eq!(
        start(&night, "2026-03-08T04:30:00Z"),
        expect("2026-03-08T04:30:00+00:00", "window")
    );
}

#[test]
fn window_boundaries_in_a_gap_move_to_its_end_and_an_empty_window_is_skipped() {
    // On 8 March 2026 clocks jump from 02:00 EST to 03:00 EDT (07:00 UTC).
    let late_start = zoned(deferral(48, &[(2, 30, 4, 0)]), NEW_YORK);
    assert_eq!(
        start(&late_start, "2026-03-08T05:00:00Z"),
        expect("2026-03-08T07:00:00+00:00", "window")
    );
    // 02:00–02:30 does not exist that day; it opens again on the 9th (EDT).
    let skipped = zoned(deferral(48, &[(2, 0, 2, 30)]), NEW_YORK);
    assert_eq!(
        start(&skipped, "2026-03-08T05:00:00Z"),
        expect("2026-03-09T06:00:00+00:00", "window")
    );
}

#[test]
fn window_boundaries_in_a_fold_are_their_first_occurrence() {
    // On 1 November 2026 clocks go back from 02:00 EDT to 01:00 EST, so
    // 01:30 happens at 05:30 and again at 06:30 UTC.
    let early = zoned(deferral(12, &[(1, 30, 3, 0)]), NEW_YORK);
    assert_eq!(
        start(&early, "2026-11-01T04:00:00Z"),
        expect("2026-11-01T05:30:00+00:00", "window")
    );
    // 01:15 EST (06:15 UTC) is inside 00:00 EDT–06:00 EST.
    let small_hours = zoned(deferral(24, &[(0, 0, 6, 0)]), NEW_YORK);
    assert_eq!(
        start(&small_hours, "2026-11-01T06:15:00Z"),
        expect("2026-11-01T06:15:00+00:00", "window")
    );
    // That window closes at 06:00 EST, 11:00 UTC.
    assert_eq!(
        start(&small_hours, "2026-11-01T11:00:00Z"),
        expect("2026-11-02T05:00:00+00:00", "window")
    );
}

#[test]
fn deadlines_and_windowless_deferrals_behave_as_in_utc() {
    let short = zoned(deferral(1, &[(22, 0, 23, 0)]), NEW_YORK);
    assert_eq!(
        start(&short, "2026-03-08T12:00:00Z"),
        expect("2026-03-08T13:00:00+00:00", "deadline")
    );
    // A window that opens exactly at the deadline is not used.
    let at_deadline = zoned(deferral(2, &[(22, 0, 23, 0)]), NEW_YORK);
    assert_eq!(
        start(&at_deadline, "2026-03-08T01:00:00Z"),
        expect("2026-03-08T03:00:00+00:00", "deadline")
    );
    let immediate = zoned(deferral(0, &[(22, 0, 23, 0)]), NEW_YORK);
    assert_eq!(
        start(&immediate, "2026-03-08T12:00:00Z"),
        expect("2026-03-08T12:00:00+00:00", "deadline")
    );
    let anytime = zoned(deferral(3, &[]), NEW_YORK);
    assert_eq!(
        start(&anytime, "2026-03-08T12:00:00Z"),
        expect("2026-03-08T12:00:00+00:00", "window")
    );
    // São Paulo has no daylight saving: 00:00–06:00 is 03:00–09:00 UTC.
    let sao_paulo = zoned(deferral(12, &[(0, 0, 6, 0)]), "America/Sao_Paulo");
    assert_eq!(
        start(&sao_paulo, "2026-10-15T21:00:00Z"),
        expect("2026-10-16T03:00:00+00:00", "window")
    );
}

#[test]
fn the_wall_clock_conversions_round_trip_outside_changes() {
    let zone = TimeZone::get(NEW_YORK).unwrap();
    assert_eq!(
        wall_ms(&zone, at("2026-03-08T12:00:00Z")),
        Some(at("2026-03-08T08:00:00Z"))
    );
    assert_eq!(
        instant_ms(&zone, at("2026-03-08T08:00:00Z")),
        Some(at("2026-03-08T12:00:00Z"))
    );
    assert_eq!(
        instant_ms(&zone, at("2026-03-08T02:30:00Z")),
        Some(at("2026-03-08T07:00:00Z"))
    );
    assert_eq!(wall_ms(&zone, i64::MAX), None);
    assert_eq!(instant_ms(&zone, i64::MAX), None);
}

#[test]
fn deferrable_task_in_tz_registers_a_zoned_tick_and_zoned_windows() {
    let scheduler = Scheduler::new()
        .deferrable_task_in_tz(
            "0 18 * * *",
            NEW_YORK,
            deferral(12, &[(22, 0, 6, 0)]),
            || async {},
        )
        .unwrap();
    let [task] = &scheduler.tasks[..] else {
        panic!("expected one task");
    };
    assert_eq!(task.label, "0 18 * * * [America/New_York]");
    // 18:00 EDT on 8 March is 22:00 UTC; its run waits for 22:00 EDT.
    let tick = task
        .schedule
        .next_after(&DateTime::from_timestamp_millis(at("2026-03-08T12:00:00Z")).unwrap())
        .unwrap();
    assert_eq!(tick.to_rfc3339(), "2026-03-08T22:00:00+00:00");
    let run = super::super::deferred_start(task, tick).unwrap();
    assert_eq!(run.to_rfc3339(), "2026-03-09T02:00:00+00:00");
}

#[test]
fn zoned_deferrable_tasks_reject_offsets_bad_zones_and_bad_expressions() {
    let offset = ScheduleDeferral::within(Duration::from_secs(3_600))
        .unwrap()
        .window(
            TimeWindow::daily(1, 0, 2, 0)
                .unwrap()
                .with_utc_offset_minutes(60)
                .unwrap(),
        )
        .unwrap();
    let error = Scheduler::new()
        .deferrable_task_in_tz("0 18 * * *", NEW_YORK, offset, || async {})
        .err()
        .unwrap();
    assert_eq!(
        error.to_string(),
        "invalid scheduler configuration: the windows of a task in America/New_York are wall times there; remove their UTC offset"
    );
    let error = Scheduler::new()
        .deferrable_task_in_tz("0 18 * * *", "Brasilia", deferral(1, &[]), || async {})
        .err()
        .unwrap();
    assert!(matches!(error, SchedulerError::InvalidTimeZone { zone, .. } if zone == "Brasilia"));
    let error = Scheduler::new()
        .deferrable_task_in_tz("0 18 * *", NEW_YORK, deferral(1, &[]), || async {})
        .err()
        .unwrap();
    assert!(
        matches!(error, SchedulerError::InvalidCron(expression, _) if expression == "0 18 * *")
    );
}
