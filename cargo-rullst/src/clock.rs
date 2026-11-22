//! The wall-clock stamps the CLI writes: migration file names and the daily
//! UI stamps in the local time zone, report times in UTC. Each format is the
//! one the CLI wrote with chrono before 13.0.0-alpha.6, byte for byte, so
//! migration names keep sorting in creation order and report parsers keep
//! reading the same text. The formatters take the instant (and the zone), so
//! tests pin them; [`now`] and [`local_zone`] read the system.

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::{Offset, TimeZone};

/// The current instant. Fails only when the system clock is outside jiff's
/// range, the years -9999 to 9999.
pub(crate) fn now() -> Result<Timestamp, jiff::Error> {
    Timestamp::try_from(std::time::SystemTime::now())
}

/// The system's time zone: `TZ`, else the operating system's setting, else
/// UTC, as chrono's `Local` chose.
pub(crate) fn local_zone() -> TimeZone {
    TimeZone::system()
}

/// `YYYYMMDDHHMMSS` of `at` in `zone`, the stamp of migration file names
/// (`m20261117093000_create_users.rs`).
pub(crate) fn migration_stamp(at: Timestamp, zone: &TimeZone) -> String {
    let time = zone.to_datetime(at);
    format!(
        "{}{:02}{:02}{:02}",
        date_digits(time),
        time.hour(),
        time.minute(),
        time.second()
    )
}

/// The migration stamp of the current instant in the local time zone.
pub(crate) fn local_migration_stamp() -> Result<String, jiff::Error> {
    Ok(migration_stamp(now()?, &local_zone()))
}

/// The stamp of `at` in `zone` and a stamp that sorts after it, for two
/// migrations of one command that must run in that order: one second later,
/// or, when the local clock went back meanwhile (the end of daylight saving
/// time), one second after the repeated hour.
pub(crate) fn ordered_migration_stamps(
    at: Timestamp,
    zone: &TimeZone,
) -> Result<(String, String), jiff::Error> {
    let first = migration_stamp(at, zone);
    let mut later = String::new();
    for seconds in [1, 3_601, 7_201] {
        later = migration_stamp(
            at.checked_add(jiff::SignedDuration::from_secs(seconds))?,
            zone,
        );
        if later > first {
            break;
        }
    }
    Ok((first, later))
}

/// `YYYY-MM-DD` of `at` in `zone`.
pub(crate) fn date(at: Timestamp, zone: &TimeZone) -> String {
    let time = zone.to_datetime(at);
    format!("{:04}-{:02}-{:02}", time.year(), time.month(), time.day())
}

/// RFC 3339 in UTC as chrono's `to_rfc3339()` wrote it: the offset as
/// `+00:00` and the fraction in 0, 3, 6 or 9 digits, the fewest that keep the
/// instant exact (`2026-11-17T09:30:00.250+00:00`).
pub(crate) fn rfc3339(at: Timestamp) -> String {
    let time = Offset::UTC.to_datetime(at);
    let nanos = time.subsec_nanosecond();
    let fraction = if nanos == 0 {
        String::new()
    } else if nanos % 1_000_000 == 0 {
        format!(".{:03}", nanos / 1_000_000)
    } else if nanos % 1_000 == 0 {
        format!(".{:06}", nanos / 1_000)
    } else {
        format!(".{nanos:09}")
    };
    format!("{}{fraction}+00:00", seconds(time))
}

/// RFC 3339 in UTC to the second, with `Z` (`2026-11-17T09:30:00Z`); the
/// fraction is cut, not rounded.
pub(crate) fn rfc3339_seconds(at: Timestamp) -> String {
    format!("{}Z", seconds(Offset::UTC.to_datetime(at)))
}

/// `YYYYMMDDTHHMMSSZ` in UTC (`20261117T093000Z`).
pub(crate) fn compact_utc(at: Timestamp) -> String {
    let time = Offset::UTC.to_datetime(at);
    format!(
        "{}T{:02}{:02}{:02}Z",
        date_digits(time),
        time.hour(),
        time.minute(),
        time.second()
    )
}

/// `YYYYMMDD`.
fn date_digits(time: DateTime) -> String {
    format!("{:04}{:02}{:02}", time.year(), time.month(), time.day())
}

/// `YYYY-MM-DDTHH:MM:SS`.
fn seconds(time: DateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        time.year(),
        time.month(),
        time.day(),
        time.hour(),
        time.minute(),
        time.second()
    )
}

#[cfg(test)]
#[path = "clock_tests.rs"]
mod tests;
