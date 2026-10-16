//! Server-independent bounds of the PostgreSQL suppression adapter.

use super::operations::{check_stored_times, observation_time};
use super::*;

#[test]
fn the_server_clock_must_be_positive_and_leave_room_for_the_skew_allowance() {
    assert_eq!(clock_seconds(1), Ok(1));
    assert_eq!(clock_seconds(1_700_000_000), Ok(1_700_000_000));
    let latest = 9_223_372_036_854_775_507_u64;
    assert_eq!(clock_seconds(latest), Ok(9_223_372_036_854_775_507));
    for value in [0, latest + 1, 9_223_372_036_854_775_807, u64::MAX] {
        assert_eq!(
            clock_seconds(value),
            Err(SuppressionError::InvalidConfiguration("server clock"))
        );
    }
}

#[test]
fn event_times_must_be_positive_and_at_most_300_seconds_ahead() {
    assert_eq!(observation_time(1, 1_000), Ok(1));
    assert_eq!(observation_time(1_300, 1_000), Ok(1_300));
    for observed in [0, 1_301, 300_000, 9_223_372_036_854_775_808] {
        assert_eq!(
            observation_time(observed, 1_000),
            Err(SuppressionError::InvalidEvent("observation time"))
        );
    }
}

#[test]
fn stored_recipient_times_must_be_ordered_and_at_most_300_seconds_ahead() {
    for (first, last) in [(1, 1), (1, 1_300), (900, 1_000)] {
        assert_eq!(check_stored_times(first, last, 1_000), Ok(()));
    }
    for (first, last) in [(0, 5), (-1, 5), (10, 5), (10, 1_301), (10, 300_000)] {
        assert_eq!(
            check_stored_times(first, last, 1_000),
            Err(SuppressionError::CorruptStorage("recipient time"))
        );
    }
}

#[test]
fn configuration_debug_shows_only_the_quotas() {
    let config = PostgresSuppressionConfig::new("tenant-a", 7, 9).unwrap();
    assert_eq!(
        format!("{config:?}"),
        "PostgresSuppressionConfig { max_recipients: 7, max_events: 9, .. }"
    );
}
