//! POSIX five-field cron expressions evaluated in UTC.
//!
//! The `cron` crate numbers weekdays 1 (Sunday) through 7 (Saturday), rejects
//! 0 and always intersects day-of-month with day-of-week. POSIX cron numbers
//! weekdays 0 through 7 (0 and 7 are Sunday) and, when both day fields are
//! restricted, runs on a day matching *either*. This module translates the
//! day-of-week field and implements that rule over two crate schedules.

use chrono::{DateTime, Utc};

/// A validated POSIX five-field schedule.
pub(crate) struct CronSchedule {
    day_of_month_or_all: cron::Schedule,
    /// Present only when both day fields are restricted (POSIX OR rule).
    day_of_week: Option<cron::Schedule>,
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
    pub(crate) fn next_after(&self, after: &DateTime<Utc>) -> Option<DateTime<Utc>> {
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

    /// Every second, for deterministic scheduler-loop tests.
    #[cfg(test)]
    pub(crate) fn every_second() -> Self {
        Self {
            day_of_month_or_all: "* * * * * * *"
                .parse()
                .unwrap_or_else(|error| panic!("valid test schedule: {error}")),
            day_of_week: None,
        }
    }
}

/// Translates a POSIX day-of-week field into the crate's 1 (Sunday) to
/// 7 (Saturday) numbering as an explicit list.
fn crate_day_of_week(field: &str) -> Result<String, String> {
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone, Weekday};

    fn next_days(expression: &str, count: usize) -> Vec<(Weekday, u32)> {
        let schedule = CronSchedule::parse(expression).unwrap();
        // Wednesday 2026-09-30 00:00 UTC.
        let mut cursor = Utc.with_ymd_and_hms(2026, 9, 30, 0, 0, 0).unwrap();
        let mut days = Vec::new();
        for _ in 0..count {
            cursor = schedule.next_after(&cursor).unwrap();
            days.push((cursor.weekday(), cursor.day()));
        }
        days
    }

    fn weekdays(expression: &str, count: usize) -> Vec<Weekday> {
        next_days(expression, count)
            .into_iter()
            .map(|(weekday, _)| weekday)
            .collect()
    }

    #[test]
    fn numeric_weekdays_follow_posix_numbering() {
        use Weekday::*;
        assert_eq!(
            weekdays("0 9 * * 1-5", 7),
            [Wed, Thu, Fri, Mon, Tue, Wed, Thu]
        );
        assert_eq!(weekdays("0 3 * * 0", 2), [Sun, Sun]);
        assert_eq!(weekdays("0 3 * * 7", 2), [Sun, Sun]);
        assert_eq!(weekdays("0 3 * * 6", 1), [Sat]);
        assert_eq!(weekdays("0 3 * * 5-7", 3), [Fri, Sat, Sun]);
        assert_eq!(weekdays("0 3 * * FRI-SUN", 3), [Fri, Sat, Sun]);
        assert_eq!(weekdays("0 3 * * 1,3,sat", 4), [Wed, Sat, Mon, Wed]);
        assert_eq!(weekdays("0 3 * * 1-5/2", 4), [Wed, Fri, Mon, Wed]);
        assert_eq!(weekdays("0 3 * * */3", 3), [Wed, Sat, Sun]);
        assert_eq!(weekdays("0 3 * * mon-fri", 3), [Wed, Thu, Fri]);
    }

    #[test]
    fn restricted_day_fields_match_either_day() {
        // POSIX: the 1st of the month OR every Monday.
        let days = next_days("0 0 1 * 1", 4);
        assert_eq!(
            days,
            [
                (Weekday::Thu, 1),
                (Weekday::Mon, 5),
                (Weekday::Mon, 12),
                (Weekday::Mon, 19)
            ]
        );
        // A star-prefixed day field keeps the intersection (Vixie cron).
        let days = next_days("0 0 */2 * 1", 2);
        assert!(
            days.iter()
                .all(|(weekday, day)| *weekday == Weekday::Mon && day % 2 == 1)
        );
    }

    #[test]
    fn schedules_are_evaluated_in_utc() {
        let schedule = CronSchedule::parse("30 23 * * *").unwrap();
        let start = Utc.with_ymd_and_hms(2026, 9, 30, 12, 0, 0).unwrap();
        assert_eq!(
            schedule.next_after(&start).unwrap(),
            Utc.with_ymd_and_hms(2026, 9, 30, 23, 30, 0).unwrap()
        );
    }

    #[test]
    fn malformed_expressions_are_rejected() {
        for expression in [
            "* * * *",
            "* * * * * *",
            "* * * * 8",
            "* * * * 5-1",
            "* * * * 1/2",
            "* * * * */0",
            "* * * * funday",
            "* * * * 1-",
            "61 * * * *",
        ] {
            assert!(
                CronSchedule::parse(expression).is_err(),
                "accepted `{expression}`"
            );
        }
    }
}
