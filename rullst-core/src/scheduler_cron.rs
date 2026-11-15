//! POSIX five-field cron expressions evaluated in UTC.
//!
//! The fields are `minute hour day-of-month month day-of-week`. Day-of-week
//! numbers weekdays 0 through 7 (0 and 7 are Sunday) and, when both day fields
//! are restricted (neither starts with `*` or `?`), a day matching *either*
//! runs the task; otherwise both must match. The other four fields follow the
//! grammar in [`field`]. Rullst evaluated these fields with the `cron` crate
//! until 13.0.0-alpha.5; this evaluator keeps its accepted syntax, except that
//! each field must now be valid on its own, and its search bounds.

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike, Utc};
use field::{DAY_OF_MONTH, HOUR, MINUTE, MONTH, Values};

#[path = "scheduler_cron_field.rs"]
mod field;

/// The last year searched, as the `cron` crate did.
const LAST_YEAR: i32 = 2100;

/// A validated POSIX five-field schedule.
pub(crate) struct CronSchedule {
    minutes: Values,
    hours: Values,
    days_of_month: Values,
    months: Values,
    /// 0 (Sunday) through 6 (Saturday).
    days_of_week: Values,
    /// Both day fields are restricted, so a day matching either one runs.
    either_day: bool,
}

impl CronSchedule {
    /// Parses `minute hour day-of-month month day-of-week`.
    pub(crate) fn parse(expression: &str) -> Result<Self, String> {
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

    /// Returns the first occurrence strictly after `after`: the first
    /// matching whole minute after it, from 1970 through 2100. As with the
    /// `cron` crate, there is none when the second after `after` falls
    /// before year 0, and an earlier `after` starts the search in 1970.
    pub(crate) fn next_after(&self, after: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        let next_second = after.checked_add_signed(TimeDelta::seconds(1))?;
        if next_second.year() < 0 {
            return None;
        }
        let first = after
            .with_second(0)?
            .with_nanosecond(0)?
            .checked_add_signed(TimeDelta::minutes(1))?
            .max(DateTime::UNIX_EPOCH);
        self.first_from(first.naive_utc())
    }

    /// The first matching minute at or after `from`, a whole minute.
    fn first_from(&self, from: NaiveDateTime) -> Option<DateTime<Utc>> {
        let mut date = from.date();
        let mut earliest = from.time();
        while date.year() <= LAST_YEAR {
            if self.months.contains(date.month()) && self.runs_on(date) {
                if let Some(time) = self.first_time_from(earliest) {
                    return Some(date.and_time(time).and_utc());
                }
            }
            date = date.succ_opt()?;
            earliest = NaiveTime::MIN;
        }
        None
    }

    /// Whether the day fields select `date`.
    fn runs_on(&self, date: NaiveDate) -> bool {
        let day_of_month = self.days_of_month.contains(date.day());
        let day_of_week = self
            .days_of_week
            .contains(date.weekday().num_days_from_sunday());
        if self.either_day {
            day_of_month || day_of_week
        } else {
            day_of_month && day_of_week
        }
    }

    /// The first matching time of day at or after `earliest`.
    fn first_time_from(&self, earliest: NaiveTime) -> Option<NaiveTime> {
        let hour = earliest.hour();
        if self.hours.contains(hour) {
            if let Some(minute) = self.minutes.first_from(earliest.minute()) {
                return NaiveTime::from_hms_opt(hour, minute, 0);
            }
        }
        let hour = self.hours.first_from(hour + 1)?;
        NaiveTime::from_hms_opt(hour, self.minutes.first_from(0)?, 0)
    }
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
#[path = "scheduler_cron_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "scheduler_cron_differential_tests.rs"]
mod differential_tests;
