//! Window occurrences on a zone's wall clock, here a fixed UTC−03:00 clock.

#![allow(clippy::unwrap_used)]

use super::*;

const HOUR: i64 = 3_600_000;

fn local_of(instant: i64) -> Option<i64> {
    Some(instant - 3 * HOUR)
}

fn instant_of(wall: i64) -> Option<i64> {
    Some(wall + 3 * HOUR)
}

#[test]
fn occurrences_are_clipped_to_the_range_and_an_empty_range_has_none() {
    // 22:00–06:00 at UTC−03:00 is 01:00–09:00 UTC.
    let night = TimeWindow::daily(22, 0, 6, 0).unwrap();
    let day_ms = 20_000 * DAY_MS;
    assert_eq!(
        night.zoned_intervals(
            day_ms + 2 * HOUR,
            day_ms + 30 * HOUR,
            &local_of,
            &instant_of
        ),
        vec![
            (day_ms + 2 * HOUR, day_ms + 9 * HOUR),
            (day_ms + 25 * HOUR, day_ms + 30 * HOUR)
        ]
    );
    assert_eq!(
        night.zoned_intervals(day_ms + 2 * HOUR, day_ms + 2 * HOUR, &local_of, &instant_of),
        Vec::new()
    );
    let unknown = |_: i64| None;
    assert_eq!(
        night.zoned_intervals(day_ms, day_ms + 30 * HOUR, &unknown, &instant_of),
        Vec::new()
    );
    assert_eq!(
        night.zoned_intervals(day_ms, day_ms + 30 * HOUR, &local_of, &unknown),
        Vec::new()
    );
}
