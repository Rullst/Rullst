//! Query building and parameter extraction for Nexus CRUD.

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
#[derive(Deserialize)]
pub struct BatchActionForm {
    /// Action verb (e.g. `delete`).
    pub action: String,
    /// Selected record IDs.
    #[serde(default)]
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
        FieldKind::Boolean => "boolean",
        FieldKind::Date => "date",
        FieldKind::DateTime => "datetime",
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
        FieldKind::Number => "INTEGER",
        FieldKind::Boolean => "INTEGER",
        FieldKind::ForeignKey { .. } => "INTEGER",
        FieldKind::Date | FieldKind::DateTime => "TEXT",
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
        FieldKind::Number => "number",
        FieldKind::Password => "password",
        FieldKind::Date => "date",
        FieldKind::DateTime => "datetime-local",
        FieldKind::ForeignKey { .. } => "select",
        FieldKind::Enum { .. } => "select",
        _ => "text",
    }
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
) -> (String, Vec<String>) {
    let clean_table = sanitize_identifier(entry.table);
    let limit = 15;
    let page = page.max(1);
    let offset = page.saturating_sub(1).saturating_mul(limit);

    let mut select_cols: Vec<String> = visible_fields
        .iter()
        .map(|f| sanitize_identifier(f.name))
        .collect();

    let clean_pk = sanitize_identifier(entry.pk);
    if !select_cols.contains(&clean_pk) {
        select_cols.insert(0, clean_pk);
    }

    let mut select_list = select_cols.join(", ");
    if select_list.is_empty() {
        select_list = "*".to_string();
    }

    let mut sql = format!("SELECT {} FROM {}", select_list, clean_table);
    let mut binds = Vec::new();

    if !q.is_empty() {
        let text_fields: Vec<String> = entry
            .fields
            .iter()
            .filter(|f| {
                matches!(
                    f.kind,
                    FieldKind::Text | FieldKind::Textarea | FieldKind::Email | FieldKind::Url
                )
            })
            .map(|f| sanitize_identifier(f.name))
            .collect();

        if !text_fields.is_empty() {
            let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
            let where_clauses: Vec<String> = text_fields
                .iter()
                .enumerate()
                .map(|(idx, col)| {
                    if driver == "postgres" {
                        format!("{} LIKE ${}", col, idx + 1)
                    } else {
                        format!("{} LIKE ?", col)
                    }
                })
                .collect();

            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" OR "));

            let search_term = format!("%{}%", q);
            for _ in 0..text_fields.len() {
                binds.push(search_term.clone());
            }
        }
    }

    let sort_col = sort_by
        .filter(|candidate| {
            *candidate == entry.pk || entry.fields.iter().any(|field| field.name == *candidate)
        })
        .unwrap_or(entry.pk);
    let sort_dir = order
        .filter(|&o| o.eq_ignore_ascii_case("asc") || o.eq_ignore_ascii_case("desc"))
        .unwrap_or("DESC");
    let clean_sort_col = sanitize_identifier(sort_col);

    let _ = std::fmt::Write::write_fmt(
        &mut sql,
        format_args!(
            " ORDER BY {} {} LIMIT {} OFFSET {}",
            clean_sort_col, sort_dir, limit, offset
        ),
    );

    (sql, binds)
}
