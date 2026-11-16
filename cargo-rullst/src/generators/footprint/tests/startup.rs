//! The start-up time: the median of the timed cold starts, how many starts
//! run, that each one is stopped before the next, and the `startup_ms`
//! limit. The starts are injected, so no process is spawned and no clock is
//! read.
use super::super::budget;
use super::super::startup::{self, Startup};
use super::*;
use std::cell::RefCell;
use std::rc::Rc;

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

/// A started app that records its stop in the shared log when dropped.
struct FakeApp {
    id: usize,
    log: Rc<RefCell<Vec<String>>>,
}

impl Drop for FakeApp {
    fn drop(&mut self) {
        self.log.borrow_mut().push(format!("stop {}", self.id));
    }
}

/// The kept app's id and the timed starts, or the error of a start.
type Outcome = Result<(usize, Vec<Duration>), String>;

/// Runs [`startup::cold_starts`] with starts that take `times` in turn and
/// fail with `"start {n} failed"` from start `fail_at` on; returns the kept
/// app's id, the timed starts and the log of starts and stops.
fn starts(times: &[Duration], fail_at: Option<usize>) -> (Outcome, Vec<String>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut next = 0;
    let result = block_on(startup::cold_starts(|| {
        next += 1;
        let id = next;
        let log = Rc::clone(&log);
        let elapsed = times[(id - 1) % times.len()];
        async move {
            log.borrow_mut().push(format!("start {id}"));
            if fail_at.is_some_and(|fail| id >= fail) {
                return Err(format!("start {id} failed"));
            }
            Ok((FakeApp { id, log }, elapsed))
        }
    }));
    let result = result.map(|(app, samples)| (app.id, samples));
    let log = log.borrow().clone();
    (result, log)
}

#[test]
fn five_fast_starts_are_timed_and_only_the_last_app_is_kept() {
    let (result, log) = starts(&[Duration::from_millis(12)], None);
    let (kept, samples) = result.unwrap();
    assert_eq!(kept, 5);
    assert_eq!(samples, [Duration::from_millis(12); 5]);
    // Each app stops before the next one starts; the fifth is dropped only
    // here, after the starts returned it.
    assert_eq!(
        log,
        [
            "start 1", "stop 1", "start 2", "stop 2", "start 3", "stop 3", "start 4", "stop 4",
            "start 5", "stop 5",
        ]
    );
}

#[test]
fn a_slow_app_is_started_until_the_time_budget_is_spent() {
    assert_eq!(startup::MAX_STARTS, 5);
    assert_eq!(startup::TIME_BUDGET, Duration::from_secs(20));
    // 8 s, 16 s: under the budget; 24 s: spent.
    let (result, _) = starts(&[Duration::from_secs(8)], None);
    assert_eq!(result.unwrap().1, [Duration::from_secs(8); 3]);
    // A start that alone takes the whole budget is the only one.
    let (result, log) = starts(&[Duration::from_secs(20)], None);
    assert_eq!(result.unwrap(), (1, vec![Duration::from_secs(20)]));
    assert_eq!(log, ["start 1", "stop 1"]);
    // Just under it, a second start runs.
    let (result, _) = starts(
        &[Duration::from_millis(19_999), Duration::from_millis(1)],
        None,
    );
    assert_eq!(result.unwrap().0, 2);
}

#[test]
fn another_start_runs_while_both_bounds_allow_it() {
    let times = |count: usize, each: u64| vec![Duration::from_millis(each); count];
    assert!(startup::another(&[]));
    assert!(startup::another(&times(4, 10)));
    assert!(!startup::another(&times(5, 10)));
    assert!(!startup::another(&times(6, 10)));
    assert!(startup::another(&times(1, 19_999)));
    assert!(!startup::another(&times(1, 20_000)));
    assert!(!startup::another(&times(2, 10_000)));
    assert!(startup::another(&times(2, 9_999)));
}

#[test]
fn a_failed_start_ends_the_starts_after_stopping_the_earlier_apps() {
    let (result, log) = starts(&[Duration::from_millis(30)], Some(3));
    assert_eq!(result, Err("start 3 failed".to_string()));
    assert_eq!(log, ["start 1", "stop 1", "start 2", "stop 2", "start 3"]);
    let (result, log) = starts(&[Duration::from_millis(30)], Some(1));
    assert_eq!(result, Err("start 1 failed".to_string()));
    assert_eq!(log, ["start 1"]);
}

#[test]
fn the_median_is_the_middle_start_or_the_mean_of_the_two_middle_ones() {
    assert_eq!(startup::median(&[]), None);
    assert_eq!(startup::median(&[7.5]), Some(7.5));
    assert_eq!(startup::median(&[30.0, 10.0]), Some(20.0));
    assert_eq!(startup::median(&[30.0, 10.0, 20.0]), Some(20.0));
    assert_eq!(startup::median(&[4.0, 1.0, 3.0, 2.0]), Some(2.5));
    assert_eq!(startup::median(&[12.3, 10.0, 15.5, 11.0, 13.0]), Some(12.3));
    assert_eq!(startup::median(&[9.0, 1.0, 1.0, 1.0, 50.0]), Some(1.0));
}

#[test]
fn times_are_kept_to_a_tenth_of_a_millisecond() {
    assert_eq!(startup::milliseconds(Duration::ZERO), 0.0);
    assert_eq!(startup::milliseconds(Duration::from_micros(12_340)), 12.3);
    assert_eq!(startup::milliseconds(Duration::from_micros(12_350)), 12.4);
    assert_eq!(startup::milliseconds(Duration::from_micros(49)), 0.0);
    assert_eq!(startup::milliseconds(Duration::from_micros(50)), 0.1);
    assert_eq!(startup::milliseconds(Duration::from_secs(2)), 2000.0);
}

#[test]
fn the_section_says_how_many_starts_were_timed_or_why_none_was() {
    let measured = Startup::measured("/health", &startup_samples());
    assert_eq!(
        measured,
        Startup {
            status: "measured",
            reason: None,
            method: Some(startup::METHOD),
            path: Some("/health".to_string()),
            runs: Some(5),
            samples_ms: vec![12.3, 10.0, 15.5, 11.0, 13.0],
            median_ms: Some(12.3),
            min_ms: Some(10.0),
            max_ms: Some(15.5),
        }
    );
    let two = Startup::measured("/", &[Duration::from_millis(40), Duration::from_millis(10)]);
    assert_eq!(
        (two.runs, two.median_ms, two.min_ms, two.max_ms),
        (Some(2), Some(25.0), Some(10.0), Some(40.0))
    );
    assert_eq!(
        Startup::measured("/", &[]),
        Startup::unmeasured("no start was timed")
    );
    let reason = super::super::subject::REMOTE;
    assert_eq!(
        Startup::unmeasured(reason),
        Startup {
            status: "not_measured",
            reason: Some(reason.to_string()),
            method: None,
            path: None,
            runs: None,
            samples_ms: Vec::new(),
            median_ms: None,
            min_ms: None,
            max_ms: None,
        }
    );
    assert!(startup::METHOD.contains("probed every 5 ms"));
}

#[test]
fn the_text_row_shows_the_median_and_the_range_of_the_starts() {
    let plain = Style { color: false };
    let mut report = sample(true, None);
    report.startup = Startup::measured("/health", &[Duration::from_micros(1_234_560)]);
    let text = render::render(&report, plain);
    assert!(
        text.contains("Startup time  1234.6 ms"),
        "missing the median in:\n{text}"
    );
    assert!(
        text.contains(
            "median of 1 cold starts (1234.6 to 1234.6 ms), spawn to the first answer to GET /health\n"
        ),
        "{text}"
    );
    report.startup = Startup::unmeasured("the app did not start");
    let text = render::render(&report, plain);
    assert!(text.contains("Startup time  NOT MEASURED"), "{text}");
    assert!(text.contains("the app did not start"), "{text}");
}

#[test]
fn the_startup_limit_checks_the_median_and_skips_an_unmeasured_time() {
    let limit = budget::parse("[footprint.budgets]\nstartup_ms = 12\n", "Rullst.toml").unwrap();
    let checked = budget::check(&limit, &sample(true, None));
    assert_eq!(checked.status, "exceeded");
    assert_eq!(checked.limits[0].name, "startup_ms");
    assert_eq!(checked.limits[0].unit, "milliseconds");
    assert_eq!(checked.limits[0].measured, Some(12.3));
    let mut report = sample(true, None);
    report.budgets = checked;
    let text = render::render(&report, Style { color: false });
    assert!(
        text.contains("  EXCEEDED  startup_ms                12.300 ms > 12.000 ms allowed"),
        "{text}"
    );

    let roomy = budget::parse("[footprint.budgets]\nstartup_ms = 12.3\n", "x").unwrap();
    assert_eq!(budget::check(&roomy, &sample(true, None)).status, "passed");

    let skipped = budget::check(&limit, &sample(false, None));
    assert_eq!(skipped.status, "skipped");
    assert_eq!(
        skipped.limits[0].reason.as_deref(),
        Some(startup::ALREADY_RUNNING)
    );
    let mut silent = sample(false, None);
    silent.startup.reason = None;
    assert_eq!(
        budget::figure("startup_ms", &silent),
        Err("not measured".into())
    );
}
