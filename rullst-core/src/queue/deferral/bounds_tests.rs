//! Exact bounds: 366-day deferral deadlines and half-open forecast slots.
#![allow(clippy::unwrap_used)]

use super::*;
use std::time::{Duration, UNIX_EPOCH};

const YEAR_AND_A_DAY: Duration = Duration::from_secs(366 * 24 * 3_600);

#[test]
fn deadlines_may_be_exactly_366_days_ahead() {
    assert!(Deferral::within(YEAR_AND_A_DAY).is_ok());
    let now = UNIX_EPOCH + Duration::from_secs(1_791_417_600);
    assert_eq!(Deferral::until(now + YEAR_AND_A_DAY).validate(now), Ok(()));
    assert_eq!(
        Deferral::until(now + YEAR_AND_A_DAY + Duration::from_millis(1)).validate(now),
        Err(DeferralError::DeadlineTooFar)
    );
}

#[tokio::test]
async fn a_fixed_forecast_returns_only_slots_overlapping_the_query() {
    let hour = |h: u64| UNIX_EPOCH + Duration::from_secs(1_791_417_600 + h * 3_600);
    let source = FixedIntensitySource::new("fixed", "g")
        .slot(hour(0), hour(1), 10.0)
        .slot(hour(1), hour(2), 20.0)
        .slot(hour(2), hour(3), 30.0);
    // `[1h, 2h)`: the slots that end at 1h or start at 2h only touch it.
    let forecast = source.forecast(hour(1), hour(2)).await.unwrap();
    let values = forecast
        .slots()
        .iter()
        .map(|slot| slot.value)
        .collect::<Vec<_>>();
    assert_eq!(values, [20.0]);
    assert_eq!(forecast.unit(), "g");
}
