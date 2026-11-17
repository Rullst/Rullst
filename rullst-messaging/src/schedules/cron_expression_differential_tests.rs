//! Differential tests against cron 0.17.0, which evaluated recurring
//! schedules until 13.0.0-alpha.5, and the generator of the golden vectors.

use super::super::{MAX_MILLIS, MIN_MILLIS, Schedule};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use std::fmt::Write as _;

/// `RecurringDefinition::next_after` as it was in 13.0.0-alpha.5, without the
/// five-token check that both sides share.
fn crate_next(expression: &str, after: i64) -> Result<Option<i64>, &'static str> {
    let schedule: cron::Schedule = format!("0 {expression} *")
        .parse()
        .map_err(|_| "five-field UTC cron")?;
    let time = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(after)
        .ok_or("cron timestamp")?;
    Ok(schedule
        .after(&time)
        .next()
        .map(|next| next.timestamp_millis()))
}

fn crate_parses(expression: &str) -> bool {
    format!("0 {expression} *").parse::<cron::Schedule>().is_ok()
}

fn ours_next(schedule: &Schedule, after: i64) -> Result<Option<i64>, &'static str> {
    schedule.next_after(after).map_err(|_| "cron timestamp")
}

#[test]
fn chrono_millisecond_bounds_are_the_constants() {
    assert_eq!(chrono::DateTime::<chrono::Utc>::MIN_UTC.timestamp_millis(), MIN_MILLIS);
    assert_eq!(chrono::DateTime::<chrono::Utc>::MAX_UTC.timestamp_millis(), MAX_MILLIS);
    assert!(chrono::DateTime::<chrono::Utc>::from_timestamp_millis(MIN_MILLIS - 1).is_none());
    assert!(chrono::DateTime::<chrono::Utc>::from_timestamp_millis(MAX_MILLIS + 1).is_none());
}

const NAMES: &[&str] = &[
    "jan", "JAN", "January", "feb", "Mar", "apr", "may", "jun", "june", "jul", "aug", "sep",
    "sept", "oct", "nov", "dec", "December", "sun", "MON", "tue", "tues", "Wed", "thu", "thurs",
    "fri", "sat", "Saturday", "L", "W", "x",
];

fn atom() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => Just("*".to_string()),
        2 => Just("?".to_string()),
        8 => (0_u32..=62).prop_map(|value| value.to_string()),
        1 => prop::sample::select(vec![
            "00", "07", "1969", "1970", "2030", "2100", "2101", "4294967295", "4294967296",
        ])
        .prop_map(String::from),
        4 => prop::sample::select(NAMES.to_vec()).prop_map(String::from),
    ]
}

fn element() -> impl Strategy<Value = String> {
    (
        atom(),
        prop::option::weighted(0.35, atom()),
        prop::option::weighted(0.3, prop_oneof![0_u32..=8, 0_u32..=61]),
    )
        .prop_map(|(start, end, step)| {
            let mut element = start;
            if let Some(end) = end {
                element = format!("{element}-{end}");
            }
            if let Some(step) = step {
                let _ = write!(element, "/{step}");
            }
            element
        })
}

/// A field-shaped token, often with a piece that makes the crate join it to
/// a neighbour or split it in two.
fn token() -> impl Strategy<Value = String> {
    let list = || prop::collection::vec(element(), 1..=3).prop_map(|elements| elements.join(","));
    let affix = prop::sample::select(vec![
        ",", "-", "/", "x", "jan", "5", "*", "?", ",5", "-5", "/5", "5,", "5-", "5/",
    ]);
    prop_oneof![
        8 => list(),
        2 => (affix.clone(), list()).prop_map(|(affix, list)| format!("{affix}{list}")),
        2 => (list(), affix.clone()).prop_map(|(list, affix)| format!("{list}{affix}")),
        1 => affix.prop_map(String::from),
        1 => "[0-9a-zA-Z*?,/@#-]{1,8}",
    ]
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/// A valid element of a field from `min` to `max`, with `names` (the first
/// for `min`) and `?` when `question`.
fn valid_element(
    min: u32,
    max: u32,
    names: &'static [&'static str],
    question: bool,
) -> impl Strategy<Value = String> {
    let star = if question {
        prop::sample::select(vec!["*", "?"]).boxed()
    } else {
        Just("*").boxed()
    };
    let bounds = (min..=max).prop_flat_map(move |start| (Just(start), start..=max));
    (star, bounds, 1..=max, any::<bool>(), 0..6_u8).prop_map(
        move |(star, (start, end), step, named, kind)| {
            let label = |value: u32| match names.get((value - min) as usize) {
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

fn valid_field(
    min: u32,
    max: u32,
    names: &'static [&'static str],
    question: bool,
) -> impl Strategy<Value = String> {
    prop::collection::vec(valid_element(min, max, names, question), 1..=3)
        .prop_map(|elements| elements.join(","))
}

/// Each slot valid three times in four, otherwise a noisy token.
fn expression() -> impl Strategy<Value = String> {
    let slot = |valid: BoxedStrategy<String>| prop_oneof![3 => valid, 1 => token()];
    (
        slot(valid_field(0, 59, &[], false).boxed()),
        slot(valid_field(0, 23, &[], false).boxed()),
        slot(valid_field(1, 31, &[], true).boxed()),
        slot(valid_field(1, 12, &MONTHS, false).boxed()),
        slot(valid_field(1, 7, &DAYS, true).boxed()),
    )
        .prop_map(|(minute, hour, day, month, weekday)| {
            format!("{minute} {hour} {day} {month} {weekday}")
        })
}

/// The last instant of each month from 1969 through 2101.
fn month_end() -> impl Strategy<Value = i64> {
    (1969_i64..=2101, 1_i64..=12).prop_map(|(year, month)| {
        let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
        super::super::days_from_civil(next_year, next_month, 1) * 86_400_000 - 1
    })
}

/// Unix milliseconds that the callers pass (0 to the end of 2101), month and
/// year ends, whole seconds and minutes, and the edges of the crate's range.
fn instant() -> impl Strategy<Value = i64> {
    let ordinary = 0_i64..4_165_516_800_000;
    prop_oneof![
        6 => ordinary.clone(),
        2 => ordinary.clone().prop_map(|value| value - value.rem_euclid(1_000)),
        2 => ordinary.prop_map(|value| value - value.rem_euclid(60_000)),
        3 => (month_end(), prop_oneof![Just(0_i64), Just(-999), Just(-59_999), Just(1)])
            .prop_map(|(end, shift)| end + shift),
        1 => prop::sample::select(vec![
            -1_i64,
            -86_400_000,
            -62_167_219_200_001,
            -62_167_219_200_000,
            -62_198_755_200_000,
            MIN_MILLIS,
            MIN_MILLIS - 1,
            MAX_MILLIS - 2_000,
            MAX_MILLIS + 1,
            253_402_300_799_000,
            4_133_980_799_999,
            4_133_980_740_000,
        ]),
    ]
}

/// Compares both evaluators on `expression` from each start for `steps`
/// occurrences, returning the crate's results.
fn compare(expression: &str, starts: &[i64], steps: usize) -> Option<Vec<(i64, Vec<String>)>> {
    let ours = Schedule::parse_five_fields(expression);
    assert_eq!(ours.is_some(), crate_parses(expression), "`{expression}`");
    let ours = ours?;
    let mut runs = Vec::new();
    for &start in starts {
        let mut cursor = start;
        let mut results = Vec::new();
        for _ in 0..steps {
            let expected = crate_next(expression, cursor);
            assert_eq!(ours_next(&ours, cursor), expected, "`{expression}` after {cursor}");
            match expected {
                Ok(Some(next)) => {
                    results.push(next.to_string());
                    cursor = next;
                }
                Ok(None) => {
                    results.push("end".to_string());
                    break;
                }
                Err(_) => {
                    results.push("error".to_string());
                    break;
                }
            }
        }
        runs.push((start, results));
    }
    Some(runs)
}

fn config(cases: u32) -> ProptestConfig {
    ProptestConfig {
        cases,
        failure_persistence: None,
        max_shrink_iters: 512,
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(config(
        std::env::var("CRON_DIFFERENTIAL_CASES").ok().and_then(|cases| cases.parse().ok()).unwrap_or(4_096)
    ))]

    #[test]
    fn expressions_parse_and_occur_as_the_cron_crate_computed_them(
        expression in expression(),
        starts in prop::collection::vec(instant(), 3),
    ) {
        compare(&expression, &starts, 4);
    }
}

/// Expressions from Rullst's documentation and tests, the quirks of the
/// crate's grammar, and edge cases of the calendar.
const CORPUS: &[&str] = &[
    "0 6 * * *",
    "* * * * *",
    "*/5 * * * *",
    "0 * * * *",
    "0 0 * * *",
    "30 9 * * 2-6",
    "0 9 * * mon-fri",
    "0 0 1 * *",
    "0 0 1 1 *",
    "59 23 31 12 *",
    "0 0 29 2 *",
    "0 0 29 feb ?",
    "0 0 30 2 *",
    "0 0 31 * *",
    "0 0 31 4,6,9,11 *",
    "0 0 1 * 1",
    "0 0 13 * 6",
    "0 12 * * 1,7",
    "0 12 ? * sat,sun",
    "15,45 6,18 1,15 * *",
    "5/15 9-17/4 ?/10 jan-dec/5 sun-sat",
    "*/59 */23 */31 */12 */7",
    "0 0 1-7 * 2",
    "1, 2 * * *",
    "1 -2 * * *",
    "1 ,2 * * *",
    "/5 * * * *",
    "/20 0 12 * *",
    "-5 * * * *",
    ",30 * * * *",
    "0 0, 1 1jan *",
    "0 0 1jan * *",
    "0 0 * * 0",
    "0 0 * * 8",
    "0 0 * * mon/2",
    "* ? * * *",
];

#[test]
#[ignore = "writes cron_expression_vectors.txt from cron 0.17.0"]
fn generate_golden_vectors() {
    let mut out = String::from("# Golden vectors of cron 0.17.0 (header written by hand)\n");
    let mut seen = std::collections::BTreeSet::new();
    let edges = [
        0_i64,
        1_763_374_440_000,
        1_709_164_799_999,
        1_709_164_800_000,
        1_740_787_199_999,
        1_767_225_599_000,
        4_102_444_799_999,
        4_133_980_740_000,
        4_133_980_799_999,
        -1,
        MIN_MILLIS - 1,
        MAX_MILLIS - 2_000,
    ];
    let mut write = |expression: &str, runs: Option<Vec<(i64, Vec<String>)>>| {
        if !seen.insert(expression.to_string()) {
            return;
        }
        match runs {
            None => {
                let _ = writeln!(out, "{expression}\treject");
            }
            Some(runs) => {
                let _ = writeln!(out, "{expression}\tok");
                for (start, results) in runs {
                    let _ = writeln!(out, "\t{start}\t{}", results.join(" "));
                }
            }
        }
    };
    for expression in CORPUS {
        write(expression, compare(expression, &edges, 6));
    }
    let mut runner = TestRunner::new_with_rng(
        Config::default(),
        TestRng::from_seed(RngAlgorithm::ChaCha, &[13; 32]),
    );
    let mut valid = 0;
    let mut rejected = 0;
    while valid < 1_500 {
        let expression = expression().new_tree(&mut runner).unwrap().current();
        let starts: Vec<i64> = prop::collection::vec(instant(), 3)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        let runs = compare(&expression, &starts, 4);
        match runs {
            Some(_) => valid += 1,
            // Keep one rejected expression in four.
            None if rejected * 3 > valid => continue,
            None => rejected += 1,
        }
        write(&expression, runs);
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/schedules/cron_expression_vectors.txt");
    std::fs::write(path, out).unwrap();
}
