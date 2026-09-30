//! Row-action forms rendered by the Studio table view.

use super::MUTATION_BODY_LIMIT;
use crate::data_browser::db::{
    StudioColumn, StudioTableSchema, build_rows_html, escape_html_attr, get_any_value_as_string,
};
use crate::data_browser::limits::{MAX_CELL_BYTES, display_cell};
use sqlx::Row;
use std::fmt::Write;

/// Largest update body besides the key fields: the longest column name and
/// a maximum-size value whose every byte is percent-encoded.
const UPDATE_FORM_BYTES: usize =
    "&column=".len() + 64 + "&value=".len() + 3 * MAX_CELL_BYTES + "&set_null=true".len();

/// Bytes a browser sends for `value` in an `application/x-www-form-urlencoded`
/// body: ASCII alphanumerics and `*-._` unchanged, a space as `+` and every
/// other byte percent-encoded.
pub(super) fn form_encoded_len(value: &str) -> usize {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'*' | b'-' | b'.' | b'_' | b' ') {
                1
            } else {
                3
            }
        })
        .sum()
}

/// Encoded size of the hidden `pk_*` fields, each followed by a separator.
fn key_form_bytes(columns: &[StudioColumn], primary_keys: &[usize], values: &[String]) -> usize {
    primary_keys
        .iter()
        .zip(values)
        .map(|(index, value)| {
            let name = columns.get(*index).map_or(0, |column| column.name.len());
            "pk_".len() + name + 1 + form_encoded_len(value) + 1
        })
        .sum()
}

/// Returns the text of each key cell when a form can submit it back to the same
/// row, or why the row must stay read-only. A NULL or undecodable key has no
/// exact text (it would render as `NULL`), a key longer than a mutation accepts
/// could not be submitted, and browsers rewrite line breaks (CR and LF become
/// CRLF) and NUL in form values, which could address a different row.
fn submittable_key(
    row: &<rullst_orm::RullstDatabase as sqlx::Database>::Row,
    primary_keys: &[usize],
) -> Result<Vec<String>, &'static str> {
    let mut values = Vec::with_capacity(primary_keys.len());
    for index in primary_keys {
        let value = match row.try_get::<Option<String>, _>(*index) {
            Ok(Some(value)) => value,
            Ok(None) => return Err("NULL key"),
            Err(_) => return Err("key is not decodable as text"),
        };
        if value.len() > MAX_CELL_BYTES {
            return Err("key longer than 16 KiB");
        }
        if value.contains(['\r', '\n', '\0']) {
            return Err("key contains a line break or NUL");
        }
        values.push(value);
    }
    Ok(values)
}

pub(crate) fn build_mutable_rows_html(
    records: &[<rullst_orm::RullstDatabase as sqlx::Database>::Row],
    schema: &StudioTableSchema,
    table: &str,
) -> String {
    let columns = &schema.columns;
    if !schema.supports_mutations() {
        let column_names = columns
            .iter()
            .map(|column| column.name.clone())
            .collect::<Vec<_>>();
        return build_rows_html(records, &column_names);
    }
    let primary_keys = schema.primary_key_indices();
    if records.is_empty() {
        return format!(
            "<tr><td colspan=\"{}\" class=\"px-6 py-16 text-center text-sm text-slate-500 font-medium bg-slate-900/20\">No records found inside this table.</td></tr>",
            columns.len() + 1
        );
    }

    let encoded_table = urlencoding::encode(table);
    let delete_form_bytes = "&confirm=".len() + form_encoded_len(&format!("DELETE {table}"));
    let mut html = String::new();
    for row in records {
        html.push_str("<tr class=\"border-b border-slate-800/40 hover:bg-slate-900/30 transition duration-150\">");
        for index in 0..columns.len() {
            let value = get_any_value_as_string(row, index);
            let class = if value == "NULL" {
                "text-slate-600 font-mono italic"
            } else {
                "text-slate-300"
            };
            let _ = write!(
                html,
                "<td class=\"px-6 py-4 text-sm truncate max-w-xs {class}\">{}</td>",
                escape_html_attr(&display_cell(&value))
            );
        }

        // Each offered form must fit the mutation body limit, or submitting it
        // could only fail with `413`.
        let key_values = submittable_key(row, &primary_keys).and_then(|values| {
            let key_bytes = key_form_bytes(columns, &primary_keys, &values);
            if key_bytes + delete_form_bytes > MUTATION_BODY_LIMIT {
                Err("key too large for the 64 KiB form limit")
            } else {
                Ok((values, key_bytes + UPDATE_FORM_BYTES <= MUTATION_BODY_LIMIT))
            }
        });
        let (key_values, update_fits) = match key_values {
            Ok(checked) => checked,
            Err(reason) => {
                let _ = write!(
                    html,
                    "<td class=\"px-6 py-4 text-xs text-slate-500\">Read-only: {reason}</td></tr>"
                );
                continue;
            }
        };

        let mut primary_inputs = String::new();
        for (index, value) in primary_keys.iter().zip(&key_values) {
            let column = &columns[*index];
            let _ = write!(
                primary_inputs,
                "<input type=\"hidden\" name=\"pk_{}\" value=\"{}\">",
                escape_html_attr(&column.name),
                escape_html_attr(value)
            );
        }
        let mut options = String::from("<option value=\"\">Choose column</option>");
        let mut editable_columns = 0usize;
        for column in columns
            .iter()
            .filter(|column| !column.primary_key && column.kind.is_editable())
        {
            editable_columns += 1;
            let null_hint = if column.nullable { " (nullable)" } else { "" };
            let _ = write!(
                options,
                "<option value=\"{}\">{}{}</option>",
                escape_html_attr(&column.name),
                escape_html_attr(&column.name),
                null_hint
            );
        }
        let update = if editable_columns == 0 {
            String::new()
        } else if !update_fits {
            "<p class=\"mb-2 text-slate-500\">Edit unavailable: key too large for the 64 KiB form limit</p>".to_string()
        } else {
            format!(
                "<details class=\"mb-2\"><summary class=\"cursor-pointer text-sky-400\">Edit</summary>\
                 <form method=\"post\" action=\"/studio/tables/{encoded_table}/rows/update\" class=\"mt-2 space-y-2\">\
                 {primary_inputs}<select name=\"column\" required class=\"w-full bg-slate-950 border border-slate-700 rounded p-1\">{options}</select>\
                 <input name=\"value\" maxlength=\"16384\" class=\"w-full bg-slate-950 border border-slate-700 rounded p-1\" placeholder=\"replacement value\">\
                 <label class=\"block text-slate-500\"><input type=\"checkbox\" name=\"set_null\" value=\"true\"> set NULL</label>\
                 <button class=\"text-sky-300 border border-sky-800 rounded px-2 py-1\" type=\"submit\">Apply</button></form></details>"
            )
        };
        let confirmation = escape_html_attr(&format!("DELETE {table}"));
        let delete = format!(
            "<details><summary class=\"cursor-pointer text-red-400\">Delete</summary>\
             <form method=\"post\" action=\"/studio/tables/{encoded_table}/rows/delete\" class=\"mt-2 space-y-2\">\
             {primary_inputs}<input name=\"confirm\" required maxlength=\"80\" class=\"w-full bg-slate-950 border border-red-900 rounded p-1\" placeholder=\"{confirmation}\">\
             <button class=\"text-red-300 border border-red-900 rounded px-2 py-1\" type=\"submit\">Delete row</button></form></details>"
        );
        let _ = write!(
            html,
            "<td class=\"px-6 py-4 text-xs min-w-56\">{update}{delete}</td></tr>"
        );
    }
    html
}
