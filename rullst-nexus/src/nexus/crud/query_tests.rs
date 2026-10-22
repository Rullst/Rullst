use super::*;
use crate::nexus::NexusTimeZone;
use crate::nexus::crud::form::typed_input;

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
    // Input types come from the form renderer, which every build (Miri
    // included) compiles; its own tests cover each kind.
    assert_eq!(
        typed_input(&FieldKind::Time, "", &NexusTimeZone::utc()).0,
        "time"
    );
    assert_eq!(
        typed_input(&FieldKind::DateTimeUtc, "", &NexusTimeZone::utc()).0,
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
