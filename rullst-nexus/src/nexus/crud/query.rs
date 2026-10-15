//! Query building and parameter extraction for Nexus CRUD.

use super::dialect::{contains_pattern, search_predicate, select_column, tenant_predicate};
use crate::nexus::types::{FieldKind, FieldMeta, NexusState, RegistryEntry};
use serde::Deserialize;

/// Query parameters for table pagination, filtering, and sorting.
#[derive(Deserialize)]
pub struct PaginationParams {
    /// Current page number (1-indexed).
    pub page: Option<u32>,
    /// Search query string.
    pub q: Option<String>,
    /// Column to sort by.
    pub sort_by: Option<String>,
    /// Sort direction (`asc` or `desc`).
    pub order: Option<String>,
}

/// Form payload for batch actions (e.g. bulk deletion).
pub struct BatchActionForm {
    /// Action verb (e.g. `delete`).
    pub action: String,
    /// Selected record IDs.
    pub selected_ids: Vec<String>,
}

/// Locates a model entry in the Nexus registry by its database table name.
pub fn find_entry<'a>(state: &'a NexusState, table: &str) -> Option<&'a RegistryEntry> {
    state.registry.iter().find(|e| e.table == table)
}

/// Human-friendly text label for a field kind.
pub fn field_kind_label(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Text => "text",
        FieldKind::Textarea => "textarea",
        FieldKind::Email => "email",
        FieldKind::Url => "url",
        FieldKind::Number => "number",
        FieldKind::Integer { .. } => "integer",
        FieldKind::Boolean => "boolean",
        FieldKind::Date => "date",
        FieldKind::DateTime => "datetime",
        FieldKind::Time => "time",
        FieldKind::DateTimeUtc => "datetime_utc",
        FieldKind::Password => "password",
        FieldKind::Json => "json",
        FieldKind::ForeignKey { .. } => "relation",
        FieldKind::Enum { .. } => "enum",
    }
}

#[cfg_attr(mutants, mutants::skip)]
#[allow(dead_code)]
pub fn field_kind_sql(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Number | FieldKind::Integer { .. } => "INTEGER",
        FieldKind::Boolean => "INTEGER",
        FieldKind::ForeignKey { .. } => "INTEGER",
        FieldKind::Date | FieldKind::DateTime | FieldKind::Time | FieldKind::DateTimeUtc => "TEXT",
        FieldKind::Json => "TEXT",
        FieldKind::Enum { .. } => "TEXT",
        _ => "TEXT",
    }
}

#[cfg(all(test, not(miri)))]
#[cfg_attr(mutants, mutants::skip)]
#[allow(dead_code)]
pub fn field_kind_input_type(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Email => "email",
        FieldKind::Url => "url",
        FieldKind::Number | FieldKind::Integer { .. } => "number",
        FieldKind::Password => "password",
        FieldKind::Date => "date",
        FieldKind::DateTime | FieldKind::DateTimeUtc => "datetime-local",
        FieldKind::Time => "time",
        FieldKind::ForeignKey { .. } => "select",
        FieldKind::Enum { .. } => "select",
        _ => "text",
    }
}

/// Whether `field` is matched by the list search: a visible text, textarea,
/// e-mail or URL column.
pub(crate) fn is_searchable(field: &FieldMeta) -> bool {
    !field.hidden
        && matches!(
            field.kind,
            FieldKind::Text | FieldKind::Textarea | FieldKind::Email | FieldKind::Url
        )
}

/// Sanitizes an identifier name preventing SQL injection in dynamic DDL/DML.
pub fn sanitize_identifier(name: &str) -> String {
    let mut res = String::with_capacity(64);
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            if res.len() + c.len_utf8() > 64 {
                break;
            }
            res.push(c);
        }
    }
    res
}

/// Constructs a parameterized SELECT query for paginated and filtered table views.
pub fn build_table_query(
    entry: &RegistryEntry,
    visible_fields: &[&FieldMeta],
    q: &str,
    page: u32,
    sort_by: Option<&str>,
    order: Option<&str>,
    tenant_id: Option<&str>,
) -> (String, Vec<String>) {
    let clean_table = sanitize_identifier(entry.table);
    let limit = 15;
    let page = page.max(1);
    let offset = page.saturating_sub(1).saturating_mul(limit);

    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    // Password values are never rendered, so they are never read either.
    let readable: Vec<&FieldMeta> = visible_fields
        .iter()
        .copied()
        .filter(|f| !matches!(f.kind, FieldKind::Password))
        .collect();
    let mut select_cols: Vec<String> = readable.iter().map(|f| select_column(f, driver)).collect();

    // Every key column is read, in key order before the fields, so each row
    // has an address even when its key is not shown.
    let key_columns: Vec<String> = entry
        .key_columns()
        .iter()
        .map(|column| sanitize_identifier(column))
        .collect();
    let missing_keys = key_columns
        .iter()
        .filter(|column| {
            !readable
                .iter()
                .any(|f| sanitize_identifier(f.name) == **column)
        })
        .cloned();
    select_cols.splice(0..0, missing_keys);

    let mut select_list = select_cols.join(", ");
    if select_list.is_empty() {
        select_list = "*".to_string();
    }

    let mut sql = format!("SELECT {} FROM {}", select_list, clean_table);
    let mut binds = Vec::new();

    let mut predicates = Vec::new();
    if !q.is_empty() {
        let text_fields: Vec<String> = entry
            .fields
            .iter()
            .filter(|f| is_searchable(f))
            .map(|f| sanitize_identifier(f.name))
            .collect();

        if text_fields.is_empty() {
            // No column can match the query, so no row is a search result;
            // unfiltered rows must not be presented as matches.
            predicates.push("1 = 0".to_string());
        } else {
            let where_clauses: Vec<String> = text_fields
                .iter()
                .enumerate()
                .map(|(idx, col)| {
                    let placeholder = if driver == "postgres" {
                        format!("${}", binds.len() + idx + 1)
                    } else {
                        "?".to_string()
                    };
                    search_predicate(col, &placeholder, driver)
                })
                .collect();

            predicates.push(format!("({})", where_clauses.join(" OR ")));

            // `%` and `_` typed by the administrator are literal characters.
            let search_term = contains_pattern(q);
            for _ in 0..text_fields.len() {
                binds.push(search_term.clone());
            }
        }
    }

    if let Some(tenant_column) = entry.tenant_column {
        if let Some(tenant_id) = tenant_id {
            let placeholder = if driver == "postgres" {
                format!("${}", binds.len() + 1)
            } else {
                "?".to_string()
            };
            predicates.push(tenant_predicate(tenant_column, &placeholder, driver));
            binds.push(tenant_id.to_string());
        } else {
            // Public rendering helpers also fail closed if called outside the
            // authenticated HTTP handler path.
            predicates.push("1 = 0".to_string());
        }
    }

    if !predicates.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&predicates.join(" AND "));
    }

    // Ordering by a Password or hidden column would reveal the order of
    // values the panel never shows (a `password_hash`, a sealed column), so
    // only the key and visible non-Password fields are sortable.
    let sort_col = sort_by
        .filter(|candidate| {
            entry.is_key_column(candidate)
                || entry.fields.iter().any(|field| {
                    field.name == *candidate
                        && !field.hidden
                        && !matches!(field.kind, FieldKind::Password)
                })
        })
        .unwrap_or(entry.pk);
    let sort_dir = order
        .filter(|&o| o.eq_ignore_ascii_case("asc") || o.eq_ignore_ascii_case("desc"))
        .unwrap_or("DESC");
    let clean_sort_col = sanitize_identifier(sort_col);
    // A non-unique sort column has no stable order between separate page
    // queries; the key columns break ties so rows neither repeat nor vanish.
    let tie_breaker = key_columns
        .iter()
        .filter(|column| **column != clean_sort_col)
        .map(|column| format!(", {column} {sort_dir}"))
        .collect::<String>();

    let _ = std::fmt::Write::write_fmt(
        &mut sql,
        format_args!(
            " ORDER BY {} {}{} LIMIT {} OFFSET {}",
            clean_sort_col, sort_dir, tie_breaker, limit, offset
        ),
    );

    (sql, binds)
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
