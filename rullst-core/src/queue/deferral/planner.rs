//! Placement of a deferrable job: the time-window rule, the deadline rule and,
//! when a source is configured, the lowest-intensity slot.

use super::intensity::{CarbonIntensitySource, IntensityForecast};
use super::window::{TimeWindow, from_millis, to_millis};
use super::{Deferral, DeferralPlan, DeferralReason};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

/// Default bound on one [`CarbonIntensitySource::forecast`] call.
pub const DEFAULT_SOURCE_TIMEOUT: Duration = Duration::from_secs(2);
/// Forecast slots inspected per placement; later slots are ignored.
const MAX_FORECAST_SLOTS: usize = 10_000;

/// The allowed intervals inside `[from_ms, until_ms)`: every window
/// occurrence, or the whole range when no window is configured.
pub(super) fn allowed_intervals(
    windows: &[TimeWindow],
    from_ms: i64,
    until_ms: i64,
) -> Vec<(i64, i64)> {
    if from_ms >= until_ms {
        return Vec::new();
    }
    if windows.is_empty() {
        return vec![(from_ms, until_ms)];
    }
    windows
        .iter()
        .flat_map(|window| window.intervals(from_ms, until_ms))
        .collect()
}

/// The time-window rule: start of the earliest allowed interval before the
/// deadline, or the deadline itself when none fits. A deadline already
/// reached means "run now".
pub(super) fn window_rule(
    windows: &[TimeWindow],
    now_ms: i64,
    run_by_ms: i64,
) -> (i64, DeferralReason) {
    if run_by_ms <= now_ms {
        return (now_ms, DeferralReason::Deadline);
    }
    allowed_intervals(windows, now_ms, run_by_ms)
        .iter()
        .map(|(start, _)| *start)
        .min()
        .map_or((run_by_ms, DeferralReason::Deadline), |start| {
            (start, DeferralReason::Window)
        })
}

/// The lowest-intensity slot overlapping an allowed interval, as the start of
/// that overlap and the slot's value. Ties go to the earliest start.
pub(super) fn lowest_intensity(
    intervals: &[(i64, i64)],
    forecast: &IntensityForecast,
) -> Option<(i64, f64)> {
    let mut best: Option<(i64, f64)> = None;
    for slot in forecast.slots().iter().take(MAX_FORECAST_SLOTS) {
        let (start, end) = (to_millis(slot.start), to_millis(slot.end));
        if !slot.value.is_finite() || slot.value < 0.0 || end <= start {
            continue;
        }
        for (allowed_start, allowed_end) in intervals {
            let overlap_start = start.max(*allowed_start);
            if overlap_start >= end.min(*allowed_end) {
                continue;
            }
            let better = best.is_none_or(|(best_start, best_value)| {
                match slot.value.total_cmp(&best_value) {
                    std::cmp::Ordering::Less => true,
                    std::cmp::Ordering::Equal => overlap_start < best_start,
                    std::cmp::Ordering::Greater => false,
                }
            });
            if better {
                best = Some((overlap_start, slot.value));
            }
        }
    }
    best
}

/// Places deferrable jobs in the lowest-intensity slot reported by a
/// [`CarbonIntensitySource`], inside the job's windows and before its
/// deadline. Unpublished v13 API.
///
/// When the source fails or exceeds its timeout, the placement falls back to
/// the time-window rule. The first failure of an outage is logged as a
/// warning and later ones at debug level until the source answers again.
#[derive(Debug)]
pub struct CarbonAwarePlanner<S> {
    source: S,
    timeout: Duration,
    fallback_logged: AtomicBool,
}

impl<S: CarbonIntensitySource> CarbonAwarePlanner<S> {
    /// Creates a planner that consults `source` with a
    /// [`DEFAULT_SOURCE_TIMEOUT`] bound.
    pub fn new(source: S) -> Self {
        Self {
            source,
            timeout: DEFAULT_SOURCE_TIMEOUT,
            fallback_logged: AtomicBool::new(false),
        }
    }

    /// Bounds each forecast call; a zero timeout always falls back.
    pub fn with_source_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The configured source.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Chooses when a job deferred by `deferral` becomes claimable.
    pub async fn plan(&self, deferral: &Deferral, now: SystemTime) -> DeferralPlan {
        let now_ms = to_millis(now);
        let run_by_ms = to_millis(deferral.run_by()).min(super::horizon_ms(now_ms));
        let intervals = allowed_intervals(deferral.windows(), now_ms, run_by_ms);
        let (Some(first), Some(last)) = (
            intervals.iter().map(|(start, _)| *start).min(),
            intervals.iter().map(|(_, end)| *end).max(),
        ) else {
            return deferral.plan(now);
        };
        let name = self.source.name().to_string();
        let forecast = tokio::time::timeout(
            self.timeout,
            self.source.forecast(from_millis(first), from_millis(last)),
        )
        .await;
        let failure = match forecast {
            Ok(Ok(forecast)) => {
                self.fallback_logged.store(false, Ordering::Relaxed);
                let mut plan = deferral.plan(now);
                plan.source = Some(name);
                if let Some((start, value)) = lowest_intensity(&intervals, &forecast) {
                    plan.run_at = from_millis(start);
                    plan.reason = DeferralReason::Intensity;
                    plan.intensity = Some(value);
                    plan.unit = Some(forecast.unit().to_string());
                }
                return plan;
            }
            Ok(Err(error)) => error.to_string(),
            Err(_) => format!("no forecast within {} ms", self.timeout.as_millis()),
        };
        if self.fallback_logged.swap(true, Ordering::Relaxed) {
            tracing::debug!(
                target: "rullst::queue",
                source = %name,
                error = %failure,
                "carbon intensity source still failing; using the time-window rule"
            );
        } else {
            tracing::warn!(
                target: "rullst::queue",
                source = %name,
                error = %failure,
                "carbon intensity source failed; using the time-window rule"
            );
        }
        let mut plan = deferral.plan(now);
        plan.source = Some(name);
        plan.source_failed = true;
        plan
    }
}
