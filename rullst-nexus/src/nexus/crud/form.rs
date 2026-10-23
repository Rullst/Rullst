//! Create/edit record form rendering.
//!
//! The edit form must round-trip every value it does not change. It therefore
//! distinguishes SQL NULL and undecodable values from real ones, never shows a
//! value in a widget that would silently blank or rewrite it, and marks itself
//! `data-nexus-mode="edit"` so `nexus.js` submits only the controls the
//! administrator changed.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::nexus::NexusTimeZone;
use crate::nexus::crud::dialect::{placeholder, select_column, tenant_predicate};
use crate::nexus::crud::input::{
    datetime_local_value, is_local_date, time_input_value, zoned_input_value,
};
use crate::nexus::crud::key::{RecordKey, key_predicate};
use crate::nexus::crud::query::sanitize_identifier;
use crate::nexus::types::{FieldKind, FieldMeta, NexusState, RegistryEntry};
use rullst_core::html::escape_str;
use std::fmt::Write as _;

/// What the edit form knows about one stored column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StoredValue {
    /// No stored row (create form) or a value that is never read (`Password`).
    Absent,
    /// The column is SQL NULL.
    Null,
    /// The column holds a value this field kind cannot decode.
    Unreadable,
    /// The decoded value, as text.
    Value(String),
}

type StoredRow = <rullst_orm::RullstDatabase as rullst_orm::_sqlx::Database>::Row;

/// Why an edit form could not be rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordFormError {
    /// No record of this model (and tenant) has that key.
    NotFound,
    /// The database is not configured or the query failed.
    Unavailable,
}

impl RecordFormError {
    fn message(self) -> &'static str {
        match self {
            Self::NotFound => "Record not found.",
            Self::Unavailable => "The record could not be loaded.",
        }
    }
}

impl IntoResponse for RecordFormError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unavailable => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.message()).into_response()
    }
}

/// Renders HTML form for creating or editing records in the modal dialog.
///
/// An edit form for a record that does not exist or cannot be loaded renders
/// a short error message instead of an empty, editable form. `DateTimeUtc`
/// values are shown in UTC; the panel's own routes use the zone selected with
/// [`Nexus::with_time_zone`](crate::nexus::Nexus::with_time_zone).
#[cfg_attr(mutants, mutants::skip)]
pub async fn render_record_form(
    state: &NexusState,
    entry: &RegistryEntry,
    record_id: Option<&str>,
    tenant_id: Option<&str>,
) -> String {
    record_form(state, entry, record_id, tenant_id, &NexusTimeZone::utc())
        .await
        .unwrap_or_else(|error| {
            format!(
                "<p class=\"nexus-error\">{}</p>",
                escape_str(error.message())
            )
        })
}

/// Loads the columns the edit form shows: registered fields that are neither
/// hidden nor `Password`, so undeclared columns and stored secrets are never
/// read.
async fn load_record(
    entry: &RegistryEntry,
    id: &str,
    tenant_id: Option<&str>,
) -> Result<StoredRow, RecordFormError> {
    let key = RecordKey::parse(entry, id).ok_or(RecordFormError::NotFound)?;
    let pool = rullst_core::db::safe_pool().ok_or(RecordFormError::Unavailable)?;
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let sql = record_sql(entry, tenant_id.is_some(), driver);
    let mut query = key.bind(rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(
        sql.as_str(),
    )));
    if entry.tenant_column.is_some()
        && let Some(tenant_id) = tenant_id
    {
        query = query.bind(tenant_id.to_owned());
    }
    match query.fetch_optional(pool).await {
        Ok(Some(row)) => Ok(row),
        Ok(None) => Err(RecordFormError::NotFound),
        Err(_) => {
            tracing::error!(table = entry.table, "Nexus record query failed");
            Err(RecordFormError::Unavailable)
        }
    }
}

/// The query that loads one record by its key; the key values are bound
/// first, then the tenant. A tenant-scoped model without a tenant matches
/// nothing.
pub(super) fn record_sql(entry: &RegistryEntry, tenant_scoped: bool, driver: &str) -> String {
    let columns = entry
        .fields
        .iter()
        .filter(|field| !field.hidden && !matches!(field.kind, FieldKind::Password))
        .map(|field| select_column(field, driver))
        .collect::<Vec<_>>();
    let select_list = if columns.is_empty() {
        entry
            .key_columns()
            .iter()
            .map(|column| sanitize_identifier(column))
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        columns.join(", ")
    };
    let key_width = entry.key_columns().len();
    let tenant_predicate = match (entry.tenant_column, tenant_scoped) {
        (Some(column), true) => format!(
            " AND {}",
            tenant_predicate(column, &placeholder(key_width + 1, driver), driver)
        ),
        (Some(_), false) => " AND 1 = 0".to_string(),
        (None, _) => String::new(),
    };
    format!(
        "SELECT {select_list} FROM {} WHERE {}{tenant_predicate} LIMIT 1",
        sanitize_identifier(entry.table),
        key_predicate(entry, 1, driver),
    )
}

/// Renders the create form (`record_id` is `None`) or the edit form for an
/// existing record, with `DateTimeUtc` values as wall times in `zone`.
pub(crate) async fn record_form(
    _state: &NexusState,
    entry: &RegistryEntry,
    record_id: Option<&str>,
    tenant_id: Option<&str>,
    zone: &NexusTimeZone,
) -> Result<String, RecordFormError> {
    let row = match record_id {
        Some(id) => Some(load_record(entry, id, tenant_id).await?),
        None => None,
    };
    Ok(form_html(entry, record_id, row.as_ref(), zone))
}

/// Renders the form for `record_id` from its loaded `row`.
pub(super) fn form_html(
    entry: &RegistryEntry,
    record_id: Option<&str>,
    row_data: Option<&StoredRow>,
    zone: &NexusTimeZone,
) -> String {
    let is_edit = record_id.is_some();
    let title = if is_edit {
        format!("Edit {}", entry.label)
    } else {
        format!("New {}", entry.label)
    };

    let t = entry.table;

    use rullst_orm::_sqlx::{Row, ValueRef};

    let fields_html =
        entry
            .fields
            .iter()
            .filter(|field| !field.hidden)
            .fold(String::new(), |mut acc, f| {
                let fname = f.name;
                let stored = match row_data {
                    None => StoredValue::Absent,
                    // The stored secret or hash never reaches the browser.
                    Some(_) if matches!(f.kind, FieldKind::Password) => StoredValue::Absent,
                    Some(r) => match r.try_get_raw(fname) {
                        Err(_) => StoredValue::Unreadable,
                        Ok(raw) if raw.is_null() => StoredValue::Null,
                        Ok(_) => {
                            let decoded = match &f.kind {
                                FieldKind::Boolean => r
                                    .try_get::<bool, _>(fname)
                                    .or_else(|_| r.try_get::<i64, _>(fname).map(|v| v != 0))
                                    // An `INTEGER` column on the strict PostgreSQL pool.
                                    .or_else(|_| r.try_get::<i32, _>(fname).map(|v| v != 0))
                                    .ok()
                                    .map(|b| if b { "1" } else { "0" }.to_string()),
                                FieldKind::Number
                                | FieldKind::Integer { .. }
                                | FieldKind::ForeignKey { .. } => r
                                    .try_get::<i32, _>(fname)
                                    .map(|v| v.to_string())
                                    .or_else(|_| r.try_get::<i64, _>(fname).map(|v| v.to_string()))
                                    .or_else(|_| r.try_get::<f64, _>(fname).map(|v| v.to_string()))
                                    .or_else(|_| r.try_get::<String, _>(fname))
                                    .ok(),
                                _ => r.try_get::<String, _>(fname).ok(),
                            };
                            decoded.map_or(StoredValue::Unreadable, StoredValue::Value)
                        }
                    },
                };

                let input_widget =
                    render_field_widget(f, &stored, is_edit, entry.key_columns(), zone);
                // The picker shows the wall time in the panel's zone; name it.
                let zone_label = if matches!(f.kind, FieldKind::DateTimeUtc) {
                    format!(" ({})", escape_str(zone.name()))
                } else {
                    String::new()
                };
                let _ = write!(
                    acc,
                    "<div class=\"nexus-form-group\">\
                 <label class=\"nexus-label\">{}{zone_label}</label>\
                 {input_widget}\
                 </div>",
                    escape_str(f.label)
                );
                acc
            });

    let table_path = urlencoding::encode(t);
    let action_url = if let Some(id) = record_id {
        format!(
            "/nexus/table/{table_path}/record/{}",
            urlencoding::encode(id)
        )
    } else {
        format!("/nexus/table/{table_path}")
    };
    let safe_action_url = escape_str(&action_url);
    let safe_title = escape_str(&title);
    let mode = if is_edit { "edit" } else { "create" };

    format!(
        "<h3 class=\"nexus-modal-title\">{safe_title}</h3>\
         <form id=\"nexus-record-form\" data-nexus-action=\"{safe_action_url}\" data-nexus-mode=\"{mode}\" \
         autocomplete=\"off\" data-nexus-no-submit=\"true\">\
         <div class=\"nexus-fields-grid\">{fields_html}</div>\
         <div class=\"nexus-form-actions\">\
         <button type=\"button\" class=\"nexus-btn nexus-btn-ghost\" data-nexus-modal-close=\"true\">Cancel</button>\
         <button type=\"button\" class=\"nexus-btn nexus-btn-primary\" data-nexus-save=\"true\">Save Record</button>\
         </div></form>"
    )
}

/// Renders the input widget for one registered field.
///
/// A value is only placed in a typed widget (`number`, `date`,
/// `datetime-local`, `email`, `url`) when that widget shows it unchanged;
/// otherwise a text input shows the raw value, because browsers silently
/// replace unrepresentable values with `''`. NULL and undecodable values
/// render empty with an explanatory placeholder. A single-line value with line
/// breaks or control characters is shown read-only in a text area. `Password`
/// values are never rendered.
pub(super) fn render_field_widget(
    f: &FieldMeta,
    stored: &StoredValue,
    is_edit: bool,
    key_columns: &[&str],
    zone: &NexusTimeZone,
) -> String {
    let safe_fname = escape_str(f.name);
    let is_readonly = f.readonly || (is_edit && key_columns.contains(&f.name));
    let readonly_attr = if is_readonly { " readonly" } else { "" };
    // `readonly` does not apply to checkboxes or selects: they would stay
    // interactive while their unnamed value is never submitted.
    let locked_attr = if is_readonly { " disabled" } else { "" };
    let name_attr = if is_readonly {
        String::new()
    } else {
        format!(" name=\"{safe_fname}\"")
    };
    let text = match stored {
        StoredValue::Value(value) => value.as_str(),
        _ => "",
    };
    let placeholder = match stored {
        StoredValue::Null => " placeholder=\"NULL\"",
        StoredValue::Unreadable => {
            " placeholder=\"Stored value not shown; unchanged unless edited\""
        }
        _ => "",
    };

    match &f.kind {
        // The HTML parser drops one newline right after `<textarea>`; the
        // emitted newline keeps a value's own leading line break.
        FieldKind::Textarea | FieldKind::Json => format!(
            "<textarea{name_attr} class=\"nexus-input\" rows=\"4\"{readonly_attr}{placeholder}>\n{}</textarea>",
            escape_str(text)
        ),
        FieldKind::Boolean => {
            let checked = if text == "1" || text == "true" {
                " checked"
            } else {
                ""
            };
            format!(
                "<input type=\"hidden\"{name_attr} value=\"0\" />\
                 <input type=\"checkbox\"{name_attr} value=\"1\"{checked}{locked_attr} class=\"nexus-checkbox\" />"
            )
        }
        FieldKind::Password => {
            // Never pre-filled; an empty submission keeps the stored value.
            let hint = if is_edit {
                " placeholder=\"Leave blank to keep the current value\""
            } else {
                ""
            };
            format!(
                "<input type=\"password\"{name_attr} value=\"\" autocomplete=\"new-password\" class=\"nexus-input\"{readonly_attr}{hint} />"
            )
        }
        FieldKind::Enum { options } => {
            let mut opts = String::new();
            match stored {
                // An unregistered stored value stays selected but disabled:
                // browsers never submit a disabled option.
                StoredValue::Value(value) if !options.contains(&value.as_str()) => {
                    let value = escape_str(value);
                    let _ = write!(
                        opts,
                        "<option value=\"{value}\" selected disabled>{value} (not a registered option)</option>"
                    );
                }
                StoredValue::Null => opts.push_str("<option value=\"\" selected>NULL</option>"),
                StoredValue::Unreadable => opts.push_str(
                    "<option value=\"\" selected disabled>Stored value not shown</option>",
                ),
                _ => {}
            }
            for option in options {
                let selected = if *option == text { " selected" } else { "" };
                let option = escape_str(option);
                let _ = write!(
                    opts,
                    "<option value=\"{option}\"{selected}>{option}</option>"
                );
            }
            format!("<select{name_attr} class=\"nexus-input\"{locked_attr}>{opts}</select>")
        }
        // A single-line input strips line breaks, so editing the field would
        // silently join the lines, and a tab or other control character could
        // never be saved back. Show such values read-only instead.
        _ if text.chars().any(char::is_control) => format!(
            "<textarea class=\"nexus-input\" rows=\"4\" readonly>\n{}</textarea>\
             <p class=\"nexus-field-note\">This stored value has line breaks or control \
             characters, which this single-line field cannot keep, so it is read-only here.</p>",
            escape_str(text)
        ),
        kind => {
            let (input_type, value) = typed_input(kind, text, zone);
            // The browser steps and flags out-of-range integers; the server
            // still validates every submitted value.
            let bounds = match kind {
                FieldKind::Integer { min, max } if input_type == "number" => {
                    format!(" step=\"1\" min=\"{min}\" max=\"{max}\"")
                }
                _ => String::new(),
            };
            format!(
                "<input type=\"{input_type}\"{name_attr} value=\"{}\" class=\"nexus-input\"{readonly_attr}{placeholder}{bounds} />",
                escape_str(&value)
            )
        }
    }
}

/// Chooses the input type for a single-line kind and the value to show in
/// it; a `DateTimeUtc` instant is shown as its wall time in `zone`.
pub(super) fn typed_input(
    kind: &FieldKind,
    text: &str,
    zone: &NexusTimeZone,
) -> (&'static str, String) {
    let shown = |input_type| (input_type, text.to_owned());
    if text.is_empty() {
        return match kind {
            FieldKind::Email => shown("email"),
            FieldKind::Url => shown("url"),
            FieldKind::Number | FieldKind::Integer { .. } => shown("number"),
            FieldKind::Date => shown("date"),
            FieldKind::DateTime | FieldKind::DateTimeUtc => shown("datetime-local"),
            FieldKind::Time => shown("time"),
            _ => shown("text"),
        };
    }
    // Email and URL inputs strip surrounding whitespace and line breaks.
    let plain = text.trim() == text && !text.contains(['\n', '\r']);
    match kind {
        FieldKind::Email if plain => shown("email"),
        FieldKind::Url if plain => shown("url"),
        FieldKind::Number | FieldKind::Integer { .. } if is_html_number(text) => shown("number"),
        FieldKind::Date if is_local_date(text) => shown("date"),
        FieldKind::DateTime => match datetime_local_value(text) {
            Some(local) => ("datetime-local", local),
            None => shown("text"),
        },
        FieldKind::DateTimeUtc => zoned_input_value(text, zone).unwrap_or_else(|| shown("text")),
        FieldKind::Time => match time_input_value(text) {
            Some(time) => ("time", time),
            None => shown("text"),
        },
        _ => shown("text"),
    }
}

/// True for an HTML "valid floating-point number", the only text a number
/// input keeps: `-?digits(.digits)?([eE][+-]?digits)?`.
fn is_html_number(text: &str) -> bool {
    fn digits(text: &str) -> (&str, bool) {
        let end = text
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(text.len());
        (&text[end..], end > 0)
    }
    let (rest, integral) = digits(text.strip_prefix('-').unwrap_or(text));
    let rest = match rest.strip_prefix('.') {
        Some(fraction) => match digits(fraction) {
            (rest, true) => rest,
            (_, false) => return false,
        },
        None => rest,
    };
    let rest = match rest.strip_prefix(['e', 'E']) {
        Some(exponent) => match digits(exponent.strip_prefix(['+', '-']).unwrap_or(exponent)) {
            (rest, true) => rest,
            (_, false) => return false,
        },
        None => rest,
    };
    integral && rest.is_empty()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod widget_tests;
