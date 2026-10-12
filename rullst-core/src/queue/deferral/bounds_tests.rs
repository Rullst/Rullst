//! Exact 366-day bounds of deferral deadlines.

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
