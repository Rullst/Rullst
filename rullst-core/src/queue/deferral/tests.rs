#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::window::{from_millis, to_millis};
use super::*;
use async_trait::async_trait;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HOUR: i64 = 3_600_000;
/// 2026-10-08T00:00:00Z, a UTC midnight.
const MIDNIGHT: i64 = 1_791_417_600_000;

fn at(ms: i64) -> SystemTime {
    from_millis(ms)
}

fn hour(h: i64) -> SystemTime {
    at(MIDNIGHT + h * HOUR)
}

fn night() -> TimeWindow {
    TimeWindow::daily(0, 0, 6, 0).unwrap()
}

#[test]
fn midnight_constant_is_a_utc_day_boundary() {
    assert_eq!(MIDNIGHT % super::window::DAY_MS, 0);
}

#[test]
fn windows_contain_their_start_but_not_their_end() {
    let window = night();
    assert!(window.contains(hour(0)));
    assert!(window.contains(at(MIDNIGHT + 6 * HOUR - 1)));
    assert!(!window.contains(hour(6)));
    assert!(!window.contains(at(MIDNIGHT - 1)));
}

#[test]
fn windows_that_end_before_they_start_wrap_midnight() {
    let window = TimeWindow::daily(22, 0, 6, 0).unwrap();
    assert!(window.contains(hour(23)));
    assert!(window.contains(hour(0)));
    assert!(window.contains(at(MIDNIGHT + 6 * HOUR - 1)));
    assert!(!window.contains(hour(6)));
    assert!(!window.contains(hour(21)));
    assert!(window.contains(hour(22)));
}

#[test]
fn utc_offsets_shift_the_window() {
    // 00:00–06:00 at UTC−03:00 is 03:00–09:00 UTC.
    let window = night().with_utc_offset_minutes(-180).unwrap();
    assert_eq!(window.utc_offset_minutes(), -180);
    assert!(!window.contains(at(MIDNIGHT + 3 * HOUR - 1)));
    assert!(window.contains(hour(3)));
    assert!(window.contains(at(MIDNIGHT + 9 * HOUR - 1)));
    assert!(!window.contains(hour(9)));

    // 01:00–05:00 at UTC+05:30 is 19:30–23:30 UTC on the previous day.
    let window = TimeWindow::daily(1, 0, 5, 0)
        .unwrap()
        .with_utc_offset_minutes(330)
        .unwrap();
    assert!(window.contains(at(MIDNIGHT - 4 * HOUR - HOUR / 2)));
    assert!(!window.contains(at(MIDNIGHT - HOUR / 2)));
    assert!(!window.contains(hour(1)));
}

#[test]
fn full_day_and_invalid_windows() {
    let all_day = TimeWindow::daily(0, 0, 24, 0).unwrap();
    assert!(all_day.contains(hour(0)));
    assert!(all_day.contains(at(MIDNIGHT - 1)));

    for (start_hour, start_minute, end_hour, end_minute) in [
        (24, 0, 6, 0),
        (0, 60, 6, 0),
        (0, 0, 24, 30),
        (25, 0, 1, 0),
        (6, 0, 6, 0),
    ] {
        assert!(matches!(
            TimeWindow::daily(start_hour, start_minute, end_hour, end_minute),
            Err(DeferralError::InvalidWindow(_))
        ));
    }
    assert!(night().with_utc_offset_minutes(841).is_err());
    assert!(night().with_utc_offset_minutes(-841).is_err());
}

#[test]
fn intervals_cover_wrapped_occurrences_and_clip_to_the_range() {
    let window = TimeWindow::daily(22, 0, 6, 0).unwrap();
    // From 01:00 to 23:00 next day: the open occurrence (01:00–06:00), the
    // evening occurrence (22:00–06:00 next day) and the next evening's start.
    let intervals = window.intervals(MIDNIGHT + HOUR, MIDNIGHT + 47 * HOUR);
    assert_eq!(
        intervals,
        vec![
            (MIDNIGHT + HOUR, MIDNIGHT + 6 * HOUR),
            (MIDNIGHT + 22 * HOUR, MIDNIGHT + 30 * HOUR),
            (MIDNIGHT + 46 * HOUR, MIDNIGHT + 47 * HOUR),
        ]
    );
    assert!(window.intervals(MIDNIGHT, MIDNIGHT).is_empty());
}

#[test]
fn the_window_rule_starts_now_inside_an_open_window() {
    let deferral = Deferral::until(hour(30)).window(night()).unwrap();
    let now = at(MIDNIGHT + 2 * HOUR + 1);
    let plan = deferral.plan(now);
    assert_eq!(plan.run_at, now);
    assert_eq!(plan.reason, DeferralReason::Window);
    assert_eq!(plan.source, None);
}

#[test]
fn the_window_rule_waits_for_the_next_window_including_day_wrap() {
    let deferral = Deferral::until(hour(40)).window(night()).unwrap();
    let plan = deferral.plan(hour(9));
    assert_eq!(plan.run_at, hour(24));
    assert_eq!(plan.reason, DeferralReason::Window);

    let evening = Deferral::until(hour(40))
        .window(TimeWindow::daily(22, 0, 6, 0).unwrap())
        .unwrap();
    assert_eq!(evening.plan(hour(21)).run_at, hour(22));

    let offset = Deferral::until(hour(40))
        .window(night().with_utc_offset_minutes(-180).unwrap())
        .unwrap();
    assert_eq!(offset.plan(hour(2)).run_at, hour(3));
}

#[test]
fn the_earliest_of_several_windows_wins() {
    let deferral = Deferral::until(hour(40))
        .window(TimeWindow::daily(20, 0, 21, 0).unwrap())
        .unwrap()
        .window(TimeWindow::daily(13, 30, 14, 0).unwrap())
        .unwrap();
    assert_eq!(
        deferral.plan(hour(9)).run_at,
        at(MIDNIGHT + 13 * HOUR + HOUR / 2)
    );
}

#[test]
fn the_deadline_rule_applies_when_no_window_opens_in_time() {
    let deferral = Deferral::until(hour(20)).window(night()).unwrap();
    let plan = deferral.plan(hour(9));
    assert_eq!(plan.run_at, hour(20));
    assert_eq!(plan.reason, DeferralReason::Deadline);

    // A window opening exactly at the deadline does not fit before it.
    let plan = Deferral::until(hour(24))
        .window(night())
        .unwrap()
        .plan(hour(9));
    assert_eq!(
        (plan.run_at, plan.reason),
        (hour(24), DeferralReason::Deadline)
    );

    // A deadline already reached means now.
    let plan = Deferral::until(hour(1))
        .window(night())
        .unwrap()
        .plan(hour(9));
    assert_eq!(
        (plan.run_at, plan.reason),
        (hour(9), DeferralReason::Deadline)
    );
}

#[test]
fn without_windows_any_time_before_the_deadline_is_allowed() {
    let plan = Deferral::until(hour(20)).plan(hour(9));
    assert_eq!(
        (plan.run_at, plan.reason),
        (hour(9), DeferralReason::Window)
    );
}

#[test]
fn deferrals_are_bounded() {
    let mut deferral = Deferral::until(hour(20));
    for _ in 0..MAX_DEFERRAL_WINDOWS {
        deferral = deferral.window(night()).unwrap();
    }
    assert_eq!(
        deferral.window(night()).unwrap_err(),
        DeferralError::TooManyWindows
    );
    assert_eq!(
        Deferral::within(Duration::from_secs(367 * 24 * 3_600)).unwrap_err(),
        DeferralError::DeadlineTooFar
    );
    assert!(Deferral::within(Duration::from_secs(60)).is_ok());
    assert!(matches!(
        QueueError::from(DeferralError::DeadlineTooFar),
        QueueError::InvalidConfiguration(_)
    ));
}

#[test]
fn schedule_deferral_places_each_tick() {
    let deferral = ScheduleDeferral::within(Duration::from_secs(12 * 3_600))
        .unwrap()
        .window(night())
        .unwrap();
    assert_eq!(deferral.max_delay(), Duration::from_secs(12 * 3_600));
    // A 18:00 tick waits for midnight; a 03:00 tick runs immediately.
    assert_eq!(
        deferral.start_for(MIDNIGHT + 18 * HOUR),
        (MIDNIGHT + 24 * HOUR, DeferralReason::Window)
    );
    assert_eq!(
        deferral.start_for(MIDNIGHT + 3 * HOUR),
        (MIDNIGHT + 3 * HOUR, DeferralReason::Window)
    );
    // A 07:00 tick cannot reach the next window within 12 hours.
    assert_eq!(
        deferral.start_for(MIDNIGHT + 7 * HOUR),
        (MIDNIGHT + 19 * HOUR, DeferralReason::Deadline)
    );
    assert!(ScheduleDeferral::within(Duration::from_secs(367 * 24 * 3_600)).is_err());
}

fn fixed() -> FixedIntensitySource {
    FixedIntensitySource::new("fixed-test", "gCO2eq/kWh")
        .slot(hour(24), hour(26), 300.0)
        .slot(hour(26), hour(28), 120.0)
        .slot(hour(28), hour(30), 200.0)
        // Outside the window: lower, but never chosen.
        .slot(hour(36), hour(37), 10.0)
}

#[tokio::test]
async fn the_planner_chooses_the_lowest_slot_inside_the_windows() {
    let planner = CarbonAwarePlanner::new(fixed());
    let deferral = Deferral::until(hour(40)).window(night()).unwrap();
    let plan = planner.plan(&deferral, hour(9)).await;
    assert_eq!(plan.run_at, hour(26));
    assert_eq!(plan.reason, DeferralReason::Intensity);
    assert_eq!(plan.source.as_deref(), Some("fixed-test"));
    assert_eq!(plan.intensity, Some(120.0));
    assert_eq!(plan.unit.as_deref(), Some("gCO2eq/kWh"));
    assert!(!plan.source_failed);
    assert_eq!(planner.source().name(), "fixed-test");
}

#[tokio::test]
async fn the_planner_respects_the_deadline_ties_and_partial_overlaps() {
    let planner = CarbonAwarePlanner::new(fixed());
    // Deadline 27:00 excludes the 28:00 slot and cuts the 26:00 slot.
    let deferral = Deferral::until(hour(27)).window(night()).unwrap();
    assert_eq!(planner.plan(&deferral, hour(9)).await.run_at, hour(26));

    // A slot that started before the window opened is used from its opening.
    let early = CarbonAwarePlanner::new(
        FixedIntensitySource::new("early", "g")
            .slot(hour(23), hour(25), 50.0)
            .slot(hour(25), hour(30), 50.0)
            .slot(hour(30), hour(31), f64::NAN)
            .slot(hour(29), hour(29), 1.0)
            .slot(hour(28), hour(29), -1.0),
    );
    let plan = early.plan(&deferral, hour(9)).await;
    assert_eq!((plan.run_at, plan.intensity), (hour(24), Some(50.0)));
}

#[tokio::test]
async fn a_forecast_without_usable_slots_keeps_the_window_rule() {
    let planner = CarbonAwarePlanner::new(FixedIntensitySource::new("empty", "g"));
    let deferral = Deferral::until(hour(40)).window(night()).unwrap();
    let plan = planner.plan(&deferral, hour(9)).await;
    assert_eq!(
        (plan.run_at, plan.reason),
        (hour(24), DeferralReason::Window)
    );
    assert_eq!(plan.source.as_deref(), Some("empty"));
    assert!(!plan.source_failed);
}

struct FailingSource;

#[async_trait]
impl CarbonIntensitySource for FailingSource {
    fn name(&self) -> &str {
        "failing"
    }

    async fn forecast(
        &self,
        _from: SystemTime,
        _until: SystemTime,
    ) -> Result<IntensityForecast, IntensitySourceError> {
        Err(IntensitySourceError::unavailable("offline"))
    }
}

struct HangingSource;

#[async_trait]
impl CarbonIntensitySource for HangingSource {
    fn name(&self) -> &str {
        "hanging"
    }

    async fn forecast(
        &self,
        _from: SystemTime,
        _until: SystemTime,
    ) -> Result<IntensityForecast, IntensitySourceError> {
        std::future::pending().await
    }
}

#[tokio::test]
async fn source_failures_and_timeouts_fall_back_to_the_window_rule() {
    let deferral = Deferral::until(hour(40)).window(night()).unwrap();
    let expected = deferral.plan(hour(9));

    let failing = CarbonAwarePlanner::new(FailingSource);
    for _ in 0..2 {
        let plan = failing.plan(&deferral, hour(9)).await;
        assert_eq!(
            (plan.run_at, plan.reason),
            (expected.run_at, expected.reason)
        );
        assert_eq!(plan.source.as_deref(), Some("failing"));
        assert!(plan.source_failed);
        assert_eq!(plan.intensity, None);
    }

    let hanging =
        CarbonAwarePlanner::new(HangingSource).with_source_timeout(Duration::from_millis(20));
    let plan = hanging.plan(&deferral, hour(9)).await;
    assert_eq!(
        (plan.run_at, plan.reason),
        (expected.run_at, expected.reason)
    );
    assert!(plan.source_failed);
}

#[tokio::test]
async fn a_passed_deadline_does_not_consult_the_source() {
    let planner = CarbonAwarePlanner::new(FailingSource);
    let deferral = Deferral::until(hour(1)).window(night()).unwrap();
    let plan = planner.plan(&deferral, hour(9)).await;
    assert_eq!(
        (plan.run_at, plan.reason),
        (hour(9), DeferralReason::Deadline)
    );
    assert_eq!(plan.source, None);
    assert!(!plan.source_failed);
}

#[test]
fn millisecond_conversions_round_down() {
    let instant = UNIX_EPOCH + Duration::from_nanos(1_500_000);
    assert_eq!(to_millis(instant), 1);
    // 1 µs, not 1 ns: Windows `SystemTime` has 100 ns resolution.
    assert_eq!(to_millis(UNIX_EPOCH - Duration::from_micros(1)), -1);
    assert_eq!(from_millis(-5), UNIX_EPOCH);
    assert_eq!(DeferralReason::Intensity.as_str(), "intensity");
    assert_eq!(DeferralReason::Window.as_str(), "window");
    assert_eq!(DeferralReason::Deadline.as_str(), "deadline");
}

struct ToggleSource(std::sync::atomic::AtomicBool);

#[async_trait]
impl CarbonIntensitySource for ToggleSource {
    fn name(&self) -> &str {
        "toggle"
    }

    async fn forecast(
        &self,
        _from: SystemTime,
        _until: SystemTime,
    ) -> Result<IntensityForecast, IntensitySourceError> {
        if self.0.load(std::sync::atomic::Ordering::SeqCst) {
            Err(IntensitySourceError::unavailable("offline"))
        } else {
            Ok(IntensityForecast::new("g"))
        }
    }
}

struct WarnCounter(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for WarnCounter {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        if *event.metadata().level() == tracing::Level::WARN {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

#[tokio::test]
async fn an_outage_is_logged_as_a_warning_once() {
    use tracing_subscriber::layer::SubscriberExt;

    let warnings = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let subscriber = tracing_subscriber::registry().with(WarnCounter(warnings.clone()));
    let _guard = tracing::subscriber::set_default(subscriber);
    let planner = CarbonAwarePlanner::new(ToggleSource(std::sync::atomic::AtomicBool::new(true)));
    let deferral = Deferral::until(hour(40)).window(night()).unwrap();
    let count = || warnings.load(std::sync::atomic::Ordering::SeqCst);

    planner.plan(&deferral, hour(9)).await;
    planner.plan(&deferral, hour(9)).await;
    assert_eq!(count(), 1);
    planner
        .source()
        .0
        .store(false, std::sync::atomic::Ordering::SeqCst);
    assert!(!planner.plan(&deferral, hour(9)).await.source_failed);
    planner
        .source()
        .0
        .store(true, std::sync::atomic::Ordering::SeqCst);
    planner.plan(&deferral, hour(9)).await;
    assert_eq!(count(), 2);
}
