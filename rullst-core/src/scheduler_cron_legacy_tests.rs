//! The scheduler's cron evaluation as it was in 13.0.0-alpha.5, on the `cron`
//! crate, kept unchanged as the reference of the differential tests.

use chrono::{DateTime, Utc};

/// A validated POSIX five-field schedule.
pub(super) struct CronSchedule {
    day_of_month_or_all: cron::Schedule,
    /// Present only when both day fields are restricted (POSIX OR rule).
    day_of_week: Option<cron::Schedule>,
}

impl CronSchedule {
    /// Parses `minute hour day-of-month month day-of-week`.
    pub(super) fn parse(expression: &str) -> Result<Self, String> {
        let fields: Vec<&str> = expression.split_whitespace().collect();
        let [minute, hour, day_of_month, month, day_of_week] = fields[..] else {
            return Err(
                "expected five fields: minute hour day-of-month month day-of-week".to_string(),
            );
        };
        let weekdays = crate_day_of_week(day_of_week)?;
        let unrestricted = |field: &str| field.starts_with(['*', '?']);
        let parse = |day_of_month: &str, day_of_week: &str| {
            format!("0 {minute} {hour} {day_of_month} {month} {day_of_week} *")
                .parse::<cron::Schedule>()
                .map_err(|error| error.to_string())
        };

        if unrestricted(day_of_month) || unrestricted(day_of_week) {
            // One day field is unrestricted, so the intersection that the
            // crate computes is exactly the POSIX meaning.
            return Ok(Self {
                day_of_month_or_all: parse(day_of_month, &weekdays)?,
                day_of_week: None,
            });
        }
        Ok(Self {
            day_of_month_or_all: parse(day_of_month, "*")?,
            day_of_week: Some(parse("*", &weekdays)?),
        })
    }

    /// Returns the first occurrence strictly after `after`.
    pub(super) fn next_after(&self, after: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        let first = self.day_of_month_or_all.after(after).next();
        let second = self
            .day_of_week
            .as_ref()
            .and_then(|schedule| schedule.after(after).next());
        match (first, second) {
            (Some(first), Some(second)) => Some(first.min(second)),
            (first, second) => first.or(second),
        }
    }
}

/// Translates a POSIX day-of-week field into the crate's 1 (Sunday) to
/// 7 (Saturday) numbering as an explicit list.
pub(super) fn crate_day_of_week(field: &str) -> Result<String, String> {
    if field.starts_with(['*', '?']) && !field.contains(',') {
        let step = match field.get(1..) {
            Some("") => 1,
            Some(rest) => parse_step(rest.strip_prefix('/').ok_or_else(|| invalid(field))?)?,
            None => return Err(invalid(field)),
        };
        return Ok(join((0..=6).step_by(step)));
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
    Ok(join(
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

/// Emits POSIX days 0 (Sunday) to 6 as crate ordinals 1 to 7.
fn join(days: impl Iterator<Item = usize>) -> String {
    days.map(|day| (day + 1).to_string())
        .collect::<Vec<_>>()
        .join(",")
}
