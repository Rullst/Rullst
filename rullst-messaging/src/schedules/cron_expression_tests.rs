#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// Occurrences of `expression` after `after_ms`, as `RecurringDefinition`
/// computes them.
fn next(expression: &str, after_ms: i64) -> Option<i64> {
    Schedule::parse_five_fields(expression)
        .unwrap()
        .next_after(after_ms)
        .unwrap()
}

fn at(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> i64 {
    (days_from_civil(year, month, day) * SECONDS_PER_DAY + hour * 3_600 + minute * 60 + second)
        * MS_PER_SECOND
}

#[test]
fn civil_days_round_trip_and_match_known_dates() {
    assert_eq!(days_from_civil(1970, 1, 1), 0);
    assert_eq!(days_from_civil(2000, 3, 1), 11_017);
    assert_eq!(days_from_civil(2100, 12, 31), 47_846);
    assert_eq!(days_from_civil(-1, 12, 31), -719_529);
    for days in [-800_000, -719_529, -1, 0, 59, 60, 11_016, 11_017, 47_846, 3_000_000] {
        let (year, month, day) = civil_from_days(days);
        assert_eq!(days_from_civil(year, month, day), days, "{days}");
    }
    assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    assert_eq!(civil_from_days(-1), (1969, 12, 31));
}

#[test]
fn fields_follow_the_crate_semantics() {
    let start = at(2026, 11, 17, 10, 14, 30);
    assert_eq!(next("0 6 * * *", start), Some(at(2026, 11, 18, 6, 0, 0)));
    // 1 is Sunday and both day fields must match: Sunday the 1st.
    assert_eq!(next("0 0 1 * 1", start), Some(at(2027, 8, 1, 0, 0, 0)));
    assert_eq!(next("0 0 * * sat", start), Some(at(2026, 11, 21, 0, 0, 0)));
    assert_eq!(next("0 0 29 feb ?", start), Some(at(2028, 2, 29, 0, 0, 0)));
    assert_eq!(next("0 0 31 * *", start), Some(at(2026, 12, 31, 0, 0, 0)));
    assert_eq!(next("5/20 * * * *", start), Some(at(2026, 11, 17, 10, 25, 0)));
    assert_eq!(next("0 0 30 2 *", start), None);
    assert!(Schedule::parse_five_fields("0 0 * * 0").is_none());
    assert!(Schedule::parse_five_fields("0 0 * * 8").is_none());
    assert!(Schedule::parse_five_fields("0 0 * * mon/2").is_none());
    assert!(Schedule::parse_five_fields("0 0 * 1-jan *").is_none());
    assert!(Schedule::parse_five_fields("0 0 5-1 * *").is_none());
    assert!(Schedule::parse_five_fields("*/0 * * * *").is_none());
    assert!(Schedule::parse_five_fields("*/60 * * * *").is_none());
    assert!(Schedule::parse_five_fields("* ? * * *").is_none());
}

#[test]
fn tokens_join_and_shift_as_the_crate_parsed_them() {
    let start = at(2026, 11, 17, 10, 14, 30);
    // A trailing comma or a leading dash joins the next token.
    assert_eq!(next("1, 2 * * *", start), Some(at(2026, 11, 17, 11, 1, 0)));
    assert_eq!(next("1 -2 * * *", start), Some(at(2026, 11, 17, 11, 1, 0)));
    // A leading slash steps the fixed seconds field.
    assert_eq!(next("/20 * * * *", start), Some(at(2026, 11, 17, 10, 14, 40)));
    // `1jan` splits into a day of the month and a month, one field too many
    // unless a comma joins two tokens.
    assert_eq!(next("0 0, 1 1jan *", start), Some(at(2027, 1, 1, 0, 0, 0)));
    assert!(Schedule::parse_five_fields("0 0 1jan * *").is_none());
    assert!(Schedule::parse_five_fields("1, x * * *").is_none());
}

#[test]
fn search_runs_from_1970_through_2100() {
    assert_eq!(next("0 0 1 1 *", -1), Some(at(1970, 1, 1, 0, 0, 0)));
    assert_eq!(next("0 0 1 1 *", at(1950, 6, 1, 0, 0, 0)), Some(0));
    assert_eq!(next("0 0 1 1 *", at(-1, 12, 31, 23, 59, 58)), None);
    assert_eq!(next("59 23 31 12 *", at(2100, 12, 31, 23, 58, 59)), Some(at(2100, 12, 31, 23, 59, 0)));
    assert_eq!(next("* * * * *", at(2100, 12, 31, 23, 59, 0)), None);
    // Strictly after, at whole seconds.
    assert_eq!(next("* * * * *", at(2026, 1, 1, 0, 0, 0)), Some(at(2026, 1, 1, 0, 1, 0)));
    assert_eq!(next("* * * * *", at(2026, 1, 1, 0, 0, 59) + 999), Some(at(2026, 1, 1, 0, 1, 0)));
    let schedule = Schedule::parse_five_fields("* * * * *").unwrap();
    assert_eq!(schedule.next_after(MIN_MILLIS - 1), Err(OutOfRange::Instant));
    assert_eq!(schedule.next_after(MAX_MILLIS + 1), Err(OutOfRange::Instant));
    assert_eq!(schedule.next_after(MAX_MILLIS), Ok(None));
    assert_eq!(schedule.next_after(MIN_MILLIS), Ok(None));
}

/// One expression of the vectors and what the crate computed for it.
struct Vector<'a> {
    expression: &'a str,
    /// `None` when the crate rejected the expression.
    runs: Option<Vec<(i64, Vec<&'a str>)>>,
}

fn vectors() -> Vec<Vector<'static>> {
    let mut vectors: Vec<Vector<'static>> = Vec::new();
    for line in include_str!("cron_expression_vectors.txt").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(run) = line.strip_prefix('\t') {
            let (after, results) = run.split_once('\t').unwrap();
            let current = vectors.last_mut().unwrap();
            current
                .runs
                .as_mut()
                .unwrap()
                .push((after.parse().unwrap(), results.split(' ').collect()));
            continue;
        }
        let (expression, verdict) = line.rsplit_once('\t').unwrap();
        vectors.push(Vector {
            expression,
            runs: (verdict == "ok").then(Vec::new),
        });
    }
    vectors
}

/// The golden vectors that cron 0.17.0 produced; see the file's header.
#[test]
fn expressions_parse_and_occur_as_the_cron_crate_computed() {
    let vectors = vectors();
    assert!(vectors.len() > 1_000);
    for vector in &vectors {
        let schedule = Schedule::parse_five_fields(vector.expression);
        let Some(runs) = &vector.runs else {
            assert!(schedule.is_none(), "`{}` must be rejected", vector.expression);
            continue;
        };
        let schedule = schedule.unwrap_or_else(|| panic!("`{}` must parse", vector.expression));
        for (after, results) in runs {
            let mut cursor = *after;
            for expected in results {
                let actual = schedule.next_after(cursor);
                let label = format!("`{}` after {cursor}", vector.expression);
                match *expected {
                    "error" => assert_eq!(actual, Err(OutOfRange::Instant), "{label}"),
                    "end" => assert_eq!(actual, Ok(None), "{label}"),
                    value => {
                        let value: i64 = value.parse().unwrap();
                        assert_eq!(actual, Ok(Some(value)), "{label}");
                        cursor = value;
                    }
                }
            }
        }
    }
}

#[path = "cron_expression_differential_tests.rs"]
mod differential;
