//! Start-up time (13.0.0-alpha.6): how long the release binary footprint
//! starts takes from process spawn to its first HTTP answer on `--path`, as
//! the median of a few cold starts, each a new process. A scale-to-zero host
//! pays this time on the first request after an idle period, on top of
//! pulling and starting the container, which footprint does not measure. An
//! app found with `--url` was already running and a published site has no
//! local process, so neither has a start-up time.
use serde::Serialize;
use std::future::Future;
use std::time::Duration;

/// Cold starts timed at most; the last one is the app footprint measures.
pub(super) const MAX_STARTS: usize = 5;
/// No further start begins once the timed starts add up to this, so a slow
/// app is started fewer times (at least once).
pub(super) const TIME_BUDGET: Duration = Duration::from_secs(20);

pub(super) const METHOD: &str = "time from spawning the release binary to its first HTTP response (any status) to GET of the path, probed every 5 ms; each start is a new process, stopped before the next one; the figure is the median of the starts";

/// Why an app found with `--url` has no start-up time.
pub(super) const ALREADY_RUNNING: &str = "not measured for an app that was already running (--url): footprint times the start-up of the release binary it starts";

/// The start-up section of the report.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Startup {
    /// `measured` or `not_measured`.
    pub status: &'static str,
    pub reason: Option<String>,
    pub method: Option<&'static str>,
    /// The path whose first answer ended each start.
    pub path: Option<String>,
    /// How many cold starts were timed (at most five).
    pub runs: Option<u64>,
    /// Each start's time in milliseconds, in the order they ran.
    pub samples_ms: Vec<f64>,
    /// The median of `samples_ms`: the figure the text report shows and the
    /// `startup_ms` budget limit checks.
    pub median_ms: Option<f64>,
    pub min_ms: Option<f64>,
    pub max_ms: Option<f64>,
}

impl Startup {
    /// No start-up time, for `reason`.
    pub(super) fn unmeasured(reason: impl Into<String>) -> Self {
        Self {
            status: "not_measured",
            reason: Some(reason.into()),
            method: None,
            path: None,
            runs: None,
            samples_ms: Vec::new(),
            median_ms: None,
            min_ms: None,
            max_ms: None,
        }
    }

    /// The start-up time of the starts in `samples`, each ended by an answer
    /// to `path`.
    pub(super) fn measured(path: &str, samples: &[Duration]) -> Self {
        let samples_ms: Vec<f64> = samples.iter().copied().map(milliseconds).collect();
        let Some(median_ms) = median(&samples_ms) else {
            return Self::unmeasured("no start was timed");
        };
        Self {
            status: "measured",
            reason: None,
            method: Some(METHOD),
            path: Some(path.to_string()),
            runs: Some(samples_ms.len() as u64),
            median_ms: Some(median_ms),
            min_ms: samples_ms.iter().copied().reduce(f64::min),
            max_ms: samples_ms.iter().copied().reduce(f64::max),
            samples_ms,
        }
    }
}

/// `duration` in milliseconds, to a tenth of a millisecond.
pub(super) fn milliseconds(duration: Duration) -> f64 {
    (duration.as_secs_f64() * 10_000.0).round() / 10.0
}

/// The middle value, or the mean of the two middle values of an even count.
pub(super) fn median(values: &[f64]) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    let upper = *sorted.get(middle)?;
    if sorted.len() % 2 == 1 {
        return Some(upper);
    }
    let lower = *sorted.get(middle.checked_sub(1)?)?;
    Some((lower + upper) / 2.0)
}

/// Whether another cold start begins after the starts in `samples`.
pub(super) fn another(samples: &[Duration]) -> bool {
    samples.len() < MAX_STARTS && samples.iter().sum::<Duration>() < TIME_BUDGET
}

/// Starts the app with `start` until [`another`] says enough: each call
/// spawns a new process and returns it with the time it took to answer.
/// Every app but the last is dropped, which stops it, before the next one
/// starts; the last one is returned with every start's time. The first
/// error ends the starts and is returned.
pub(super) async fn cold_starts<A, E, F, Fut>(mut start: F) -> Result<(A, Vec<Duration>), E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<(A, Duration), E>>,
{
    let mut samples = Vec::with_capacity(MAX_STARTS);
    loop {
        let (app, elapsed) = start().await?;
        samples.push(elapsed);
        if !another(&samples) {
            return Ok((app, samples));
        }
        drop(app);
    }
}
