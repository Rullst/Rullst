//! POSIX five-field cron expressions evaluated in UTC, exactly as Rullst
//! Core's scheduler evaluates them (`Scheduler::task`).
//!
//! The fields are `minute hour day-of-month month day-of-week`. Day-of-week
//! numbers weekdays 0 through 7 (0 and 7 are Sunday) and, when both day fields
//! are restricted (neither starts with `*` or `?`), a day matching *either*
//! runs; otherwise both must match. The other four fields follow the grammar
//! in [`field`], and each field must be valid on its own. Occurrences are
//! whole minutes from 1970 through 2100.
//!
//! This is a copy of `rullst-core/src/scheduler_cron.rs` and
//! `scheduler_cron_field.rs` on Unix milliseconds instead of jiff values;
//! a differential test checks it against Core's evaluator.

use super::cron_expression::{civil_from_days, days_from_civil};
use field::{DAY_OF_MONTH, HOUR, MINUTE, MONTH, Values};

#[path = "posix_cron_field.rs"]
mod field;

/// The last year searched.
const LAST_YEAR: i64 = 2100;

/// 0000-01-01T00:00:00Z in Unix seconds.
const YEAR_ZERO: i64 = -62_167_219_200;

const SECONDS_PER_DAY: i64 = 86_400;

/// A validated POSIX five-field schedule.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct PosixSchedule {
    minutes: Values,
    hours: Values,
    days_of_month: Values,
    months: Values,
    /// 0 (Sunday) through 6 (Saturday).
    days_of_week: Values,
    /// Both day fields are restricted, so a day matching either one runs.
    either_day: bool,
}

impl PosixSchedule {
    /// Parses `minute hour day-of-month month day-of-week`.
    pub(super) fn parse(expression: &str) -> Result<Self, String> {
        let fields: Vec<&str> = expression.split_whitespace().collect();
        let [minute, hour, day_of_month, month, day_of_week] = fields[..] else {
            return Err(
                "expected five fields: minute hour day-of-month month day-of-week".to_string(),
            );
        };
        let days_of_week = weekdays(day_of_week)?;
        let unrestricted = |field: &str| field.starts_with(['*', '?']);
        Ok(Self {
            minutes: MINUTE.parse(minute)?,
            hours: HOUR.parse(hour)?,
            days_of_month: DAY_OF_MONTH.parse(day_of_month)?,
            months: MONTH.parse(month)?,
            days_of_week,
            either_day: !unrestricted(day_of_month) && !unrestricted(day_of_week),
        })
    }

    /// The first occurrence strictly after `after_ms`, in Unix milliseconds:
    /// the first matching whole minute after it, from 1970 through 2100.
    /// There is none when the second after `after_ms` falls before year 0,
    /// and an earlier instant starts the search in 1970.
    pub(super) fn next_after(&self, after_ms: i64) -> Option<i64> {
        let second = after_ms.div_euclid(1_000);
        if second.checked_add(1)? < YEAR_ZERO {
            return None;
        }
        let first = second
            .div_euclid(60)
            .checked_add(1)?
            .checked_mul(60)?
            .max(0);
        let mut day = first.div_euclid(SECONDS_PER_DAY);
        let mut earliest = first.rem_euclid(SECONDS_PER_DAY);
        let last_day = days_from_civil(LAST_YEAR, 12, 31);
        while day <= last_day {
            let (_, month, day_of_month) = civil_from_days(day);
            if self.months.contains(as_value(month)) && self.runs_on(day, day_of_month) {
                if let Some(minute) = self.first_minute_from(earliest / 60) {
                    return (day * SECONDS_PER_DAY + minute * 60).checked_mul(1_000);
                }
            }
            day += 1;
            earliest = 0;
        }
        None
    }

    /// Whether the day fields select `day`, the `day_of_month` of its month.
    fn runs_on(&self, day: i64, day_of_month: i64) -> bool {
        let day_of_month = self.days_of_month.contains(as_value(day_of_month));
        // 1970-01-01 (day 0) was a Thursday, day 4 counted from Sunday as 0.
        let day_of_week = self
            .days_of_week
            .contains(as_value((day + 4).rem_euclid(7)));
        if self.either_day {
            day_of_month || day_of_week
        } else {
            day_of_month && day_of_week
        }
    }

    /// The first matching minute of the day at or after minute `earliest`.
    fn first_minute_from(&self, earliest: i64) -> Option<i64> {
        let hour = as_value(earliest / 60);
        if self.hours.contains(hour) {
            if let Some(minute) = self.minutes.first_from(as_value(earliest % 60)) {
                return Some(i64::from(hour) * 60 + i64::from(minute));
            }
        }
        let hour = self.hours.first_from(hour + 1)?;
        Some(i64::from(hour) * 60 + i64::from(self.minutes.first_from(0)?))
    }
}

/// A calendar or clock value, all of which fit a `u32`.
fn as_value(value: i64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Parses a POSIX day-of-week field into days 0 (Sunday) to 6.
fn weekdays(field: &str) -> Result<Values, String> {
    if field.starts_with(['*', '?']) && !field.contains(',') {
        let step = match field.get(1..) {
            Some("") => 1,
            Some(rest) => parse_step(rest.strip_prefix('/').ok_or_else(|| invalid(field))?)?,
            None => return Err(invalid(field)),
        };
        return Ok(weekday_set((0..=6).step_by(step)));
    }

    let mut days = [false; 7];
    for element in field.split(',') {
        let (range, step) = match element.split_once('/') {
            Some((range, step)) => (range, parse_step(step)?),
            None => (element, 1),
        };
        let (start, end) = match range.split_once('-') {
            Some((start, end)) => {
                let start = weekday(start)?;
                let mut end = weekday(end)?;
                // Allow a range to end on Sunday named or numbered as 0.
                if end == 0 && start > 0 {
                    end = 7;
                }
                if start > end {
                    return Err(format!("day-of-week range `{range}` is reversed"));
                }
                (start, end)
            }
            None if step == 1 => {
                let day = weekday(range)?;
                (day, day)
            }
            None => {
                return Err(format!(
                    "day-of-week step `{element}` needs a range, such as `1-5/2` or `*/2`"
                ));
            }
        };
        for day in (start..=end).step_by(step) {
            days[usize::from(day % 7)] = true;
        }
    }
    Ok(weekday_set(
        days.iter()
            .enumerate()
            .filter_map(|(day, selected)| selected.then_some(day)),
    ))
}

/// Parses one POSIX weekday: 0-7 (0 and 7 are Sunday) or an English name.
fn weekday(value: &str) -> Result<u8, String> {
    if let Ok(number) = value.parse::<u8>() {
        return if number <= 7 {
            Ok(number)
        } else {
            Err(format!(
                "day-of-week `{value}` must be 0-7 (0 and 7 are Sunday) or a name"
            ))
        };
    }
    let day = match value.to_ascii_lowercase().as_str() {
        "sun" | "sunday" => 0,
        "mon" | "monday" => 1,
        "tue" | "tues" | "tuesday" => 2,
        "wed" | "wednesday" => 3,
        "thu" | "thurs" | "thursday" => 4,
        "fri" | "friday" => 5,
        "sat" | "saturday" => 6,
        _ => return Err(invalid(value)),
    };
    Ok(day)
}

fn parse_step(value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(step) if (1..=7).contains(&step) => Ok(step),
        _ => Err(format!("day-of-week step `{value}` must be 1-7")),
    }
}

fn invalid(value: &str) -> String {
    format!("day-of-week `{value}` is not a POSIX weekday, list, range or step")
}

/// The set of the distinct days 0 (Sunday) to 6 in `days`.
fn weekday_set(days: impl Iterator<Item = usize>) -> Values {
    Values::from_bits(days.map(|day| 1_u64 << day).sum())
}

#[cfg(test)]
#[path = "posix_cron_tests.rs"]
mod tests;
