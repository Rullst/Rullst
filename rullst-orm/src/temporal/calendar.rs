//! Civil dates and date-times counted from 1970-01-01, without going through
//! `jiff::Timestamp`, whose range is about a day narrower at each end than a
//! civil value's. jiff bounds a day count added to a date by its distance
//! from 1970, so the counts start there.

// Each pool uses part of this module; a build with one driver leaves the rest
// unused.
#![cfg_attr(not(feature = "drivers-all"), allow(dead_code))]

use jiff::{SignedDuration, civil};

const NANOS_PER_DAY: i128 = 86_400_000_000_000;

/// The date `days` after 1970-01-01.
pub(super) fn date_from_unix_days(days: i64) -> Option<civil::Date> {
    civil::Date::constant(1970, 1, 1)
        .checked_add(SignedDuration::from_secs(days.checked_mul(86_400)?))
        .ok()
}

/// The civil date-time `nanos` nanoseconds after 1970-01-01T00:00:00.
pub(super) fn date_time_from_unix_nanos(nanos: i128) -> Option<civil::DateTime> {
    let days = i64::try_from(nanos.div_euclid(NANOS_PER_DAY)).ok()?;
    let time_of_day = i64::try_from(nanos.rem_euclid(NANOS_PER_DAY)).ok()?;
    let time = civil::Time::midnight().wrapping_add(SignedDuration::from_nanos(time_of_day));
    Some(date_from_unix_days(days)?.to_datetime(time))
}

#[cfg(test)]
mod tests {
    use super::{date_from_unix_days, date_time_from_unix_nanos};

    #[test]
    fn counts_reach_both_ends_of_the_civil_range() {
        assert_eq!(date_from_unix_days(0).unwrap().to_string(), "1970-01-01");
        assert_eq!(date_from_unix_days(-1).unwrap().to_string(), "1969-12-31");
        assert_eq!(
            date_from_unix_days(2_932_896).unwrap().to_string(),
            "9999-12-31"
        );
        assert_eq!(date_from_unix_days(2_932_897), None);
        assert_eq!(
            date_from_unix_days(-4_371_587).unwrap().to_string(),
            "-009999-01-01"
        );
        assert_eq!(date_from_unix_days(-4_371_588), None);
        assert_eq!(date_from_unix_days(i64::MAX), None);
        assert_eq!(
            date_time_from_unix_nanos(-1).unwrap().to_string(),
            "1969-12-31T23:59:59.999999999"
        );
        assert_eq!(
            date_time_from_unix_nanos(253_402_300_799_999_999_999)
                .unwrap()
                .to_string(),
            "9999-12-31T23:59:59.999999999"
        );
        assert_eq!(date_time_from_unix_nanos(253_402_300_800_000_000_000), None);
        assert_eq!(
            date_time_from_unix_nanos(-377_705_116_800_000_000_000)
                .unwrap()
                .to_string(),
            "-009999-01-01T00:00:00"
        );
        assert_eq!(
            date_time_from_unix_nanos(-377_705_116_800_000_000_001),
            None
        );
        assert_eq!(date_time_from_unix_nanos(i128::MAX), None);
    }
}
