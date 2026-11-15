//! Differential tests against the `cron` crate, which evaluated cron fields
//! until 13.0.0-alpha.5. [`legacy`] is that implementation, unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::CronSchedule;
use super::field::{DAY_OF_MONTH, HOUR, MINUTE, MONTH, Unit};
use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use cron::TimeUnitSpec;
use proptest::prelude::*;

/// The scheduler's cron evaluation as it was in 13.0.0-alpha.5.
#[path = "scheduler_cron_legacy_tests.rs"]
mod legacy;

/// A field position, for the `cron` crate's view of one field.
#[derive(Clone, Copy, Debug)]
enum Slot {
    Minute,
    Hour,
    DayOfMonth,
    Month,
}

impl Slot {
    const ALL: [Self; 4] = [Self::Minute, Self::Hour, Self::DayOfMonth, Self::Month];

    fn unit(self) -> &'static Unit {
        match self {
            Self::Minute => &MINUTE,
            Self::Hour => &HOUR,
            Self::DayOfMonth => &DAY_OF_MONTH,
            Self::Month => &MONTH,
        }
    }

    fn bounds(self) -> (u32, u32) {
        match self {
            Self::Minute => (0, 59),
            Self::Hour => (0, 23),
            Self::DayOfMonth => (1, 31),
            Self::Month => (1, 12),
        }
    }

    /// The values the `cron` crate gives `field` alone, with `*` in every
    /// other field (which never joins a neighbour), or `None` if it rejects it.
    /// An empty field never reaches it: the expression is split on whitespace.
    fn crate_values(self, field: &str) -> Option<Vec<u32>> {
        if field.is_empty() {
            return None;
        }
        let expression = match self {
            Self::Minute => format!("* {field} * * * * *"),
            Self::Hour => format!("* * {field} * * * *"),
            Self::DayOfMonth => format!("* * * {field} * * *"),
            Self::Month => format!("* * * * {field} * *"),
        };
        let schedule = expression.parse::<cron::Schedule>().ok()?;
        Some(match self {
            Self::Minute => schedule.minutes().iter().collect(),
            Self::Hour => schedule.hours().iter().collect(),
            Self::DayOfMonth => schedule.days_of_month().iter().collect(),
            Self::Month => schedule.months().iter().collect(),
        })
    }

    /// The values Rullst gives `field`, or `None` if it rejects it.
    fn values(self, field: &str) -> Option<Vec<u32>> {
        let values = self.unit().parse(field).ok()?;
        let (min, max) = self.bounds();
        Some(
            (min..=max)
                .filter(|value| values.contains(*value))
                .collect(),
        )
    }
}

fn number(max: u32) -> impl Strategy<Value = String> {
    prop_oneof![
        6 => (0..=max + 2).prop_map(|value| value.to_string()),
        1 => (0..=max).prop_map(|value| format!("0{value}")),
        1 => prop::sample::select(vec!["4294967296", "+5", "-1", ""]).prop_map(String::from),
    ]
}

fn name() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "jan", "JAN", "January", "feb", "Mar", "apr", "may", "jun", "june", "jul", "aug", "sep",
        "sept", "oct", "nov", "dec", "December", "mon", "fri", "L", "W", "x",
    ])
    .prop_map(String::from)
}

fn atom(max: u32) -> impl Strategy<Value = String> {
    prop_oneof![
        2 => Just("*".to_string()),
        1 => Just("?".to_string()),
        5 => number(max),
        2 => name(),
    ]
}

/// A field shaped like cron syntax, valid or not, or a short noisy string.
fn field(max: u32) -> impl Strategy<Value = String> {
    let base = prop_oneof![
        3 => atom(max),
        2 => (atom(max), atom(max)).prop_map(|(start, end)| format!("{start}-{end}")),
    ];
    let element = (base, prop::option::of(number(max))).prop_map(|(base, step)| match step {
        Some(step) => format!("{base}/{step}"),
        None => base,
    });
    prop_oneof![
        2 => Just("*".to_string()),
        6 => prop::collection::vec(element, 1..=3).prop_map(|elements| elements.join(",")),
        1 => "[0-9a-zA-Z*?,/@#-]{1,6}",
    ]
}

fn weekday_field() -> impl Strategy<Value = String> {
    let day = prop::sample::select(vec![
        "0", "1", "2", "3", "5", "6", "7", "8", "sun", "MON", "tues", "fri", "Saturday", "x",
    ]);
    let element = prop_oneof![
        3 => day.clone().prop_map(String::from),
        2 => (day.clone(), day).prop_map(|(start, end)| format!("{start}-{end}")),
        1 => (0_u32..=8).prop_map(|step| format!("*/{step}")),
        1 => (1_u32..=8).prop_map(|step| format!("1-5/{step}")),
    ];
    prop_oneof![
        3 => Just("*".to_string()),
        1 => Just("?".to_string()),
        4 => prop::collection::vec(element, 1..=2).prop_map(|elements| elements.join(",")),
    ]
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/// A valid element of a field from `min` to `max`: `*`, `?` (day of month),
/// numbers or month names, ordered ranges and steps.
fn valid_element(min: u32, max: u32, month: bool, question: bool) -> impl Strategy<Value = String> {
    let star = if question {
        prop::sample::select(vec!["*", "?"]).boxed()
    } else {
        Just("*").boxed()
    };
    let bounds = (min..=max).prop_flat_map(move |start| (Just(start), start..=max));
    let named = if month {
        any::<bool>().boxed()
    } else {
        Just(false).boxed()
    };
    (star, bounds, 1..=max, named, 0..6_u8).prop_map(
        move |(star, (start, end), step, named, kind)| {
            let label = |value: u32| match value
                .checked_sub(1)
                .and_then(|index| MONTHS.get(index as usize))
            {
                Some(name) if named => (*name).to_string(),
                _ => value.to_string(),
            };
            match kind {
                0 => star.to_string(),
                1 => format!("{star}/{step}"),
                2 => label(start),
                3 => format!("{}-{}", label(start), label(end)),
                4 => format!("{}-{}/{step}", label(start), label(end)),
                _ => format!("{start}/{step}"),
            }
        },
    )
}

fn valid_field(min: u32, max: u32, month: bool, question: bool) -> impl Strategy<Value = String> {
    prop::collection::vec(valid_element(min, max, month, question), 1..=3)
        .prop_map(|elements| elements.join(","))
}

/// A valid POSIX day-of-week field.
fn valid_weekday_field() -> impl Strategy<Value = String> {
    let label = |day: u32, named: bool| match DAYS.get(day as usize) {
        Some(name) if named => (*name).to_string(),
        _ => day.to_string(),
    };
    let range = (0..=7_u32)
        .prop_flat_map(|start| (Just(start), prop_oneof![start..=7, Just(0)]))
        .prop_filter("ordered", |(start, end)| start <= end || *end == 0);
    let element = (range, 1..=7_u32, any::<bool>(), 0..3_u8).prop_map(
        move |((start, end), step, named, kind)| match kind {
            0 => label(start, named),
            1 => format!("{}-{}", label(start, named), label(end, named)),
            _ => format!("{}-{}/{step}", label(start, named), label(end, named)),
        },
    );
    prop_oneof![
        2 => prop::sample::select(vec!["*", "?"]).prop_map(String::from),
        1 => (prop::sample::select(vec!["*", "?"]), 1..=7_u32)
            .prop_map(|(star, step)| format!("{star}/{step}")),
        4 => prop::collection::vec(element, 1..=2).prop_map(|elements| elements.join(",")),
    ]
}

/// Instants from 1900 to 2101: whole minutes, or any nanosecond.
fn instant() -> impl Strategy<Value = DateTime<Utc>> {
    let seconds = -2_208_988_800_i64..4_133_980_800_i64;
    let whole_minutes = seconds
        .clone()
        .prop_map(|seconds| seconds - seconds.rem_euclid(60));
    (
        prop_oneof![whole_minutes, seconds],
        prop_oneof![Just(0_u32), 0_u32..1_000_000_000],
    )
        .prop_map(|(seconds, nanos)| DateTime::from_timestamp(seconds, nanos).unwrap())
}

/// Both evaluators agree on `expression` from `start` for `steps` occurrences.
fn assert_same_occurrences(expression: &str, start: DateTime<Utc>, steps: usize) {
    let ours = CronSchedule::parse(expression);
    let theirs = legacy::CronSchedule::parse(expression);
    let (Ok(ours), Ok(theirs)) = (ours, theirs) else {
        panic!("`{expression}` must parse in both evaluators");
    };
    let mut cursor = start;
    for _ in 0..steps {
        let expected = theirs.next_after(&cursor);
        assert_eq!(
            ours.next_after(&cursor),
            expected,
            "`{expression}` after {cursor}"
        );
        let Some(next) = expected else { break };
        cursor = next;
    }
}

/// `cases` cases, without regression files and with bounded shrinking, so a
/// failure (or a mutant that scans every day to 2100) still ends quickly.
fn config(cases: u32) -> ProptestConfig {
    ProptestConfig {
        cases,
        failure_persistence: None,
        max_shrink_iters: 256,
        max_shrink_time: 20_000,
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(config(2_048))]

    #[test]
    fn each_field_parses_as_the_cron_crate_parsed_it(
        minute in field(59),
        hour in field(23),
        day_of_month in field(31),
        month in field(12),
    ) {
        for (slot, text) in Slot::ALL.into_iter().zip([&minute, &hour, &day_of_month, &month]) {
            prop_assert_eq!(slot.values(text), slot.crate_values(text), "{:?} `{}`", slot, text);
        }
    }
}

proptest! {
    #![proptest_config(config(768))]

    #[test]
    fn expressions_run_when_the_cron_crate_ran_them(
        minute in field(59),
        hour in field(23),
        day_of_month in field(31),
        month in field(12),
        day_of_week in weekday_field(),
        starts in prop::collection::vec(instant(), 3),
    ) {
        let expression = format!("{minute} {hour} {day_of_month} {month} {day_of_week}");
        let ours = CronSchedule::parse(&expression);
        let theirs = legacy::CronSchedule::parse(&expression);
        // Rullst accepts exactly the expressions whose fields the `cron`
        // crate accepted one by one; it also took some whose fields only
        // parsed joined with a neighbour (`1, 2 3 4 5`).
        let fields_valid = Slot::ALL
            .into_iter()
            .zip([&minute, &hour, &day_of_month, &month])
            .all(|(slot, text)| slot.crate_values(text).is_some());
        let weekdays_valid = legacy::crate_day_of_week(&day_of_week).is_ok();
        prop_assert_eq!(ours.is_ok(), fields_valid && weekdays_valid, "`{}`", expression);
        if ours.is_ok() {
            prop_assert!(theirs.is_ok(), "`{}`", expression);
            for start in starts {
                assert_same_occurrences(&expression, start, 4);
            }
        }
    }
}

proptest! {
    #![proptest_config(config(768))]

    #[test]
    fn valid_expressions_occur_when_the_cron_crate_said(
        minute in valid_field(0, 59, false, false),
        hour in valid_field(0, 23, false, false),
        day_of_month in valid_field(1, 31, false, true),
        month in valid_field(1, 12, true, false),
        day_of_week in valid_weekday_field(),
        starts in prop::collection::vec(instant(), 3),
    ) {
        let expression = format!("{minute} {hour} {day_of_month} {month} {day_of_week}");
        for start in starts {
            assert_same_occurrences(&expression, start, 4);
        }
    }
}

#[test]
fn the_cron_crate_joined_fields_that_now_must_stand_alone() {
    let expression = "1, 2 * * *";
    assert!(CronSchedule::parse(expression).is_err());
    // The crate read minutes 1 and 2 of every hour, from January to July
    // only: the weekday list had become the month field.
    let theirs = legacy::CronSchedule::parse(expression).unwrap();
    let august = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
    let january = Utc.with_ymd_and_hms(2027, 1, 1, 0, 1, 0).unwrap();
    assert_eq!(theirs.next_after(&august), Some(january));
    assert_eq!(
        theirs.next_after(&january),
        Some(january + TimeDelta::minutes(1))
    );
}

/// Expressions from Rullst's documentation, generators and tests, and edge
/// cases of the calendar.
const CORPUS: &[&str] = &[
    "* * * * *",
    "0 * * * *",
    "*/5 * * * *",
    "*/10 * * * *",
    "0 0 * * *",
    "0 7 * * *",
    "0 18 * * *",
    "0 9 * * 1-5",
    "0 3 * * 0",
    "0 3 * * 7",
    "30 23 * * *",
    "0 0 1 * 1",
    "0 0 */2 * 1",
    "0 0 13 * 5",
    "0 0 1 * *",
    "0 0 1 1 *",
    "59 23 31 12 *",
    "0 0 29 2 *",
    "0 0 28,29 2 *",
    "0 0 30 2 *",
    "0 0 31 2 *",
    "0 0 30 2 1",
    "0 0 31 4,6,9,11 *",
    "0 0 31 * *",
    "0 12 30,31 * *",
    "*/59 */23 */31 */12 */7",
    "5/15 9-17/4 ?/10 jan-dec/5 mon-fri",
    "0 0 ? * sat,sun",
    "0 0 1-7 * 1",
    "0 0 1-7 * */1",
    "15,45 6,18 1,15 * *",
    "0 0 * feb,FEBRUARY *",
    "0 0 1 */3 *",
    "0 12 * * 5-7",
    "0 12 * * FRI-SUN",
    "0 0 1 1 0",
    "0 0 29 2 0",
];

#[test]
fn documented_and_calendar_edge_expressions_agree_with_the_cron_crate() {
    let mut starts = vec![
        Utc.with_ymd_and_hms(2026, 11, 14, 12, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2024, 2, 28, 23, 59, 0).unwrap(),
        Utc.with_ymd_and_hms(2024, 2, 29, 0, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2025, 2, 28, 23, 59, 59).unwrap(),
        Utc.with_ymd_and_hms(2026, 1, 31, 23, 59, 0).unwrap(),
        Utc.with_ymd_and_hms(2026, 4, 30, 23, 59, 0).unwrap(),
        Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 0).unwrap(),
        Utc.with_ymd_and_hms(1999, 12, 31, 23, 59, 59).unwrap(),
        Utc.with_ymd_and_hms(2099, 12, 31, 23, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2100, 12, 31, 23, 58, 30).unwrap(),
        Utc.with_ymd_and_hms(1969, 12, 31, 23, 59, 30).unwrap(),
        Utc.with_ymd_and_hms(1950, 6, 1, 0, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(0, 1, 1, 0, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(-1, 12, 31, 23, 59, 58).unwrap(),
    ];
    starts.push(starts[0] + TimeDelta::nanoseconds(1));
    starts.push(starts[0] - TimeDelta::nanoseconds(1));
    for expression in CORPUS {
        for start in &starts {
            assert_same_occurrences(expression, *start, 6);
        }
    }
}
