use super::*;
use crate::nexus::types::{FieldKind, FieldMeta};
use axum::http::StatusCode;
use std::sync::Arc;

#[test]
fn placeholders_are_dialect_correct() {
    assert_eq!(placeholder(3, "postgres"), "$3");
    assert_eq!(placeholder(2, "sqlite"), "?");
}

fn single() -> RegistryEntry {
    RegistryEntry {
        table: "notes",
        label: "Notes",
        icon: "N",
        pk: "id",
        pk_columns: Vec::new(),
        tenant_column: Some("tenant_id"),
        fields: vec![
            FieldMeta::new("id", "ID", FieldKind::Number),
            FieldMeta::new("tenant_id", "Tenant", FieldKind::Text)
                .readonly()
                .hidden(),
            FieldMeta::new("title", "Title", FieldKind::Text),
        ],
    }
}

/// `lesson_progress (user_id, lesson_id)` with an editable percentage.
fn composite() -> RegistryEntry {
    RegistryEntry {
        table: "lesson_progress",
        label: "Progress",
        icon: "P",
        pk: "user_id",
        pk_columns: vec!["user_id", "lesson_id"],
        tenant_column: None,
        fields: vec![
            FieldMeta::new("user_id", "User", FieldKind::Number),
            FieldMeta::new("lesson_id", "Lesson", FieldKind::Text),
            FieldMeta::new("percent", "Percent", FieldKind::Number),
        ],
    }
}

#[test]
fn updates_bind_the_assignments_then_every_key_column_then_the_tenant() {
    let assignments = ["title = $1".to_string(), "body = $2".to_string()];
    assert_eq!(
        update_sql(&single(), &assignments, 2, true, "postgres"),
        "UPDATE notes SET title = $1, body = $2 WHERE id = $3 AND tenant_id = $4"
    );
    assert_eq!(
        update_sql(&single(), &assignments[..1], 1, false, "sqlite"),
        "UPDATE notes SET title = $1 WHERE id = ?"
    );
    let mut scoped = composite();
    scoped.tenant_column = Some("tenant_id");
    assert_eq!(
        update_sql(&scoped, &["percent = $1".to_string()], 1, true, "postgres"),
        "UPDATE lesson_progress SET percent = $1 WHERE user_id = $2 AND lesson_id = $3 \
         AND tenant_id = $4"
    );
    // A literal assignment binds nothing, so the key starts at $1.
    assert_eq!(
        update_sql(
            &composite(),
            &["done = '1'".to_string()],
            0,
            false,
            "postgres"
        ),
        "UPDATE lesson_progress SET done = '1' WHERE user_id = $1 AND lesson_id = $2"
    );
}

#[test]
fn deletes_bind_every_key_column_then_the_tenant() {
    assert_eq!(
        delete_sql(&single(), true, "postgres"),
        "DELETE FROM notes WHERE id = $1 AND tenant_id = $2"
    );
    assert_eq!(
        delete_sql(&single(), false, "sqlite"),
        "DELETE FROM notes WHERE id = ?"
    );
    let mut scoped = composite();
    scoped.tenant_column = Some("tenant_id");
    assert_eq!(
        delete_sql(&scoped, true, "postgres"),
        "DELETE FROM lesson_progress WHERE user_id = $1 AND lesson_id = $2 AND tenant_id = $3"
    );
    assert_eq!(
        delete_sql(&scoped, true, "mysql"),
        "DELETE FROM lesson_progress WHERE user_id = ? AND lesson_id = ? \
         AND CAST(tenant_id AS BINARY) = CAST(? AS BINARY)"
    );
}

fn validated<'a>(entry: &'a RegistryEntry, pairs: &[(&str, &str)]) -> Vec<ValidatedFieldValue<'a>> {
    validate_form_values(
        entry,
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        FormMode::Create,
    )
    .expect("valid create form")
}

#[test]
fn a_create_form_names_its_record_only_with_every_canonical_key_value() {
    let entry = composite();
    let full = validated(
        &entry,
        &[("user_id", "3"), ("lesson_id", "a,b"), ("percent", "5")],
    );
    assert_eq!(submitted_address(&entry, &full).as_deref(), Some("3,a%2Cb"));

    let partial = validated(&entry, &[("user_id", "3"), ("percent", "5")]);
    assert_eq!(submitted_address(&entry, &partial), None);
    let blank = validated(&entry, &[("user_id", "3"), ("lesson_id", "  ")]);
    assert_eq!(submitted_address(&entry, &blank), None);
    // `07` is a valid number but not the canonical spelling of the key.
    let padded = validated(&entry, &[("user_id", "07"), ("lesson_id", "x")]);
    assert_eq!(submitted_address(&entry, &padded), None);

    let mut text_key = single();
    text_key.fields[0].kind = FieldKind::Text;
    let single_key = validated(&text_key, &[("id", "page,one"), ("title", "t")]);
    assert_eq!(
        submitted_address(&text_key, &single_key).as_deref(),
        Some("page,one")
    );
    let generated = validated(&text_key, &[("title", "t")]);
    assert_eq!(submitted_address(&text_key, &generated), None);
}

#[test]
fn blank_key_values_are_absent_or_whitespace() {
    assert!(blank(None));
    assert!(blank(Some("")));
    assert!(blank(Some(" \t")));
    assert!(!blank(Some("0")));
    assert!(!blank(Some(" 7 ")));
}

fn state(entry: &RegistryEntry) -> NexusState {
    NexusState {
        registry: Arc::new(vec![entry.clone()]),
        brand: Arc::new("Nexus".to_owned()),
        audit_policy: NexusAuditPolicy::Disabled,
    }
}

fn principal() -> NexusPrincipal {
    NexusPrincipal::authenticated("test-admin")
}

fn form(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// These requests end before a query runs: no database is configured.
#[tokio::test]
async fn composite_record_routes_reject_a_wrong_key_before_the_database() {
    let entry = composite();
    let state = state(&entry);
    let headers = HeaderMap::new();
    for address in ["3", "3,x,y", "03,x", "3,a b"] {
        let response = update_record(
            &state,
            &entry,
            address,
            &principal(),
            None,
            &headers,
            form(&[("percent", "5")]),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{address:?}");
        let response = delete_record(&state, &entry, address, &principal(), None, &headers).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{address:?}");
    }

    // Every key column is read-only on edit.
    for column in ["user_id", "lesson_id"] {
        let response = update_record(
            &state,
            &entry,
            "3,x",
            &principal(),
            None,
            &headers,
            form(&[(column, "4")]),
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{column}"
        );
    }

    // A valid address and form reach the (absent) database.
    let response = update_record(
        &state,
        &entry,
        "3,x",
        &principal(),
        None,
        &headers,
        form(&[("percent", "5")]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let response = delete_record(&state, &entry, "3,x", &principal(), None, &headers).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn creating_a_composite_record_accepts_every_key_column() {
    let entry = composite();
    let state = state(&entry);
    let headers = HeaderMap::new();
    let response = create_record(
        &state,
        &entry,
        &principal(),
        None,
        &headers,
        form(&[("user_id", "x")]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = create_record(
        &state,
        &entry,
        &principal(),
        None,
        &headers,
        form(&[("user_id", "3"), ("lesson_id", "x"), ("percent", "5")]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn fields_bind_their_typed_value_else_their_text_else_null() {
    let entry = RegistryEntry {
        tenant_column: None,
        fields: vec![
            FieldMeta::new("title", "Title", FieldKind::Text),
            FieldMeta::new("due_on", "Due", FieldKind::Date),
            FieldMeta::new("paid_at", "Paid", FieldKind::DateTimeUtc),
            FieldMeta::new("score", "Score", FieldKind::Number),
        ],
        ..single()
    };
    let values = validate_form_values(
        &entry,
        form(&[
            ("title", "Ada"),
            ("due_on", "2026-10-15"),
            ("paid_at", "2026-10-15T09:30"),
            ("score", ""),
        ]),
        FormMode::Update,
    )
    .expect("valid form");
    let bound = |driver| {
        values
            .iter()
            .map(|value| format!("{:?}", bound_value(value, driver)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        bound("postgres"),
        [
            "String(\"Ada\")",
            "Date(2026-10-15)",
            "DateTime(2026-10-15T09:30:00Z)",
            "Null"
        ]
    );
    assert_eq!(bound("mysql")[2], "NaiveDateTime(2026-10-15T09:30:00)");
}

#[tokio::test]
async fn malformed_dates_and_times_are_validation_errors() {
    let entry = RegistryEntry {
        tenant_column: None,
        fields: vec![
            FieldMeta::new("id", "ID", FieldKind::Number),
            FieldMeta::new("due_on", "Due", FieldKind::Date),
            FieldMeta::new("opens_at", "Opens", FieldKind::Time),
            FieldMeta::new("paid_at", "Paid", FieldKind::DateTimeUtc),
        ],
        ..single()
    };
    let state = state(&entry);
    let headers = HeaderMap::new();
    for (field, value, reason) in [
        ("due_on", "2026-02-30", "must use a valid YYYY-MM-DD date"),
        ("due_on", "0000-01-01", "must use a valid YYYY-MM-DD date"),
        (
            "opens_at",
            "25:00",
            "must use a valid HH:MM[:SS[.fraction]] time",
        ),
        (
            "paid_at",
            "2026-10-15T09:30+25:00",
            "must use a valid date-time; one without an offset is UTC",
        ),
    ] {
        let response = update_record(
            &state,
            &entry,
            "7",
            &principal(),
            None,
            &headers,
            form(&[(field, value)]),
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{value}"
        );
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .expect("bounded body");
        assert_eq!(
            String::from_utf8_lossy(&body),
            format!("Invalid Notes form: field `{field}` {reason}")
        );
    }
}
