//! Opt-in deferral of queue jobs and scheduled tasks that need not run
//! immediately (reports, exports, re-indexing).
//!
//! A deferrable job has a deadline (`run_by`) and optional daily
//! [`TimeWindow`]s. Without an intensity source it becomes claimable at the
//! start of the next allowed window (now, when a window is open), never after
//! its deadline; when no window opens before the deadline it becomes claimable
//! at the deadline. With a [`CarbonAwarePlanner`] the job is placed in the
//! lowest-intensity forecast slot inside its windows before the deadline.
//!
//! Placement is expressed through [`Queue::dispatch_at`], so drivers and
//! stored rows need no new column: a deferred job is an ordinary scheduled
//! job. The planner only chooses a time; it measures nothing and makes no
//! claim about emissions.

mod intensity;
mod planner;
mod schedule;
mod window;

pub use intensity::{
    CarbonIntensitySource, FixedIntensitySource, IntensityForecast, IntensitySlot,
    IntensitySourceError,
};
pub use planner::{CarbonAwarePlanner, DEFAULT_SOURCE_TIMEOUT};
pub use schedule::ScheduleDeferral;
pub use window::TimeWindow;

use super::{MAX_SCHEDULE_DELAY, Queue, QueueError};
use serde_json::Value;
use std::time::{Duration, SystemTime};
use tracing::Instrument;
use window::{from_millis, to_millis};

/// Most windows one deferral accepts.
pub const MAX_DEFERRAL_WINDOWS: usize = 24;

/// Errors raised while configuring a deferral. Unpublished v13 API.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DeferralError {
    /// A window time or UTC offset is out of range, or the window is empty.
    #[error("invalid deferral window: {0}")]
    InvalidWindow(String),
    /// More than [`MAX_DEFERRAL_WINDOWS`] windows were added.
    #[error("a deferral accepts at most {MAX_DEFERRAL_WINDOWS} windows")]
    TooManyWindows,
    /// The deadline is more than 366 days ahead, beyond what the queue stores.
    #[error("deferral deadlines may be at most 366 days ahead")]
    DeadlineTooFar,
}

impl From<DeferralError> for QueueError {
    fn from(error: DeferralError) -> Self {
        QueueError::InvalidConfiguration(error.to_string())
    }
}

/// Why a deferred job was placed where it was. Unpublished v13 API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeferralReason {
    /// Start of the next allowed window (or now, inside an open window).
    Window,
    /// The lowest-intensity forecast slot inside the allowed windows.
    Intensity,
    /// No window opened before the deadline, or the deadline has passed.
    Deadline,
}

impl DeferralReason {
    /// Stable lowercase name: `window`, `intensity` or `deadline`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Window => "window",
            Self::Intensity => "intensity",
            Self::Deadline => "deadline",
        }
    }
}

/// The placement chosen for one deferred job. Unpublished v13 API.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DeferralPlan {
    /// When the job becomes claimable (millisecond precision).
    pub run_at: SystemTime,
    /// Which rule chose [`Self::run_at`].
    pub reason: DeferralReason,
    /// Name of the intensity source consulted, if any.
    pub source: Option<String>,
    /// The source's forecast value for the chosen slot, when the reason is
    /// [`DeferralReason::Intensity`]; never estimated by Rullst.
    pub intensity: Option<f64>,
    /// Unit of [`Self::intensity`], as reported by the source.
    pub unit: Option<String>,
    /// Whether the source failed or timed out and the window rule was used.
    pub source_failed: bool,
}

/// A job dispatched with a deferral. Unpublished v13 API.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DeferredJob {
    /// Identifier returned by the queue driver.
    pub id: String,
    /// Where and why the job was placed.
    pub plan: DeferralPlan,
}

/// Marks a job as deferrable: it must become claimable no later than
/// `run_by` and preferably inside one of its windows. Unpublished v13 API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deferral {
    run_by: SystemTime,
    windows: Vec<TimeWindow>,
}

impl Deferral {
    /// A deferral whose job becomes claimable no later than `run_by`.
    pub fn until(run_by: SystemTime) -> Self {
        Self {
            run_by,
            windows: Vec::new(),
        }
    }

    /// A deferral whose deadline is `max_delay` from now.
    ///
    /// # Errors
    /// Returns [`DeferralError::DeadlineTooFar`] beyond 366 days.
    pub fn within(max_delay: Duration) -> Result<Self, DeferralError> {
        if max_delay > MAX_SCHEDULE_DELAY {
            return Err(DeferralError::DeadlineTooFar);
        }
        SystemTime::now()
            .checked_add(max_delay)
            .map(Self::until)
            .ok_or(DeferralError::DeadlineTooFar)
    }

    /// Adds an allowed window. Without windows any time before the deadline
    /// is allowed.
    ///
    /// # Errors
    /// Returns [`DeferralError::TooManyWindows`] beyond
    /// [`MAX_DEFERRAL_WINDOWS`].
    pub fn window(mut self, window: TimeWindow) -> Result<Self, DeferralError> {
        if self.windows.len() >= MAX_DEFERRAL_WINDOWS {
            return Err(DeferralError::TooManyWindows);
        }
        self.windows.push(window);
        Ok(self)
    }

    /// The deadline.
    pub fn run_by(&self) -> SystemTime {
        self.run_by
    }

    /// The allowed windows, in the order they were added.
    pub fn windows(&self) -> &[TimeWindow] {
        &self.windows
    }

    /// Applies the time-window and deadline rules at `now`, without an
    /// intensity source.
    pub fn plan(&self, now: SystemTime) -> DeferralPlan {
        let now_ms = to_millis(now);
        let run_by_ms = to_millis(self.run_by).min(horizon_ms(now_ms));
        let (run_at, reason) = planner::window_rule(&self.windows, now_ms, run_by_ms);
        DeferralPlan {
            run_at: from_millis(run_at),
            reason,
            source: None,
            intensity: None,
            unit: None,
            source_failed: false,
        }
    }

    fn validate(&self, now: SystemTime) -> Result<(), DeferralError> {
        match self.run_by.duration_since(now) {
            Ok(delay) if delay > MAX_SCHEDULE_DELAY => Err(DeferralError::DeadlineTooFar),
            _ => Ok(()),
        }
    }
}

/// Latest instant a plan may choose: the queue's 366-day scheduling bound.
fn horizon_ms(now_ms: i64) -> i64 {
    let bound = i64::try_from(MAX_SCHEDULE_DELAY.as_millis()).unwrap_or(i64::MAX);
    now_ms.saturating_add(bound)
}

impl Queue {
    /// Dispatches a deferrable job placed by the time-window and deadline
    /// rules (see [`Deferral`]). Unpublished v13 API.
    ///
    /// The job is stored through [`Self::dispatch_at`], so the queue's
    /// at-least-once semantics and poll-based start apply, and a custom
    /// driver without durable scheduling rejects a future placement. The
    /// placement is recorded on a `rullst.queue.deferral` tracing span.
    ///
    /// # Errors
    /// [`QueueError::InvalidConfiguration`] for an invalid job name or a
    /// deadline beyond 366 days, and any error of [`Self::dispatch_at`].
    pub async fn dispatch_deferred(
        &self,
        job_name: &str,
        payload: Value,
        deferral: &Deferral,
    ) -> Result<DeferredJob, QueueError> {
        let now = SystemTime::now();
        super::bounds::validate_job_name(job_name)?;
        deferral.validate(now)?;
        self.dispatch_planned(job_name, payload, deferral.plan(now))
            .await
    }

    /// Dispatches a deferrable job placed by `planner`: the lowest-intensity
    /// slot inside its windows before its deadline, or the time-window rule
    /// when the source fails. Unpublished v13 API.
    ///
    /// # Errors
    /// As [`Self::dispatch_deferred`]; a failing source is not an error.
    pub async fn dispatch_deferred_with<S: CarbonIntensitySource>(
        &self,
        planner: &CarbonAwarePlanner<S>,
        job_name: &str,
        payload: Value,
        deferral: &Deferral,
    ) -> Result<DeferredJob, QueueError> {
        let now = SystemTime::now();
        super::bounds::validate_job_name(job_name)?;
        deferral.validate(now)?;
        let plan = planner.plan(deferral, now).await;
        self.dispatch_planned(job_name, payload, plan).await
    }

    async fn dispatch_planned(
        &self,
        job_name: &str,
        payload: Value,
        plan: DeferralPlan,
    ) -> Result<DeferredJob, QueueError> {
        let span = tracing::info_span!(
            target: "rullst::queue",
            "rullst.queue.deferral",
            job.name = job_name,
            job.id = tracing::field::Empty,
            deferral.run_at_unix_ms = to_millis(plan.run_at),
            deferral.reason = plan.reason.as_str(),
            deferral.source = plan.source.as_deref().unwrap_or("none"),
            deferral.source_failed = plan.source_failed,
        );
        let id = self
            .dispatch_at(job_name, payload, plan.run_at)
            .instrument(span.clone())
            .await?;
        span.record("job.id", id.as_str());
        Ok(DeferredJob { id, plan })
    }
}

#[cfg(all(test, feature = "queue-sqlite", not(miri)))]
mod sqlite_tests;
#[cfg(test)]
mod tests;
