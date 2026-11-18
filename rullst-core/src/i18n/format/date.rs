//! Dates and times from a locale's [`DateData`] patterns.

use super::{DateData, DateStyle, TimeStyle};
use jiff::civil::{Date, DateTime, Time};
use std::fmt::Write as _;

/// The calendar and clock fields a pattern may use.
#[derive(Default)]
struct Fields {
    year: i32,
    month: u32,
    day: u32,
    /// 0 for Monday.
    weekday: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

impl Fields {
    fn date(value: Date) -> Self {
        Self {
            year: value.year().into(),
            month: value.month().unsigned_abs().into(),
            day: value.day().unsigned_abs().into(),
            weekday: value
                .weekday()
                .to_monday_zero_offset()
                .unsigned_abs()
                .into(),
            ..Self::default()
        }
    }

    fn with_time(self, value: Time) -> Self {
        Self {
            hour: value.hour().unsigned_abs().into(),
            minute: value.minute().unsigned_abs().into(),
            second: value.second().unsigned_abs().into(),
            ..self
        }
    }
}

/// The `index`th of `|`-separated names (0-based).
fn name(names: &'static str, index: u32) -> &'static str {
    let index = usize::try_from(index).unwrap_or(usize::MAX);
    names.split('|').nth(index).unwrap_or_default()
}

/// Writes one CLDR field.
fn field(out: &mut String, token: &str, data: &DateData, fields: &Fields) {
    let hour12 = match fields.hour % 12 {
        0 => 12,
        hour => hour,
    };
    let _ = match token {
        "d" => write!(out, "{}", fields.day),
        "dd" => write!(out, "{:02}", fields.day),
        "M" => write!(out, "{}", fields.month),
        "MM" => write!(out, "{:02}", fields.month),
        "MMM" => write!(out, "{}", name(data.months, fields.month.saturating_sub(1))),
        "MMMM" => write!(
            out,
            "{}",
            name(data.months, fields.month.saturating_sub(1) + 12)
        ),
        "y" => write!(out, "{}", fields.year),
        "yy" => write!(out, "{:02}", fields.year.rem_euclid(100)),
        "EEEE" => write!(out, "{}", name(data.weekdays, fields.weekday)),
        "h" => write!(out, "{hour12}"),
        "H" => write!(out, "{}", fields.hour),
        "HH" => write!(out, "{:02}", fields.hour),
        "mm" => write!(out, "{:02}", fields.minute),
        "ss" => write!(out, "{:02}", fields.second),
        "a" => {
            let [am, pm] = data.day_periods;
            write!(out, "{}", if fields.hour < 12 { am } else { pm })
        }
        _ => Ok(()),
    };
}

/// Expands `{field}`s in `pattern`; the rest is literal text.
fn render(pattern: &str, data: &DateData, fields: &Fields) -> String {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some((literal, after)) = rest.split_once('{') {
        out.push_str(literal);
        let Some((token, tail)) = after.split_once('}') else {
            rest = after;
            break;
        };
        field(&mut out, token, data, fields);
        rest = tail;
    }
    out.push_str(rest);
    out
}

fn date_pattern(data: &DateData, style: DateStyle) -> &'static str {
    let [short, medium, long, full, numeric] = data.date;
    match style {
        DateStyle::Short => short,
        DateStyle::Medium => medium,
        DateStyle::Long => long,
        DateStyle::Full => full,
        DateStyle::Numeric => numeric,
    }
}

fn time_pattern(data: &DateData, style: TimeStyle) -> &'static str {
    let [short, medium] = data.time;
    match style {
        TimeStyle::Short => short,
        TimeStyle::Medium => medium,
    }
}

pub(super) fn date(data: &DateData, value: Date, style: DateStyle) -> String {
    render(date_pattern(data, style), data, &Fields::date(value))
}

pub(super) fn time(data: &DateData, value: Time, style: TimeStyle) -> String {
    let fields = Fields::default().with_time(value);
    render(time_pattern(data, style), data, &fields)
}

pub(super) fn datetime(
    data: &DateData,
    value: DateTime,
    date_style: DateStyle,
    time_style: TimeStyle,
) -> String {
    let fields = Fields::date(value.date()).with_time(value.time());
    let date = render(date_pattern(data, date_style), data, &fields);
    let time = render(time_pattern(data, time_style), data, &fields);
    let [short, medium, long, full] = data.glue;
    let glue = match date_style {
        DateStyle::Short | DateStyle::Numeric => short,
        DateStyle::Medium => medium,
        DateStyle::Long => long,
        DateStyle::Full => full,
    };
    let mut out = String::new();
    let mut rest = glue;
    while let Some((literal, after)) = rest.split_once('{') {
        out.push_str(literal);
        let (part, tail) = after.split_once('}').unwrap_or((after, ""));
        out.push_str(if part == "1" { &date } else { &time });
        rest = tail;
    }
    out.push_str(rest);
    out
}
