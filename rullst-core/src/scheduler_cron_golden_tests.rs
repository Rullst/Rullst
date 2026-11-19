//! Golden vectors of the `cron` crate 0.17.0, which evaluated the cron fields
//! until 13.0.0-alpha.5, and of the scheduler's evaluation then.
//!
//! They were the oracle of differential property tests until chrono and the
//! `cron` crate left Rullst in 13.0.0-alpha.6. The `generate_golden_vectors`
//! test of commit fa34caf89 wrote them (`cargo test -p rullst-core --features
//! scheduler-tz --lib -- --ignored generate_golden_vectors`), drawing those
//! tests' strategies with the fixed ChaCha seed `[13; 32]`:
//!
//! - `fields.txt`: 2,048 draws of the minute, hour, day-of-month and month
//!   fields, valid or not, each with the values the crate gave it alone;
//! - `expressions.txt`: 768 draws of field-shaped expressions, valid or not,
//!   and 768 of valid ones, each with whether Rullst accepts it and, when it
//!   does, three start instants from 1900 to 2101 (whole minutes or any
//!   nanosecond) and the next four occurrences the 13.0.0-alpha.5 evaluator
//!   computed (`end` when there was none);
//! - `corpus.txt`: the expressions of Rullst's documentation, generators and
//!   tests and calendar edge cases, from 16 start instants, six occurrences
//!   each.
//!
//! A start is Unix seconds, rounded down, and nanoseconds after a `.`; an
//! occurrence is Unix seconds.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::CronSchedule;
use super::field::{DAY_OF_MONTH, HOUR, MINUTE, MONTH, Unit};
use jiff::Timestamp;

fn unit(slot: &str) -> (&'static Unit, u32, u32) {
    match slot {
        "Minute" => (&MINUTE, 0, 59),
        "Hour" => (&HOUR, 0, 23),
        "DayOfMonth" => (&DAY_OF_MONTH, 1, 31),
        "Month" => (&MONTH, 1, 12),
        other => panic!("unknown slot `{other}`"),
    }
}

fn data_lines(text: &'static str) -> impl Iterator<Item = &'static str> {
    text.lines().filter(|line| !line.starts_with('#'))
}

#[test]
fn each_field_parses_as_the_cron_crate_parsed_it() {
    let mut checked = 0;
    for line in data_lines(include_str!("scheduler_cron_golden/fields.txt")) {
        let mut parts = line.split('\t');
        let (slot, field, expected) = (
            parts.next().unwrap(),
            parts.next().unwrap(),
            parts.next().unwrap(),
        );
        let (unit, min, max) = unit(slot);
        let ours = unit
            .parse(field)
            .ok()
            .map(|values| {
                (min..=max)
                    .filter(|value| values.contains(*value))
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_else(|| "reject".to_string());
        assert_eq!(ours, expected, "{slot} `{field}`");
        checked += 1;
    }
    assert!(checked > 5_000, "{checked} fields");
}

/// Checks every expression of a vector file; returns how many ran.
fn check_expressions(text: &'static str) -> usize {
    let mut current: Option<CronSchedule> = None;
    let mut expression = "";
    let mut accepted = 0;
    for line in data_lines(text) {
        let Some(run) = line.strip_prefix('\t') else {
            let (text, verdict) = line.rsplit_once('\t').unwrap();
            expression = text;
            current = CronSchedule::parse(expression).ok();
            assert_eq!(current.is_some(), verdict == "ok", "`{expression}`");
            accepted += usize::from(current.is_some());
            continue;
        };
        let schedule = current.as_ref().unwrap();
        let (start, results) = run.split_once('\t').unwrap();
        let (seconds, nanos) = start.split_once('.').unwrap_or((start, "0"));
        let mut cursor = Timestamp::new(seconds.parse().unwrap(), nanos.parse().unwrap()).unwrap();
        for expected in results.split(' ') {
            let actual = schedule.next_after(&cursor);
            if expected == "end" {
                assert_eq!(actual, None, "`{expression}` after {cursor}");
                break;
            }
            let expected = Timestamp::from_second(expected.parse().unwrap()).unwrap();
            assert_eq!(actual, Some(expected), "`{expression}` after {cursor}");
            cursor = expected;
        }
    }
    accepted
}

#[test]
fn expressions_run_when_the_cron_crate_ran_them() {
    let accepted = check_expressions(include_str!("scheduler_cron_golden/expressions.txt"));
    assert!(accepted > 768, "{accepted} accepted expressions");
}

#[test]
fn documented_and_calendar_edge_expressions_agree_with_the_cron_crate() {
    let accepted = check_expressions(include_str!("scheduler_cron_golden/corpus.txt"));
    assert_eq!(accepted, 37);
}

#[test]
fn the_cron_crate_joined_fields_that_now_must_stand_alone() {
    // The crate read minutes 1 and 2 of every hour, from January to July
    // only: the weekday list had become the month field.
    assert!(CronSchedule::parse("1, 2 * * *").is_err());
}
