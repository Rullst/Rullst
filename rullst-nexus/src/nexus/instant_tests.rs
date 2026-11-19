use jiff::civil::{Date, DateTime, Time, date};

use super::*;

fn local(text: &str) -> DateTime {
    text.parse().unwrap()
}

fn at(text: &str, offset: i32) -> Instant {
    Instant::at_offset(local(text), offset).unwrap()
}

#[test]
fn instants_are_wall_times_minus_their_offset() {
    let instant = at("2026-10-15T09:30:00.5", -3 * 3600);
    assert_eq!(instant, at("2026-10-15T12:30:00.5", 0));
    assert_ne!(instant, at("2026-10-15T09:30:00.5", 0));
    assert_eq!(
        instant.timestamp(),
        Some(jiff::Timestamp::new(1_792_067_400, 500_000_000).unwrap())
    );
    assert_eq!(instant.wall_time(0), Some(local("2026-10-15T12:30:00.5")));
    assert_eq!(
        instant.wall_time(19_800),
        Some(local("2026-10-15T18:00:00.5"))
    );
    assert_eq!(
        instant.wall_time(-86_399),
        Some(local("2026-10-14T12:30:01.5"))
    );
    assert_eq!(
        Instant::at_offset(local("1970-01-01T00:00:00"), 0)
            .unwrap()
            .timestamp(),
        Some(jiff::Timestamp::UNIX_EPOCH)
    );
}

#[test]
fn utc_text_matches_chrono_auto_si_forms() {
    for (wall, offset, plus, z) in [
        (
            "2026-10-15T09:30:00",
            0,
            "2026-10-15T09:30:00+00:00",
            "2026-10-15T09:30:00Z",
        ),
        (
            "2026-10-15T09:30:00.5",
            7_200,
            "2026-10-15T07:30:00.500+00:00",
            "2026-10-15T07:30:00.500Z",
        ),
        (
            "2026-10-15T09:30:00.000120",
            0,
            "2026-10-15T09:30:00.000120+00:00",
            "2026-10-15T09:30:00.000120Z",
        ),
        (
            "2026-10-15T09:30:00.000000007",
            0,
            "2026-10-15T09:30:00.000000007+00:00",
            "2026-10-15T09:30:00.000000007Z",
        ),
        // Year 0, which an offset reaches from 0001-01-01.
        (
            "0001-01-01T00:30:00",
            3_600,
            "0000-12-31T23:30:00+00:00",
            "0000-12-31T23:30:00Z",
        ),
    ] {
        let instant = at(wall, offset);
        assert_eq!(instant.rfc3339(false).as_deref(), Some(plus), "{wall}");
        assert_eq!(instant.rfc3339(true).as_deref(), Some(z), "{wall}");
    }
}

#[test]
fn instants_beyond_jiff_timestamps_keep_their_chrono_text() {
    // jiff's last instant.
    let last = at("9999-12-30T22:00:00.999999999", 0);
    assert!(last.timestamp().is_some());
    let later = at("9999-12-30T22:00:01", 0);
    assert_eq!(later.timestamp(), None);
    assert_eq!(
        later.rfc3339(false).as_deref(),
        Some("9999-12-30T22:00:01+00:00")
    );
    assert_eq!(
        later.utc_wall_text().as_deref(),
        Some("9999-12-30 22:00:01")
    );

    // A negative offset on 9999-12-31 names an instant in year 10000.
    let year_10000 = at("9999-12-31T23:00:00.25", -5 * 3600);
    assert_eq!(year_10000.timestamp(), None);
    assert_eq!(year_10000.wall_time(0), None);
    assert_eq!(
        year_10000.wall_time(-5 * 3600),
        Some(local("9999-12-31T23:00:00.25"))
    );
    assert_eq!(
        year_10000.rfc3339(false).as_deref(),
        Some("+10000-01-01T04:00:00.250+00:00")
    );
    assert_eq!(
        year_10000.rfc3339(true).as_deref(),
        Some("+10000-01-01T04:00:00.250Z")
    );
    assert_eq!(
        year_10000.utc_wall_text().as_deref(),
        Some("+10000-01-01 04:00:00.250")
    );
    let midnight = at("9999-12-31T23:59:59.999999999", -1);
    assert_eq!(
        midnight.rfc3339(true).as_deref(),
        Some("+10000-01-01T00:00:00.999999999Z")
    );
    let last_minute = at("9999-12-31T23:59:59", -86_399);
    assert_eq!(
        last_minute.rfc3339(true).as_deref(),
        Some("+10000-01-01T23:59:58Z")
    );
}

#[test]
fn instants_past_year_10000_have_no_text() {
    let beyond = at("9999-12-31T23:00:00", 0);
    let next_day = Instant(beyond.0.checked_add(ONE_DAY).unwrap());
    assert_eq!(
        next_day.rfc3339(false).as_deref(),
        Some("+10000-01-01T23:00:00+00:00")
    );
    let two_days = Instant(beyond.0.checked_add(ONE_DAY + ONE_DAY).unwrap());
    assert_eq!(two_days.rfc3339(false), None);
    assert_eq!(two_days.utc_wall_text(), None);
    let early = Instant(
        date(-9999, 1, 1)
            .at(0, 0, 0, 0)
            .duration_since(UNIX_EPOCH)
            .checked_sub(SignedDuration::from_secs(1))
            .unwrap(),
    );
    assert_eq!(early.rfc3339(true), None);
}

#[test]
fn dates_and_times_are_written_as_chrono_wrote_them() {
    assert_eq!(date_text(date(1, 2, 3)), "0001-02-03");
    assert_eq!(date_text(date(9999, 12, 31)), "9999-12-31");
    assert_eq!(date_text(Date::new(0, 1, 1).unwrap()), "0000-01-01");
    for (nanos, text) in [
        (0, "07:08:09"),
        (1_000_000, "07:08:09.001"),
        (500_000_000, "07:08:09.500"),
        (1_000, "07:08:09.000001"),
        (120_000, "07:08:09.000120"),
        (1_001_000, "07:08:09.001001"),
        (1, "07:08:09.000000001"),
        (123_456_789, "07:08:09.123456789"),
        (100_000_010, "07:08:09.100000010"),
    ] {
        let time = Time::new(7, 8, 9, nanos).unwrap();
        assert_eq!(time_text(time), text, "{nanos}");
    }
    assert_eq!(
        date_time_text(local("2026-10-15T09:30:00.25"), 'T'),
        "2026-10-15T09:30:00.250"
    );
    assert_eq!(
        date_time_text(local("2026-10-15T09:30:00"), ' '),
        "2026-10-15 09:30:00"
    );
}
