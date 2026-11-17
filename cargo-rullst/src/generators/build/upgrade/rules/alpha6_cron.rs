//! Messaging schedules adopt Core's POSIX cron syntax in 13.0.0-alpha.6.
//! `RecurringDefinition::new` (`rullst-messaging`, feature
//! `schedules-postgres`) read its five fields with the `cron` crate until
//! alpha.5: weekdays 1 (Sunday) to 7 (Saturday), both day fields
//! intersecting, and a field that starts or ends with `,`, `-` or `/` joined
//! with its neighbour. It now reads them as Core's `Scheduler` does:
//! weekdays 0 to 7 (0 and 7 are Sunday), a day matching either day field
//! when both are restricted, and each field valid on its own. Stored
//! definitions keep their meaning; code that creates one gets the new one.
//!
//! A definition stored by an earlier version keeps its old meaning, and
//! `PostgresRecurringStore::create` refuses to change it in place
//! (`RecurringError::LegacyDefinition`), so the finding asks for a review:
//! keep the expression of a schedule already stored, or cancel it and create
//! the POSIX form under a new name; write POSIX for a new one.
//!
//! [`super::tokens`] reports each `RecurringDefinition::new` call in a file
//! that names Rullst's Messaging, except one whose expression is a string
//! literal that means the same under both syntaxes. For a literal,
//! [`advice`] writes the location's note: the POSIX form with the weekday
//! numbers moved down by one where that is mechanical, or why it is not.

use super::Rule;
use super::catalog::MESSAGING_CRON_POSIX;
use super::tokens::string_value;
use proc_macro2::TokenTree;

/// The rule and the location's note for a call `segments(args)`, if any.
pub(super) fn call_rule(
    segments: &[&str],
    args: &[TokenTree],
    text: &str,
) -> Option<(&'static Rule, Option<String>)> {
    if !segments.ends_with(&["RecurringDefinition", "new"]) || !names_messaging(text) {
        return None;
    }
    let expression = args
        .split(|tree| matches!(tree, TokenTree::Punct(comma) if comma.as_char() == ','))
        .nth(1)
        .and_then(|argument| match argument {
            [literal] => string_value(Some(literal)),
            _ => None,
        });
    match expression {
        None => Some((&MESSAGING_CRON_POSIX, None)),
        Some(expression) => advice(&expression).map(|note| (&MESSAGING_CRON_POSIX, Some(note))),
    }
}

fn names_messaging(text: &str) -> bool {
    text.contains("rullst_messaging") || text.contains("messaging::")
}

/// What `expression`, as the `cron` crate read it, needs under POSIX, or
/// `None` when it means the same: no weekday number, not both day fields
/// restricted, and accepted by Core's evaluator.
pub(super) fn advice(expression: &str) -> Option<String> {
    let fields: Vec<&str> = expression.split_whitespace().collect();
    if fields
        .iter()
        .any(|field| field.starts_with([',', '-', '/']) || field.ends_with([',', '-', '/']))
    {
        return Some(format!(
            "`RecurringDefinition::new` now refuses `{expression}`: a field starts or ends with `,`, `-` or `/`, which the `cron` crate joined with the next one, shifting the fields after it. A schedule already stored with it keeps running, but this call fails: stop creating it here, or cancel it and create a POSIX expression under a new name"
        ));
    }
    let [minute, hour, day, month, weekday] = fields[..] else {
        // The `cron` crate refused it too, at run time.
        return None;
    };
    let both_days = !day.starts_with(['*', '?']) && !weekday.starts_with(['*', '?']);
    let accepted = |candidate: &str| {
        rullst_core::Scheduler::new()
            .task(candidate, || async {})
            .is_ok()
    };
    let posix = weekdays(weekday).map(|weekday| format!("{minute} {hour} {day} {month} {weekday}"));
    let mut notes = Vec::new();
    let normalized = fields.join(" ");
    match posix {
        Some(posix) if !accepted(&posix) && posix == normalized => notes.push(format!(
            "`RecurringDefinition::new` now refuses `{expression}`. A schedule already stored with it keeps running, but this call fails: stop creating it here, or cancel it and create a POSIX expression under a new name"
        )),
        Some(posix) if !accepted(&posix) => notes.push(format!(
            "no mechanical POSIX form: `{posix}`, its weekday numbers moved down by one, is refused; write a schedule not yet stored by hand"
        )),
        None => notes.push(format!(
            "`{expression}` has a weekday number outside 1 to 7, which neither syntax accepts; write the POSIX expression by hand"
        )),
        Some(posix) if posix == normalized => {}
        Some(posix) if both_days => notes.push(format!(
            "its weekdays in POSIX numbers: `{posix}` (Sunday is 0 or 7, Monday 1, Saturday 6)"
        )),
        Some(posix) => notes.push(format!(
            "POSIX form for a schedule not yet stored: `{posix}` (weekday numbers move down by one: Sunday is 0 or 7, Monday 1, Saturday 6)"
        )),
    }
    if both_days {
        notes.insert(0, format!(
            "it restricts both the day of the month (`{day}`) and the weekday (`{weekday}`): a schedule already stored keeps running on days that match both; one created after the upgrade runs on days that match either, so restrict one field and check the other when the message is handled"
        ));
    }
    (!notes.is_empty()).then(|| notes.join("; "))
}

/// The weekday field with each number moved from the `cron` crate's 1
/// (Sunday) to 7 (Saturday) to POSIX's 0 to 6; names, `*` and `?` stay. A
/// single day with a step (`2/3`, from that day to Saturday) gets the range
/// POSIX needs (`1-6/3`). `None` for a number outside 1 to 7.
fn weekdays(field: &str) -> Option<String> {
    let elements = field
        .split(',')
        .map(|element| {
            let (range, step) = match element.split_once('/') {
                Some((range, step)) => (range, Some(step)),
                None => (element, None),
            };
            let range = match range.split_once('-') {
                Some((start, end)) => format!("{}-{}", day(start)?, day(end)?),
                None if step.is_some() && is_number(range) => format!("{}-6", day(range)?),
                None => day(range)?,
            };
            Some(match step {
                Some(step) => format!("{range}/{step}"),
                None => range,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(elements.join(","))
}

/// One weekday: a number 1 to 7 moved down by one, anything else as it is.
fn day(text: &str) -> Option<String> {
    if !is_number(text) {
        return Some(text.to_string());
    }
    match text.parse::<u8>() {
        Ok(number @ 1..=7) => Some((number - 1).to_string()),
        _ => None,
    }
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}
