//! Bounded server-side validation for registered Nexus form metadata.

use std::collections::BTreeMap;
use std::fmt;

use crate::nexus::types::{FieldKind, FieldMeta, RegistryEntry};

const MAX_FORM_PAIRS: usize = 256;
const MAX_SHORT_TEXT_BYTES: usize = 4 * 1024;
const MAX_LONG_TEXT_BYTES: usize = 64 * 1024;
const MAX_EMAIL_BYTES: usize = 320;
const MAX_URL_BYTES: usize = 2 * 1024;
/// The body field Core's CSRF middleware reads when no header is sent.
const CSRF_FORM_FIELD: &str = "_token";

#[derive(Clone, Copy)]
pub(super) enum FormMode {
    Create,
    Update,
}

pub(super) struct ValidatedFieldValue<'a> {
    pub(super) field: &'a FieldMeta,
    /// `None` is SQL NULL: an emptied field whose kind cannot store `''`.
    pub(super) value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FormInputError {
    TooManyFields,
    UnknownOrProtectedField,
    DuplicateField {
        field: &'static str,
    },
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
}

impl fmt::Display for FormInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyFields => formatter.write_str("the form contains too many values"),
            Self::UnknownOrProtectedField => {
                formatter.write_str("the form contains an unknown or protected field")
            }
            Self::DuplicateField { field } => {
                write!(formatter, "field `{field}` was submitted more than once")
            }
            Self::InvalidField { field, reason } => {
                write!(formatter, "field `{field}` {reason}")
            }
        }
    }
}

pub(super) fn validate_form_values<'a>(
    entry: &'a RegistryEntry,
    pairs: Vec<(String, String)>,
    mode: FormMode,
) -> Result<Vec<ValidatedFieldValue<'a>>, FormInputError> {
    if pairs.len() > MAX_FORM_PAIRS {
        return Err(FormInputError::TooManyFields);
    }

    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    for (name, value) in pairs {
        // Core's CSRF middleware accepts the double-submit token in the form
        // body and has already verified it; it is not model data.
        if name == CSRF_FORM_FIELD {
            continue;
        }
        let Some(field) = entry.fields.iter().find(|field| field.name == name) else {
            return Err(FormInputError::UnknownOrProtectedField);
        };
        let protected = field.hidden
            || field.readonly
            || matches!(mode, FormMode::Update) && field.name == entry.pk;
        if protected {
            return Err(FormInputError::UnknownOrProtectedField);
        }
        grouped.entry(name).or_default().push(value);
    }

    let mut values = entry
        .fields
        .iter()
        .filter_map(|field| {
            grouped
                .remove(field.name)
                .map(|values| normalize_values(field, values))
        })
        .collect::<Result<Vec<_>, _>>()?;
    values.retain(|value| match value.value.as_deref() {
        // The form never receives a stored Password value, so an empty
        // Password input means "keep the current value".
        Some("") => !matches!(value.field.kind, FieldKind::Password),
        // A new record omits an emptied typed field so the column default
        // applies; an update stores NULL.
        None => matches!(mode, FormMode::Update),
        Some(_) => true,
    });
    Ok(values)
}

/// Kinds whose empty input is an empty string. For every other kind `''` is
/// not a value (not a number, date, option or JSON document) and means NULL.
fn stores_empty_text(kind: &FieldKind) -> bool {
    matches!(
        kind,
        FieldKind::Text
            | FieldKind::Textarea
            | FieldKind::Email
            | FieldKind::Url
            | FieldKind::Password
    )
}

fn normalize_values<'a>(
    field: &'a FieldMeta,
    values: Vec<String>,
) -> Result<ValidatedFieldValue<'a>, FormInputError> {
    let value = if matches!(field.kind, FieldKind::Boolean) {
        normalize_boolean_values(field.name, &values)?
    } else {
        if values.len() != 1 {
            return Err(FormInputError::DuplicateField { field: field.name });
        }
        values
            .into_iter()
            .next()
            .ok_or(FormInputError::InvalidField {
                field: field.name,
                reason: "has no value",
            })?
    };
    if value.is_empty() && !stores_empty_text(&field.kind) {
        return Ok(ValidatedFieldValue { field, value: None });
    }
    validate_semantic_value(field, &value)?;
    Ok(ValidatedFieldValue {
        field,
        value: Some(value),
    })
}

fn normalize_boolean_values(
    field: &'static str,
    values: &[String],
) -> Result<String, FormInputError> {
    let normalized = values
        .iter()
        .map(|value| parse_boolean(field, value))
        .collect::<Result<Vec<_>, _>>()?;
    let value = match normalized.as_slice() {
        [value] => *value,
        [false, true] => true,
        _ => return Err(FormInputError::DuplicateField { field }),
    };
    Ok(if value { "1" } else { "0" }.to_string())
}

fn parse_boolean(field: &'static str, value: &str) -> Result<bool, FormInputError> {
    if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("on") || value == "1" {
        Ok(true)
    } else if value.eq_ignore_ascii_case("false")
        || value.eq_ignore_ascii_case("off")
        || value == "0"
    {
        Ok(false)
    } else {
        Err(FormInputError::InvalidField {
            field,
            reason: "must be a Boolean value",
        })
    }
}

fn validate_semantic_value(field: &FieldMeta, value: &str) -> Result<(), FormInputError> {
    let limit = match field.kind {
        FieldKind::Textarea | FieldKind::Json => MAX_LONG_TEXT_BYTES,
        FieldKind::Email => MAX_EMAIL_BYTES,
        FieldKind::Url => MAX_URL_BYTES,
        _ => MAX_SHORT_TEXT_BYTES,
    };
    if value.len() > limit {
        return invalid(field, "exceeds the bounded input size");
    }
    let multiline = matches!(field.kind, FieldKind::Textarea | FieldKind::Json);
    if value.chars().any(|character| {
        character.is_control() && !(multiline && matches!(character, '\n' | '\r' | '\t'))
    }) {
        return invalid(field, "contains a forbidden control character");
    }
    if value.is_empty() && stores_empty_text(&field.kind) {
        return Ok(());
    }

    match &field.kind {
        FieldKind::Email => validate_email(field, value),
        FieldKind::Url => validate_url(field, value),
        FieldKind::Number => match value.parse::<f64>() {
            Ok(number) if number.is_finite() => Ok(()),
            _ => invalid(field, "must be a finite number"),
        },
        FieldKind::Boolean => {
            if matches!(value, "0" | "1") {
                Ok(())
            } else {
                invalid(field, "must be a normalized Boolean value")
            }
        }
        FieldKind::Date => validate_date(field, value),
        FieldKind::DateTime => validate_datetime(field, value),
        FieldKind::Json => serde_json::from_str::<serde_json::Value>(value)
            .map(|_| ())
            .map_err(|_| FormInputError::InvalidField {
                field: field.name,
                reason: "must contain valid JSON",
            }),
        FieldKind::Enum { options } => {
            if options.contains(&value) {
                Ok(())
            } else {
                invalid(field, "is not one of the registered enum options")
            }
        }
        FieldKind::Text
        | FieldKind::Textarea
        | FieldKind::Password
        | FieldKind::ForeignKey { .. } => Ok(()),
    }
}

fn validate_email(field: &FieldMeta, value: &str) -> Result<(), FormInputError> {
    let mut parts = value.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    if !local.is_empty()
        && !domain.is_empty()
        && parts.next().is_none()
        && !value.chars().any(char::is_whitespace)
    {
        Ok(())
    } else {
        invalid(field, "must contain a bounded e-mail address")
    }
}

fn validate_url(field: &FieldMeta, value: &str) -> Result<(), FormInputError> {
    let parsed = url::Url::parse(value).map_err(|_| FormInputError::InvalidField {
        field: field.name,
        reason: "must contain an absolute HTTP(S) URL",
    })?;
    if matches!(parsed.scheme(), "http" | "https")
        && parsed.host().is_some()
        && parsed.username().is_empty()
        && parsed.password().is_none()
    {
        Ok(())
    } else {
        invalid(
            field,
            "must contain an absolute HTTP(S) URL without credentials",
        )
    }
}

fn validate_date(field: &FieldMeta, value: &str) -> Result<(), FormInputError> {
    let mut parts = value.split('-');
    let year = parse_date_part(parts.next(), 4);
    let month = parse_date_part(parts.next(), 2);
    let day = parse_date_part(parts.next(), 2);
    let valid = match (year, month, day, parts.next()) {
        (Some(year), Some(month @ 1..=12), Some(day), None) => {
            (1..=days_in_month(year, month)).contains(&day)
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        invalid(field, "must use a valid YYYY-MM-DD date")
    }
}

fn validate_datetime(field: &FieldMeta, value: &str) -> Result<(), FormInputError> {
    let Some((date, time)) = value.split_once('T') else {
        return invalid(field, "must use a valid local date-time");
    };
    validate_date(field, date)?;
    // An RFC 3339 offset (`Z`, `+HH:MM`, `-HH:MM`) is accepted so a stored
    // offset value shown in a text input can be edited and saved back.
    let (time, offset_valid) = if let Some(local) = time.strip_suffix(['Z', 'z']) {
        (local, true)
    } else if let Some(index) = time.rfind(['+', '-']) {
        let (local, offset) = time.split_at(index);
        (local, valid_utc_offset(&offset[1..]))
    } else {
        (time, true)
    };
    if !offset_valid {
        return invalid(field, "must use a valid date-time offset");
    }
    let mut parts = time.split(':');
    let hour = parse_date_part(parts.next(), 2);
    let minute = parse_date_part(parts.next(), 2);
    let seconds = parts.next();
    let seconds_valid = seconds.is_none_or(|part| {
        let (whole, fraction) = part
            .split_once('.')
            .map_or((part, None), |(whole, fraction)| (whole, Some(fraction)));
        parse_date_part(Some(whole), 2).is_some_and(|second| second <= 59)
            && fraction.is_none_or(|digits| {
                !digits.is_empty()
                    && digits.len() <= 9
                    && digits.bytes().all(|byte| byte.is_ascii_digit())
            })
    });
    if matches!(hour, Some(0..=23))
        && matches!(minute, Some(0..=59))
        && seconds_valid
        && parts.next().is_none()
    {
        Ok(())
    } else {
        invalid(field, "must use a valid local date-time")
    }
}

fn valid_utc_offset(offset: &str) -> bool {
    let mut parts = offset.split(':');
    matches!(parse_date_part(parts.next(), 2), Some(0..=23))
        && matches!(parse_date_part(parts.next(), 2), Some(0..=59))
        && parts.next().is_none()
}

/// True when `value` is a valid `YYYY-MM-DD` date that a date input shows.
pub(super) fn is_local_date(value: &str) -> bool {
    validate_date(&FieldMeta::new("date", "Date", FieldKind::Date), value).is_ok()
}

/// Returns the `datetime-local` form of `value` when that input can show it
/// exactly: no offset and at most millisecond precision. A space separator is
/// normalized to `T`. Other values must be shown as text so the browser does
/// not silently blank them.
pub(super) fn datetime_local_value(value: &str) -> Option<String> {
    let normalized = value.replacen(' ', "T", 1);
    let (_, time) = normalized.split_once('T')?;
    let local = !time.contains(['Z', 'z', '+', '-'])
        && time
            .split_once('.')
            .is_none_or(|(_, fraction)| fraction.len() <= 3);
    let field = FieldMeta::new("datetime", "Date-time", FieldKind::DateTime);
    (local && validate_datetime(&field, &normalized).is_ok()).then_some(normalized)
}

fn parse_date_part(value: Option<&str>, width: usize) -> Option<u32> {
    let value = value?;
    if value.len() == width && value.bytes().all(|byte| byte.is_ascii_digit()) {
        value.parse().ok()
    } else {
        None
    }
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || year.is_multiple_of(4) && !year.is_multiple_of(100) => 29,
        2 => 28,
        _ => 31,
    }
}

fn invalid<T>(field: &FieldMeta, reason: &'static str) -> Result<T, FormInputError> {
    Err(FormInputError::InvalidField {
        field: field.name,
        reason,
    })
}

#[cfg(test)]
mod tests;
