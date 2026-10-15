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
mod tests {
    use super::*;

    fn tenant_entry() -> RegistryEntry {
        RegistryEntry {
            table: "records",
            label: "Records",
            icon: "R",
            pk: "id",
            pk_columns: Vec::new(),
            tenant_column: Some("tenant_id"),
            fields: vec![
                FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
                FieldMeta::new("tenant_id", "Tenant", FieldKind::Text)
                    .readonly()
                    .hidden(),
                FieldMeta::new("title", "Title", FieldKind::Text),
            ],
        }
    }

    #[test]
    fn scoped_query_binds_search_before_trusted_tenant() {
        let entry = tenant_entry();
        let visible = vec![&entry.fields[0], &entry.fields[2]];
        let (sql, binds) = build_table_query(
            &entry,
            &visible,
            "needle",
            1,
            Some("title"),
            Some("asc"),
            Some("tenant-a"),
        );

        assert!(
            sql.contains("(title LIKE ? ESCAPE '!')")
                || sql.contains("(CAST(title AS TEXT) ILIKE $1 ESCAPE '!')")
        );
        assert!(sql.contains("tenant_id = ?") || sql.contains("tenant_id = $2"));
        assert_eq!(binds, ["%needle%", "tenant-a"]);
    }

    #[test]
    fn password_columns_are_neither_selected_nor_sortable() {
        let entry = RegistryEntry {
            table: "accounts",
            label: "Accounts",
            icon: "A",
            pk: "id",
            pk_columns: Vec::new(),
            tenant_column: None,
            fields: vec![
                FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
                FieldMeta::new("name", "Name", FieldKind::Text),
                FieldMeta::new("api_key", "API key", FieldKind::Password),
            ],
        };
        let visible = entry.fields.iter().collect::<Vec<_>>();
        let (sql, _) = build_table_query(
            &entry,
            &visible,
            "needle",
            1,
            Some("api_key"),
            Some("asc"),
            None,
        );

        assert!(sql.starts_with("SELECT id, name FROM accounts"), "{sql}");
        assert!(!sql.contains("api_key"), "{sql}");
        assert!(sql.contains("ORDER BY id asc"), "{sql}");
    }

    #[test]
    fn the_primary_key_is_selected_once_whether_or_not_it_is_visible() {
        let entry = RegistryEntry {
            table: "accounts",
            label: "Accounts",
            icon: "A",
            pk: "id",
            pk_columns: Vec::new(),
            tenant_column: None,
            fields: vec![
                FieldMeta::new("id", "ID", FieldKind::Number),
                FieldMeta::new("name", "Name", FieldKind::Text),
            ],
        };
        let without_key = vec![&entry.fields[1]];
        let (sql, _) = build_table_query(&entry, &without_key, "", 1, None, None, None);
        assert!(sql.starts_with("SELECT id, name FROM accounts"), "{sql}");
        let only_key = vec![&entry.fields[0]];
        let (sql, _) = build_table_query(&entry, &only_key, "", 1, None, None, None);
        assert!(sql.starts_with("SELECT id FROM accounts"), "{sql}");
    }

    #[test]
    fn search_without_a_searchable_column_matches_nothing() {
        let entry = RegistryEntry {
            table: "orders",
            label: "Orders",
            icon: "O",
            pk: "id",
            pk_columns: Vec::new(),
            tenant_column: None,
            fields: vec![
                FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
                FieldMeta::new("total", "Total", FieldKind::Number),
                FieldMeta::new("note", "Note", FieldKind::Text).hidden(),
            ],
        };
        let visible = vec![&entry.fields[0], &entry.fields[1]];
        let (sql, binds) = build_table_query(&entry, &visible, "refunded", 1, None, None, None);
        assert!(sql.contains(" WHERE 1 = 0 ORDER BY"), "{sql}");
        assert!(!sql.contains("note"), "{sql}");
        assert!(binds.is_empty());

        // An empty query still lists every row.
        let (sql, _) = build_table_query(&entry, &visible, "", 1, None, None, None);
        assert!(!sql.contains("WHERE"), "{sql}");
    }

    #[test]
    fn hidden_columns_are_not_sortable() {
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "U",
            pk: "id",
            pk_columns: Vec::new(),
            tenant_column: None,
            fields: vec![
                FieldMeta::new("id", "ID", FieldKind::Number)
                    .hidden()
                    .readonly(),
                FieldMeta::new("email", "E-mail", FieldKind::Email),
                FieldMeta::new("password_hash", "Password hash", FieldKind::Text).hidden(),
            ],
        };
        let visible = vec![&entry.fields[1]];
        let (sql, _) = build_table_query(
            &entry,
            &visible,
            "",
            1,
            Some("password_hash"),
            Some("asc"),
            None,
        );
        assert!(!sql.contains("password_hash"), "{sql}");
        assert!(sql.contains("ORDER BY id asc LIMIT"), "{sql}");

        // The key stays sortable even when it is hidden.
        let (sql, _) = build_table_query(&entry, &visible, "", 1, Some("id"), None, None);
        assert!(sql.contains("ORDER BY id DESC LIMIT"), "{sql}");
    }

    #[test]
    fn pages_sorted_by_a_non_unique_column_break_ties_by_primary_key() {
        let entry = tenant_entry();
        let visible = vec![&entry.fields[0], &entry.fields[2]];
        let (sql, _) = build_table_query(
            &entry,
            &visible,
            "",
            2,
            Some("title"),
            Some("asc"),
            Some("tenant-a"),
        );
        assert!(
            sql.ends_with(" ORDER BY title asc, id asc LIMIT 15 OFFSET 15"),
            "{sql}"
        );

        let (sql, _) = build_table_query(&entry, &visible, "", 1, None, None, Some("tenant-a"));
        assert!(
            sql.ends_with(" ORDER BY id DESC LIMIT 15 OFFSET 0"),
            "{sql}"
        );
    }

    #[test]
    fn every_field_kind_has_a_label() {
        let relation = FieldKind::ForeignKey {
            table: "teams",
            label_col: "name",
        };
        let options = FieldKind::Enum { options: vec!["a"] };
        let labels = [
            (FieldKind::Text, "text"),
            (FieldKind::Textarea, "textarea"),
            (FieldKind::Email, "email"),
            (FieldKind::Url, "url"),
            (FieldKind::Number, "number"),
            (FieldKind::Integer { min: 0, max: 1 }, "integer"),
            (FieldKind::Boolean, "boolean"),
            (FieldKind::Date, "date"),
            (FieldKind::DateTime, "datetime"),
            (FieldKind::Time, "time"),
            (FieldKind::DateTimeUtc, "datetime_utc"),
            (FieldKind::Password, "password"),
            (FieldKind::Json, "json"),
            (relation, "relation"),
            (options, "enum"),
        ];
        for (kind, label) in labels {
            assert_eq!(field_kind_label(&kind), label);
        }
        assert_eq!(field_kind_input_type(&FieldKind::Time), "time");
        assert_eq!(
            field_kind_input_type(&FieldKind::DateTimeUtc),
            "datetime-local"
        );
    }

    #[test]
    fn composite_keys_are_selected_in_key_order_and_break_ties() {
        let entry = RegistryEntry {
            table: "lesson_progress",
            label: "Progress",
            icon: "P",
            pk: "user_id",
            pk_columns: vec!["user_id", "lesson_id"],
            tenant_column: None,
            fields: vec![
                FieldMeta::new("user_id", "User", FieldKind::Number).hidden(),
                FieldMeta::new("lesson_id", "Lesson", FieldKind::Number),
                FieldMeta::new("percent", "Percent", FieldKind::Number),
            ],
        };
        let visible = vec![&entry.fields[1], &entry.fields[2]];
        let (sql, _) = build_table_query(&entry, &visible, "", 1, None, None, None);
        assert_eq!(
            sql,
            "SELECT user_id, lesson_id, percent FROM lesson_progress \
             ORDER BY user_id DESC, lesson_id DESC LIMIT 15 OFFSET 0"
        );

        let only_percent = vec![&entry.fields[2]];
        let (sql, _) = build_table_query(
            &entry,
            &only_percent,
            "",
            2,
            Some("percent"),
            Some("asc"),
            None,
        );
        assert_eq!(
            sql,
            "SELECT user_id, lesson_id, percent FROM lesson_progress \
             ORDER BY percent asc, user_id asc, lesson_id asc LIMIT 15 OFFSET 15"
        );

        // Any key column is sortable, even a hidden one.
        let (sql, _) = build_table_query(&entry, &visible, "", 1, Some("lesson_id"), None, None);
        assert!(
            sql.ends_with(" ORDER BY lesson_id DESC, user_id DESC LIMIT 15 OFFSET 0"),
            "{sql}"
        );
        let (sql, _) = build_table_query(&entry, &visible, "", 1, Some("user_id"), None, None);
        assert!(
            sql.ends_with(" ORDER BY user_id DESC, lesson_id DESC LIMIT 15 OFFSET 0"),
            "{sql}"
        );
    }

    #[test]
    // TM-NEXUS-02
    fn scoped_query_without_context_is_empty_by_construction() {
        let entry = tenant_entry();
        let visible = vec![&entry.fields[2]];
        let (sql, binds) = build_table_query(&entry, &visible, "", 1, None, None, None);

        assert!(sql.contains("WHERE 1 = 0"));
        assert!(binds.is_empty());
    }
}
