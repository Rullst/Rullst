//! POSIX five-field cron expressions evaluated in UTC.
//!
//! The fields are `minute hour day-of-month month day-of-week`. Day-of-week
//! numbers weekdays 0 through 7 (0 and 7 are Sunday) and, when both day fields
//! are restricted (neither starts with `*` or `?`), a day matching *either*
//! runs the task; otherwise both must match. The other four fields follow the
//! grammar in [`field`]. Rullst evaluated these fields with the `cron` crate
//! until 13.0.0-alpha.5; this evaluator keeps its accepted syntax, except that
//! each field must now be valid on its own, and its search bounds.

use field::{DAY_OF_MONTH, HOUR, MINUTE, MONTH, Values};
use jiff::Timestamp;
use jiff::civil::{Date, DateTime, Time};
use jiff::tz::Offset;

#[path = "scheduler_cron_field.rs"]
mod field;

/// The last year searched, as the `cron` crate did.
const LAST_YEAR: i16 = 2100;

/// 0000-01-01T00:00:00Z in Unix seconds.
const YEAR_ZERO: i64 = -62_167_219_200;

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
    pub(crate) fn next_after(&self, after: &Timestamp) -> Option<Timestamp> {
        let (second, _) = crate::rfc3339::unix_parts(*after);
        if second + 1 < YEAR_ZERO {
            return None;
        }
        let first = (second.div_euclid(60) + 1) * 60;
        self.first_from(Offset::UTC.to_datetime(Timestamp::from_second(first.max(0)).ok()?))
    }

    /// The first matching minute at or after `from`, a whole minute.
    fn first_from(&self, from: DateTime) -> Option<Timestamp> {
        let mut date = from.date();
        let mut earliest = from.time();
        while date.year() <= LAST_YEAR {
            if self.months.contains(date.month().unsigned_abs().into()) && self.runs_on(date) {
                if let Some(time) = self.first_time_from(earliest) {
                    return Offset::UTC.to_timestamp(date.to_datetime(time)).ok();
                }
            }
            date = date.tomorrow().ok()?;
            earliest = Time::midnight();
        }
        None
    }

    /// Whether the day fields select `date`.
    fn runs_on(&self, date: Date) -> bool {
        let day_of_month = self
            .days_of_month
            .contains(date.day().unsigned_abs().into());
        let day_of_week = self
            .days_of_week
            .contains(date.weekday().to_sunday_zero_offset().unsigned_abs().into());
        if self.either_day {
            day_of_month || day_of_week
        } else {
            day_of_month && day_of_week
        }
    }

    /// The first matching time of day at or after `earliest`.
    fn first_time_from(&self, earliest: Time) -> Option<Time> {
        let hour = u32::from(earliest.hour().unsigned_abs());
        if self.hours.contains(hour) {
            if let Some(minute) = self
                .minutes
                .first_from(earliest.minute().unsigned_abs().into())
            {
                return time(hour, minute);
            }
        }
        let hour = self.hours.first_from(hour + 1)?;
        time(hour, self.minutes.first_from(0)?)
    }
}

/// The time of day `hour:minute:00`.
fn time(hour: u32, minute: u32) -> Option<Time> {
    Time::new(i8::try_from(hour).ok()?, i8::try_from(minute).ok()?, 0, 0).ok()
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
#[path = "scheduler_cron_golden_tests.rs"]
mod golden_tests;
