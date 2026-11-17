//! The five-field UTC cron expressions of recurring schedules.
//!
//! Until 13.0.0-alpha.5 the `cron` crate (0.17) evaluated `0 <expression> *`:
//! a seconds field fixed to `0`, the five calendar fields, and every year. This
//! module evaluates exactly what it evaluated, so stored schedules keep their
//! occurrences. That includes the crate's grammar, in which numbers and names
//! absorb the spaces around them, so whitespace-separated tokens can join one
//! field (`1, 2 * * *`, `1 -2 * * *`; then the appended `*` is the day of the
//! week) or split (`1jan` is a day of the month and a month), which moves the
//! later tokens to other fields: `/5 * * * *` makes the fixed seconds field
//! `0/5`. Its semantics:
//!
//! - fields, in order: second, minute, hour, day of month, month, day of week,
//!   and an optional year (1970 to 2100, every year when absent);
//! - a field is a comma-separated list of `*`, a number, a range `a-b`
//!   (`a <= b`), a name or a range of names (month and day of week only), each
//!   but a single name optionally stepped (`*/15`, `1-30/2`, and `5/15`, from 5
//!   to the field's maximum; a step is 1 to the field's maximum); `?` is `*` in
//!   the day fields;
//! - days of the week are 1 (Sunday) to 7 (Saturday) or English names, and a
//!   day must match both day fields;
//! - an occurrence is the first matching whole second strictly after the given
//!   instant, from 1970 through 2100.

/// The years the crate searched.
const FIRST_YEAR: i64 = 1970;
const LAST_YEAR: i64 = 2100;

/// Milliseconds of `chrono::DateTime::<Utc>::MIN_UTC` and `MAX_UTC`, the
/// range the crate's callers could convert.
const MIN_MILLIS: i64 = -8_334_601_228_800_000;
const MAX_MILLIS: i64 = 8_210_266_876_799_999;

const MS_PER_SECOND: i64 = 1_000;
const SECONDS_PER_DAY: i64 = 86_400;

/// A parsed schedule: the allowed values of each field.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Schedule {
    seconds: Values,
    minutes: Values,
    hours: Values,
    days_of_month: Values,
    months: Values,
    /// 1 (Sunday) to 7 (Saturday).
    days_of_week: Values,
    years: Values,
}

/// Why [`Schedule::next_after`] has no answer.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum OutOfRange {
    /// The instant itself is outside the range the crate accepted.
    Instant,
}

impl Schedule {
    /// Parses `0 <expression> *`, as the crate parsed it. `None` when the
    /// crate rejected it.
    pub(super) fn parse_five_fields(expression: &str) -> Option<Self> {
        parse(&format!("0 {expression} *"))
    }

    /// The first occurrence strictly after `after_ms`, in Unix milliseconds,
    /// or `Ok(None)` when none falls from 1970 through 2100.
    pub(super) fn next_after(&self, after_ms: i64) -> Result<Option<i64>, OutOfRange> {
        if !(MIN_MILLIS..=MAX_MILLIS).contains(&after_ms) {
            return Err(OutOfRange::Instant);
        }
        // The crate searched from the second after `after`, at whole seconds.
        let Some(start) = after_ms
            .div_euclid(MS_PER_SECOND)
            .checked_add(1)
            .filter(|second| *second <= MAX_MILLIS.div_euclid(MS_PER_SECOND))
        else {
            return Ok(None);
        };
        let start_day = start.div_euclid(SECONDS_PER_DAY);
        let (year, _, _) = civil_from_days(start_day);
        // A year before 0 searched nothing; an earlier year started in 1970.
        let (mut day, mut earliest) = if year < 0 {
            return Ok(None);
        } else if year < FIRST_YEAR {
            (days_from_civil(FIRST_YEAR, 1, 1), 0)
        } else {
            (start_day, start.rem_euclid(SECONDS_PER_DAY))
        };
        let last_day = days_from_civil(LAST_YEAR, 12, 31);
        while day <= last_day {
            if self.runs_on(day)
                && let Some(second) = self.first_second_from(earliest)
            {
                return Ok(Some((day * SECONDS_PER_DAY + second) * MS_PER_SECOND));
            }
            day += 1;
            earliest = 0;
        }
        Ok(None)
    }

    /// Whether the year, month and both day fields select `day`.
    fn runs_on(&self, day: i64) -> bool {
        let (year, month, day_of_month) = civil_from_days(day);
        // 1970-01-01 (day 0) was a Thursday, day 5 counted from Sunday as 1.
        let day_of_week = (day + 4).rem_euclid(7) + 1;
        self.years.contains(year)
            && self.months.contains(month)
            && self.days_of_month.contains(day_of_month)
            && self.days_of_week.contains(day_of_week)
    }

    /// The first matching second of the day at or after `earliest`.
    fn first_second_from(&self, earliest: i64) -> Option<i64> {
        let (hour, minute, second) = (earliest / 3_600, earliest / 60 % 60, earliest % 60);
        let at = |hour: i64, minute: i64, second: i64| hour * 3_600 + minute * 60 + second;
        if self.hours.contains(hour) {
            if self.minutes.contains(minute)
                && let Some(second) = self.seconds.first_from(second)
            {
                return Some(at(hour, minute, second));
            }
            if let Some(minute) = self.minutes.first_from(minute + 1) {
                return Some(at(hour, minute, self.seconds.first_from(0)?));
            }
        }
        let hour = self.hours.first_from(hour + 1)?;
        Some(at(
            hour,
            self.minutes.first_from(0)?,
            self.seconds.first_from(0)?,
        ))
    }
}

/// The allowed values of one field, from its minimum to its maximum.
#[derive(Debug, PartialEq, Eq)]
struct Values {
    min: i64,
    allowed: Vec<bool>,
}

impl Values {
    fn contains(&self, value: i64) -> bool {
        value
            .checked_sub(self.min)
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| self.allowed.get(index))
            .is_some_and(|allowed| *allowed)
    }

    /// The smallest allowed value that is at least `value`.
    fn first_from(&self, value: i64) -> Option<i64> {
        let from = usize::try_from(value.saturating_sub(self.min)).unwrap_or(0);
        let offset = self
            .allowed
            .get(from..)?
            .iter()
            .position(|allowed| *allowed)?;
        i64::try_from(from + offset)
            .ok()
            .map(|index| self.min + index)
    }
}

/// One field's bounds and names, as the crate defined them.
struct Unit {
    min: u32,
    max: u32,
    names: &'static [(&'static str, u32)],
}

const SECONDS: Unit = Unit::numbers(0, 59);
const MINUTES: Unit = Unit::numbers(0, 59);
const HOURS: Unit = Unit::numbers(0, 23);
const DAYS_OF_MONTH: Unit = Unit::numbers(1, 31);
const YEARS: Unit = Unit::numbers(1970, 2100);
const MONTHS: Unit = Unit {
    min: 1,
    max: 12,
    names: &[
        ("jan", 1),
        ("january", 1),
        ("feb", 2),
        ("february", 2),
        ("mar", 3),
        ("march", 3),
        ("apr", 4),
        ("april", 4),
        ("may", 5),
        ("jun", 6),
        ("june", 6),
        ("jul", 7),
        ("july", 7),
        ("aug", 8),
        ("august", 8),
        ("sep", 9),
        ("september", 9),
        ("oct", 10),
        ("october", 10),
        ("nov", 11),
        ("november", 11),
        ("dec", 12),
        ("december", 12),
    ],
};
const DAYS_OF_WEEK: Unit = Unit {
    min: 1,
    max: 7,
    names: &[
        ("sun", 1),
        ("sunday", 1),
        ("mon", 2),
        ("monday", 2),
        ("tue", 3),
        ("tues", 3),
        ("tuesday", 3),
        ("wed", 4),
        ("wednesday", 4),
        ("thu", 5),
        ("thurs", 5),
        ("thursday", 5),
        ("fri", 6),
        ("friday", 6),
        ("sat", 7),
        ("saturday", 7),
    ],
};

impl Unit {
    const fn numbers(min: u32, max: u32) -> Self {
        Self {
            min,
            max,
            names: &[],
        }
    }

    /// The values of a parsed field, or `None` when the crate rejected it.
    fn values(&self, field: &[Root<'_>]) -> Option<Values> {
        let mut allowed = vec![false; (self.max - self.min + 1) as usize];
        for root in field {
            for value in self.root(root)? {
                let value = self.checked(value)?;
                if let Some(slot) = allowed.get_mut((value - self.min) as usize) {
                    *slot = true;
                }
            }
        }
        Some(Values {
            min: i64::from(self.min),
            allowed,
        })
    }

    fn root(&self, root: &Root<'_>) -> Option<Vec<u32>> {
        match root {
            Root::Specifier(specifier) => self.specifier(specifier),
            Root::Period(start, step) => {
                if !(1..=self.max).contains(step) {
                    return None;
                }
                let base = match start {
                    // `5/15` runs from 5 to the maximum.
                    Specifier::Point(start) => (self.checked(*start)?..=self.max).collect(),
                    other => self.specifier(other)?,
                };
                Some(base.into_iter().step_by(*step as usize).collect())
            }
            Root::NamedPoint(name) => Some(vec![self.name(name)?]),
        }
    }

    fn specifier(&self, specifier: &Specifier<'_>) -> Option<Vec<u32>> {
        let (start, end) = match specifier {
            Specifier::All => (self.min, self.max),
            Specifier::Point(value) => return Some(vec![*value]),
            Specifier::Range(start, end) => (*start, *end),
            Specifier::NamedRange(start, end) => (self.name(start)?, self.name(end)?),
        };
        let (start, end) = (self.checked(start)?, self.checked(end)?);
        (start <= end).then(|| (start..=end).collect())
    }

    fn checked(&self, value: u32) -> Option<u32> {
        (self.min..=self.max).contains(&value).then_some(value)
    }

    fn name(&self, name: &str) -> Option<u32> {
        let name = name.to_ascii_lowercase();
        self.names
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, value)| *value)
    }
}

/// A list element without its step.
#[derive(Debug, PartialEq, Eq)]
enum Specifier<'a> {
    All,
    Point(u32),
    Range(u32, u32),
    NamedRange(&'a str, &'a str),
}

/// A list element.
#[derive(Debug, PartialEq, Eq)]
enum Root<'a> {
    Specifier(Specifier<'a>),
    Period(Specifier<'a>, u32),
    NamedPoint(&'a str),
}

/// The crate's `longhand` grammar: six fields, an optional year, the end.
fn parse(expression: &str) -> Option<Schedule> {
    let (seconds, rest) = unit_field(expression, false, &SECONDS)?;
    let (minutes, rest) = unit_field(rest, false, &MINUTES)?;
    let (hours, rest) = unit_field(rest, false, &HOURS)?;
    let (days_of_month, rest) = unit_field(rest, true, &DAYS_OF_MONTH)?;
    let (months, rest) = unit_field(rest, false, &MONTHS)?;
    let (days_of_week, rest) = unit_field(rest, true, &DAYS_OF_WEEK)?;
    // An invalid year field is skipped, which leaves its text before the end.
    let (years, rest) = unit_field(rest, false, &YEARS)
        .unwrap_or((YEARS.values(&[Root::Specifier(Specifier::All)])?, rest));
    rest.is_empty().then_some(Schedule {
        seconds,
        minutes,
        hours,
        days_of_month,
        months,
        days_of_week,
        years,
    })
}

fn unit_field<'a>(input: &'a str, any: bool, unit: &Unit) -> Option<(Values, &'a str)> {
    let (field, rest) = field(input, any)?;
    Some((unit.values(&field)?, rest))
}

/// Spaces around a list, then elements separated by commas. A comma not
/// followed by an element ends the list before the comma.
fn field(input: &str, any: bool) -> Option<(Vec<Root<'_>>, &str)> {
    let (first, mut rest) = root(skip_space(input), any)?;
    let mut list = vec![first];
    while let Some((element, after)) = rest
        .strip_prefix(',')
        .and_then(|after_comma| root(after_comma, any))
    {
        list.push(element);
        rest = after;
    }
    Some((list, skip_space(rest)))
}

/// A stepped specifier, a specifier, or a single name.
fn root(input: &str, any: bool) -> Option<(Root<'_>, &str)> {
    let Some((specifier, rest)) = specifier(input, any) else {
        let (name, rest) = name(input)?;
        return Some((Root::NamedPoint(name), rest));
    };
    match rest.strip_prefix('/').and_then(ordinal) {
        Some((step, after)) => Some((Root::Period(specifier, step), after)),
        None => Some((Root::Specifier(specifier), rest)),
    }
}

/// `?` (day fields), `*`, a range, a number or a range of names.
fn specifier(input: &str, any: bool) -> Option<(Specifier<'_>, &str)> {
    if let Some(rest) = input
        .strip_prefix('*')
        .or_else(|| input.strip_prefix('?').filter(|_| any))
    {
        return Some((Specifier::All, rest));
    }
    if let Some((start, rest)) = ordinal(input) {
        return Some(match rest.strip_prefix('-').and_then(ordinal) {
            Some((end, after)) => (Specifier::Range(start, end), after),
            None => (Specifier::Point(start), rest),
        });
    }
    let (start, rest) = name(input)?;
    let (end, rest) = name(rest.strip_prefix('-')?)?;
    Some((Specifier::NamedRange(start, end), rest))
}

/// ASCII digits that fit a `u32`, with the spaces around them.
fn ordinal(input: &str) -> Option<(u32, &str)> {
    let (digits, rest) = token(input, |byte| byte.is_ascii_digit())?;
    Some((digits.parse().ok()?, rest))
}

/// ASCII letters, with the spaces around them.
fn name(input: &str) -> Option<(&str, &str)> {
    token(input, |byte| byte.is_ascii_alphabetic())
}

fn token(input: &str, accept: impl Fn(u8) -> bool) -> Option<(&str, &str)> {
    let input = skip_space(input);
    let end = input
        .bytes()
        .position(|byte| !accept(byte))
        .unwrap_or(input.len());
    (end > 0).then(|| (&input[..end], skip_space(&input[end..])))
}

fn skip_space(input: &str) -> &str {
    input.trim_start_matches([' ', '\t', '\r', '\n'])
}

/// Days since 1970-01-01 of a proleptic Gregorian date.
pub(super) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The proleptic Gregorian date `days` after 1970-01-01.
pub(super) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
#[path = "cron_expression_tests.rs"]
mod tests;
