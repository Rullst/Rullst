//! Create/edit record form rendering.
//!
//! The edit form must round-trip every value it does not change. It therefore
//! distinguishes SQL NULL and undecodable values from real ones, never shows a
//! value in a widget that would silently blank or rewrite it, and marks itself
//! `data-nexus-mode="edit"` so `nexus.js` submits only the controls the
//! administrator changed.

use crate::nexus::crud::input::{datetime_local_value, is_local_date};
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

/// Renders HTML form for creating or editing records in the modal dialog.
#[cfg_attr(mutants, mutants::skip)]
pub async fn render_record_form(
    _state: &NexusState,
    entry: &RegistryEntry,
    record_id: Option<&str>,
    tenant_id: Option<&str>,
) -> String {
    let is_edit = record_id.is_some();
    let title = if is_edit {
        format!("Edit {}", entry.label)
    } else {
        format!("New {}", entry.label)
    };

    let t = entry.table;
    let pk = entry.pk;

    use rullst_orm::_sqlx::{Row, ValueRef};
    let row_data = if let Some(id) = record_id {
        if let Some(pool) = rullst_core::db::safe_pool() {
            let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
            let clean_table = sanitize_identifier(t);
            let clean_pk = sanitize_identifier(pk);
            let pk_placeholder = if driver == "postgres" { "$1" } else { "?" };
            let tenant_predicate = match (entry.tenant_column, tenant_id) {
                (Some(column), Some(_)) if driver == "postgres" => {
                    format!(" AND {} = $2", sanitize_identifier(column))
                }
                (Some(column), Some(_)) => {
                    format!(" AND {} = ?", sanitize_identifier(column))
                }
                (Some(_), None) => " AND 1 = 0".to_string(),
                (None, _) => String::new(),
            };
            let sql = format!(
                "SELECT * FROM {} WHERE {} = {}{} LIMIT 1",
                clean_table, clean_pk, pk_placeholder, tenant_predicate
            );
            let mut q = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
            if let Ok(num_id) = id.parse::<i64>() {
                q = q.bind(num_id);
            } else {
                q = q.bind(id);
            }
            if entry.tenant_column.is_some()
                && let Some(tenant_id) = tenant_id
            {
                q = q.bind(tenant_id);
            }
            match q.fetch_optional(pool).await {
                Ok(row) => row,
                Err(_) => {
                    tracing::error!(table = entry.table, "Nexus record query failed");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    let fields_html =
        entry
            .fields
            .iter()
            .filter(|field| !field.hidden)
            .fold(String::new(), |mut acc, f| {
                let fname = f.name;
                let stored = match row_data.as_ref() {
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
                                    .ok()
                                    .map(|b| if b { "1" } else { "0" }.to_string()),
                                FieldKind::Number | FieldKind::ForeignKey { .. } => r
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

                let input_widget = render_field_widget(f, &stored, is_edit, pk);
                let _ = write!(
                    acc,
                    "<div class=\"nexus-form-group\">\
                 <label class=\"nexus-label\">{}</label>\
                 {input_widget}\
                 </div>",
                    escape_str(f.label)
                );
                acc
            });

    let table_path = urlencoding::encode(t);
    let action_url = if let Some(id) = record_id {
        format!("/nexus/table/{table_path}/{}", urlencoding::encode(id))
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
/// render empty with an explanatory placeholder. `Password` values are never
/// rendered.
pub(super) fn render_field_widget(
    f: &FieldMeta,
    stored: &StoredValue,
    is_edit: bool,
    pk: &str,
) -> String {
    let safe_fname = escape_str(f.name);
    let is_readonly = f.readonly || (is_edit && f.name == pk);
    let readonly_attr = if is_readonly { " readonly" } else { "" };
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
        FieldKind::Textarea | FieldKind::Json => format!(
            "<textarea{name_attr} class=\"nexus-input\" rows=\"4\"{readonly_attr}{placeholder}>{}</textarea>",
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
                 <input type=\"checkbox\"{name_attr} value=\"1\"{checked}{readonly_attr} class=\"nexus-checkbox\" />"
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
            format!("<select{name_attr} class=\"nexus-input\"{readonly_attr}>{opts}</select>")
        }
        kind => {
            let (input_type, value) = typed_input(kind, text);
            format!(
                "<input type=\"{input_type}\"{name_attr} value=\"{}\" class=\"nexus-input\"{readonly_attr}{placeholder} />",
                escape_str(&value)
            )
        }
    }
}

/// Chooses the input type for a single-line kind and the value to show in it.
fn typed_input(kind: &FieldKind, text: &str) -> (&'static str, String) {
    let shown = |input_type| (input_type, text.to_owned());
    if text.is_empty() {
        return match kind {
            FieldKind::Email => shown("email"),
            FieldKind::Url => shown("url"),
            FieldKind::Number => shown("number"),
            FieldKind::Date => shown("date"),
            FieldKind::DateTime => shown("datetime-local"),
            _ => shown("text"),
        };
    }
    // Email and URL inputs strip surrounding whitespace and line breaks.
    let plain = text.trim() == text && !text.contains(['\n', '\r']);
    match kind {
        FieldKind::Email if plain => shown("email"),
        FieldKind::Url if plain => shown("url"),
        FieldKind::Number if is_html_number(text) => shown("number"),
        FieldKind::Date if is_local_date(text) => shown("date"),
        FieldKind::DateTime => match datetime_local_value(text) {
            Some(local) => ("datetime-local", local),
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
