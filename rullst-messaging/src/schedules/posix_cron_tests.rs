//! POSIX schedules against Core's own evaluator (`Scheduler::task`), which
//! they copy: random field-shaped expressions, valid or not, and valid ones,
//! from random instants, through Core's `test-util` hook.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::PosixSchedule;
use proptest::prelude::*;

/// Core's verdict and next occurrences for `expression` after `after_ms`.
fn core(expression: &str, after_ms: i64, steps: usize) -> Result<Vec<Option<i64>>, String> {
    let mut cursor = after_ms;
    let mut occurrences = Vec::new();
    for _ in 0..steps {
        let next = rullst_core::scheduler::__test_hooks::cron_next_after(expression, cursor)?;
        occurrences.push(next);
        let Some(next) = next else { break };
        cursor = next;
    }
    Ok(occurrences)
}

fn ours(expression: &str, after_ms: i64, steps: usize) -> Result<Vec<Option<i64>>, String> {
    let schedule = PosixSchedule::parse(expression)?;
    let mut cursor = after_ms;
    let mut occurrences = Vec::new();
    for _ in 0..steps {
        let next = schedule.next_after(cursor);
        occurrences.push(next);
        let Some(next) = next else { break };
        cursor = next;
    }
    Ok(occurrences)
}

/// Both evaluators accept or reject `expression` alike and agree on its
/// next four occurrences after each start.
fn assert_same(expression: &str, starts: &[i64]) {
    for &start in starts {
        let (ours, core) = (ours(expression, start, 4), core(expression, start, 4));
        assert_eq!(
            ours.is_ok(),
            core.is_ok(),
            "`{expression}`: {ours:?} {core:?}"
        );
        if let (Ok(ours), Ok(core)) = (ours, core) {
            assert_eq!(ours, core, "`{expression}` after {start}");
        }
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

/// Unix milliseconds from 1900 to 2101: whole minutes, or any millisecond.
fn instant() -> impl Strategy<Value = i64> {
    let millis = -2_208_988_800_000_i64..4_133_980_800_000_i64;
    prop_oneof![
        millis
            .clone()
            .prop_map(|millis| millis - millis.rem_euclid(60_000)),
        millis,
    ]
}

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
    #![proptest_config(config(768))]

    #[test]
    fn expressions_run_as_core_runs_them(
        minute in field(59),
        hour in field(23),
        day_of_month in field(31),
        month in field(12),
        day_of_week in weekday_field(),
        starts in prop::collection::vec(instant(), 3),
    ) {
        assert_same(&format!("{minute} {hour} {day_of_month} {month} {day_of_week}"), &starts);
    }

    #[test]
    fn valid_expressions_occur_when_core_says(
        minute in valid_field(0, 59, false, false),
        hour in valid_field(0, 23, false, false),
        day_of_month in valid_field(1, 31, false, true),
        month in valid_field(1, 12, true, false),
        day_of_week in valid_weekday_field(),
        starts in prop::collection::vec(instant(), 3),
    ) {
        let expression = format!("{minute} {hour} {day_of_month} {month} {day_of_week}");
        prop_assert!(PosixSchedule::parse(&expression).is_ok(), "`{}`", expression);
        assert_same(&expression, &starts);
    }
}

#[test]
fn documented_and_calendar_edge_expressions_agree_with_core() {
    let starts = [
        1_794_910_470_000,
        1_709_164_740_000,
        1_709_164_800_000,
        1_740_787_199_000,
        1_769_903_940_000,
        1_777_593_540_000,
        1_798_761_540_000,
        946_684_799_000,
        4_102_441_200_000,
        4_133_980_710_000,
        -30_000,
        -617_760_000_000,
        -62_167_219_200_000,
        -62_167_219_202_000,
        1_794_910_470_001,
        1_794_910_469_999,
    ];
    for expression in [
        "* * * * *",
        "0 * * * *",
        "*/5 * * * *",
        "0 0 * * *",
        "0 6 * * *",
        "0 9 * * 1-5",
        "0 3 * * 0",
        "0 3 * * 7",
        "30 23 * * *",
        "0 0 1 * 1",
        "0 0 */2 * 1",
        "0 0 13 * 5",
        "0 0 1 1 *",
        "59 23 31 12 *",
        "0 0 29 2 *",
        "0 0 30 2 *",
        "0 0 30 2 1",
        "0 0 31 4,6,9,11 *",
        "*/59 */23 */31 */12 */7",
        "5/15 9-17/4 ?/10 jan-dec/5 mon-fri",
        "0 0 ? * sat,sun",
        "0 12 * * FRI-SUN",
        "0 0 1 1 0",
        "1, 2 * * *",
        "0 0 * * 8",
    ] {
        assert_same(expression, &starts);
    }
}

#[test]
fn the_documented_rules_hold() {
    let schedule = PosixSchedule::parse("0 0 1 * 1").unwrap();
    // 2026-11-17T10:14:30Z, a Tuesday: next is Monday the 23rd; after Monday the
    // 30th comes Tuesday 1 December.
    assert_eq!(
        schedule.next_after(1_794_910_470_000),
        Some(1_795_392_000_000)
    );
    assert_eq!(
        schedule.next_after(1_795_996_800_000),
        Some(1_796_083_200_000)
    );
    assert!(PosixSchedule::parse("1, 2 * * *").is_err());
    assert!(PosixSchedule::parse("0 0 * * 1/2").is_err());
    assert_eq!(
        PosixSchedule::parse("0 0 31 2 *").unwrap().next_after(0),
        None
    );
}
