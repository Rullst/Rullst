//! Where a task's cron fields are matched: against UTC, or (with the
//! `scheduler-tz` feature) against the wall clock of a named IANA time zone.

use super::cron_expression::CronSchedule;
use chrono::{DateTime, Utc};

#[cfg(feature = "scheduler-tz")]
use super::{ScheduledHandler, ScheduledTask, Scheduler, SchedulerError};
#[cfg(feature = "scheduler-tz")]
use chrono::TimeDelta;
#[cfg(feature = "scheduler-tz")]
use jiff::Timestamp;
#[cfg(feature = "scheduler-tz")]
use jiff::tz::{AmbiguousOffset, Offset, TimeZone};
#[cfg(feature = "scheduler-tz")]
use std::{future::Future, sync::Arc};

/// A task's cron schedule and the clock its fields are matched against.
pub(crate) enum TaskSchedule {
    /// Fields matched against UTC, as [`Scheduler::task`](super::Scheduler::task) does.
    Utc(CronSchedule),
    /// Fields matched against a time zone's wall clock.
    #[cfg(feature = "scheduler-tz")]
    Zoned(ZonedCron),
}

impl From<CronSchedule> for TaskSchedule {
    fn from(schedule: CronSchedule) -> Self {
        Self::Utc(schedule)
    }
}

impl TaskSchedule {
    /// Returns the first occurrence strictly after `after`.
    pub(crate) fn next_after(&self, after: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Utc(schedule) => schedule.next_after(after),
            #[cfg(feature = "scheduler-tz")]
            Self::Zoned(schedule) => schedule.next_after(after),
        }
    }
}

/// Upper bound on the wall times one search examines. Only the repeated hour
/// of a daylight-saving fold makes the search pass over matches, one per
/// match in it, so this is far above a per-minute schedule in a 25-hour fold.
#[cfg(feature = "scheduler-tz")]
const MAX_CANDIDATES: usize = 2_000;

/// A cron schedule matched against the wall clock of an IANA time zone.
#[cfg(feature = "scheduler-tz")]
pub(crate) struct ZonedCron {
    schedule: CronSchedule,
    zone: TimeZone,
}

#[cfg(feature = "scheduler-tz")]
impl ZonedCron {
    /// Looks `zone_name` up in the IANA time-zone database.
    pub(crate) fn new(schedule: CronSchedule, zone_name: &str) -> Result<Self, String> {
        let zone = TimeZone::get(zone_name).map_err(|error| error.to_string())?;
        Ok(Self { schedule, zone })
    }

    /// Returns the first occurrence strictly after `after`.
    pub(crate) fn next_after(&self, after: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        let offset = self.zone.to_offset(timestamp(after)?);
        // Wall times are written as UTC values, which is how the cron fields
        // are matched. Start one second before the current wall time, so a
        // match at that very time is examined and rejected below.
        let mut wall = shift(after, offset.seconds())?.checked_sub_signed(TimeDelta::seconds(1))?;
        for _ in 0..MAX_CANDIDATES {
            wall = self.schedule.next_after(&wall)?;
            let instant = self.instant_of(&wall)?;
            // Only false for a match at or before the current wall time, or
            // in the repeated hour of a fold that already ran.
            if instant > *after {
                return Some(instant);
            }
        }
        None
    }

    /// The instant at which the task runs for the wall time `wall`.
    fn instant_of(&self, wall: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        let civil = Offset::UTC.to_datetime(timestamp(wall)?);
        match self.zone.to_ambiguous_timestamp(civil).offset() {
            AmbiguousOffset::Unambiguous { offset } => shift(wall, -offset.seconds()),
            // A wall time that a fold repeats runs once, at its first instant.
            AmbiguousOffset::Fold { before, .. } => shift(wall, -before.seconds()),
            // A wall time that a gap skips runs when the gap ends. Read with
            // the later offset it lies before the transition, which is
            // therefore the first one following it.
            AmbiguousOffset::Gap { after, .. } => {
                let inside_gap = timestamp(&shift(wall, -after.seconds())?)?;
                let transition = self.zone.following(inside_gap).next()?;
                DateTime::from_timestamp(transition.timestamp().as_second(), 0)
            }
        }
    }
}

/// `instant` moved by `seconds`, or `None` outside the supported range.
#[cfg(feature = "scheduler-tz")]
fn shift(instant: &DateTime<Utc>, seconds: i32) -> Option<DateTime<Utc>> {
    instant.checked_add_signed(TimeDelta::seconds(i64::from(seconds)))
}

#[cfg(feature = "scheduler-tz")]
fn timestamp(instant: &DateTime<Utc>) -> Option<Timestamp> {
    let nanosecond = i32::try_from(instant.timestamp_subsec_nanos()).ok()?;
    Timestamp::new(instant.timestamp(), nanosecond).ok()
}

#[cfg(feature = "scheduler-tz")]
impl Scheduler {
    /// Registers a recurring task, like [`Self::task`], whose five cron fields
    /// are matched against the wall clock of the IANA time zone `time_zone`,
    /// such as `America/Sao_Paulo`, instead of UTC. Requires the
    /// `scheduler-tz` feature. Added in v13.
    ///
    /// Daylight-saving changes follow one rule:
    /// - a wall time that a gap skips (clocks move forward) runs at the first
    ///   instant after the gap: in `America/New_York`, `30 2 * * *` runs at
    ///   03:00 EDT on the day clocks spring forward, and several skipped
    ///   times run once there;
    /// - a wall time that a fold repeats (clocks move back) runs once, at its
    ///   first occurrence: `30 1 * * *` runs at 01:30 EDT and not again at
    ///   01:30 EST, and matches in the repeated hour do not run twice.
    ///
    /// Zone rules come from the system database (`/usr/share/zoneinfo`) when
    /// it has the zone, otherwise from the copy bundled with the feature.
    /// The task's label in [`SchedulerError`] values is the expression
    /// followed by the zone in brackets, such as `0 7 * * * [America/Sao_Paulo]`.
    ///
    /// ```
    /// # extern crate rullst_core as rullst;
    /// use rullst::Scheduler;
    ///
    /// // 07:00 in São Paulo, which is 10:00 UTC.
    /// let scheduler = Scheduler::new().task_in_tz("0 7 * * *", "America/Sao_Paulo", || async {
    ///     // send the morning digest
    /// })?;
    /// assert!(matches!(
    ///     Scheduler::new().task_in_tz("0 7 * * *", "Brasilia", || async {}),
    ///     Err(rullst::SchedulerError::InvalidTimeZone { .. })
    /// ));
    /// # drop(scheduler);
    /// # Ok::<(), rullst::SchedulerError>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`SchedulerError::InvalidCron`] like [`Self::task`], and
    /// [`SchedulerError::InvalidTimeZone`] when the database has no zone of
    /// that name.
    pub fn task_in_tz<F, Fut>(
        mut self,
        cron_expr: &str,
        time_zone: &str,
        handler: F,
    ) -> Result<Self, SchedulerError>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let schedule = CronSchedule::parse(cron_expr)
            .map_err(|error| SchedulerError::InvalidCron(cron_expr.to_string(), error))?;
        let schedule = ZonedCron::new(schedule, time_zone).map_err(|message| {
            SchedulerError::InvalidTimeZone {
                zone: time_zone.to_string(),
                message,
            }
        })?;
        let boxed: ScheduledHandler = Arc::new(Box::new(move || Box::pin(handler())));

        self.tasks.push(ScheduledTask {
            label: format!("{cron_expr} [{time_zone}]"),
            schedule: TaskSchedule::Zoned(schedule),
            handler: boxed,
            deferral: None,
        });
        Ok(self)
    }
}

#[cfg(all(test, feature = "scheduler-tz"))]
#[path = "scheduler_zone_tests.rs"]
mod tests;
