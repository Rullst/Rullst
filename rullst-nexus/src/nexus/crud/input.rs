//! Bounded server-side validation for registered Nexus form metadata.

use std::collections::BTreeMap;
use std::fmt;

use crate::nexus::types::{FieldKind, FieldMeta, RegistryEntry};

const MAX_FORM_PAIRS: usize = 256;
const MAX_SHORT_TEXT_BYTES: usize = 4 * 1024;
const MAX_LONG_TEXT_BYTES: usize = 64 * 1024;
const MAX_EMAIL_BYTES: usize = 320;
const MAX_URL_BYTES: usize = 2 * 1024;

#[derive(Clone, Copy)]
pub(super) enum FormMode {
    Create,
    Update,
}

pub(super) struct ValidatedFieldValue<'a> {
    pub(super) field: &'a FieldMeta,
    pub(super) value: String,
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

    entry
        .fields
        .iter()
        .filter_map(|field| {
            grouped
                .remove(field.name)
                .map(|values| normalize_values(field, values))
        })
        .collect()
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
    validate_semantic_value(field, &value)?;
    Ok(ValidatedFieldValue { field, value })
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
    if value.is_empty() {
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
mod tests {
    use super::*;

    fn entry() -> RegistryEntry {
        RegistryEntry {
            table: "articles",
            label: "Articles",
            icon: "A",
            pk: "id",
            fields: vec![
                FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
                FieldMeta::new("body", "Body", FieldKind::Textarea),
                FieldMeta::new("active", "Active", FieldKind::Boolean),
                FieldMeta::new(
                    "status",
                    "Status",
                    FieldKind::Enum {
                        options: vec!["draft", "published"],
                    },
                ),
                FieldMeta::new("metadata", "Metadata", FieldKind::Json),
            ],
        }
    }

    #[test]
    fn validates_and_normalizes_semantic_widgets() {
        let entry = entry();
        let values = validate_form_values(
            &entry,
            vec![
                ("body".to_string(), "first\nsecond".to_string()),
                ("active".to_string(), "0".to_string()),
                ("active".to_string(), "1".to_string()),
                ("status".to_string(), "published".to_string()),
                ("metadata".to_string(), "{\"safe\":true}".to_string()),
            ],
            FormMode::Create,
        )
        .expect("valid registered form");

        assert_eq!(values.len(), 4);
        assert_eq!(values[1].value, "1");
        assert_eq!(values[2].value, "published");
    }

    #[test]
    fn rejects_unregistered_enum_duplicate_and_protected_fields() {
        let entry = entry();
        for pairs in [
            vec![("status".to_string(), "administrator".to_string())],
            vec![
                ("status".to_string(), "draft".to_string()),
                ("status".to_string(), "published".to_string()),
            ],
            vec![("id".to_string(), "7".to_string())],
            vec![("unknown".to_string(), "value".to_string())],
        ] {
            assert!(validate_form_values(&entry, pairs, FormMode::Update).is_err());
        }
    }

    #[test]
    fn rejects_oversized_text_invalid_json_and_malformed_boolean() {
        let entry = entry();
        for pairs in [
            vec![("body".to_string(), "x".repeat(MAX_LONG_TEXT_BYTES + 1))],
            vec![("metadata".to_string(), "{invalid".to_string())],
            vec![("active".to_string(), "yes".to_string())],
        ] {
            assert!(validate_form_values(&entry, pairs, FormMode::Create).is_err());
        }
    }
}
