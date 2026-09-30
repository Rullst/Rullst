use super::*;

fn widget(kind: FieldKind, stored: StoredValue) -> String {
    render_field_widget(&FieldMeta::new("field", "Field", kind), &stored, true, "id")
}

fn value(text: &str) -> StoredValue {
    StoredValue::Value(text.to_owned())
}

#[test]
fn password_widgets_are_never_prefilled() {
    let field = FieldMeta::new("api_key", "API key", FieldKind::Password);
    for is_edit in [false, true] {
        let html = render_field_widget(&field, &value("stored-sample-value"), is_edit, "id");
        assert!(html.contains("type=\"password\""));
        assert!(html.contains("value=\"\""));
        assert!(html.contains("autocomplete=\"new-password\""));
        assert!(!html.contains("stored-sample-value"));
        assert_eq!(html.contains("Leave blank"), is_edit);
    }
}

#[test]
fn null_and_unreadable_values_render_empty_with_an_explanation() {
    for kind in [
        FieldKind::Number,
        FieldKind::Text,
        FieldKind::Date,
        FieldKind::DateTime,
    ] {
        let html = widget(kind.clone(), StoredValue::Null);
        assert!(html.contains("value=\"\""), "{kind:?}");
        assert!(html.contains("placeholder=\"NULL\""), "{kind:?}");
        let html = widget(kind.clone(), StoredValue::Unreadable);
        assert!(html.contains("value=\"\""), "{kind:?}");
        assert!(html.contains("Stored value not shown"), "{kind:?}");
    }
    let html = widget(FieldKind::Json, StoredValue::Null);
    assert!(html.contains("placeholder=\"NULL\"></textarea>"));
    let html = widget(FieldKind::Boolean, StoredValue::Null);
    assert!(!html.contains("checked"));
}

#[test]
fn unrepresentable_values_use_a_text_input_instead_of_being_blanked() {
    let offset = "2026-01-01T10:00:00+00:00";
    let html = widget(FieldKind::DateTime, value(offset));
    assert!(html.contains(&format!("type=\"text\" name=\"field\" value=\"{offset}\"")));

    let html = widget(FieldKind::DateTime, value("2026-01-01 10:00:00"));
    assert!(html.contains("type=\"datetime-local\" name=\"field\" value=\"2026-01-01T10:00:00\""));

    let html = widget(FieldKind::Date, value("2026-01-01"));
    assert!(html.contains("type=\"date\" name=\"field\" value=\"2026-01-01\""));
    let html = widget(FieldKind::Date, value("01/02/2026"));
    assert!(html.contains("type=\"text\" name=\"field\" value=\"01/02/2026\""));

    for number in ["42", "-1.5", "2e10", "0.25E-3"] {
        let html = widget(FieldKind::Number, value(number));
        assert!(html.contains("type=\"number\""), "{number}");
    }
    for text in ["+1", "1.", ".5", "NaN", "12 apples", "1e"] {
        let html = widget(FieldKind::Number, value(text));
        assert!(html.contains("type=\"text\""), "{text}");
    }

    let html = widget(FieldKind::Email, value(" padded@example.com"));
    assert!(html.contains("type=\"text\""));
    let html = widget(FieldKind::Email, value("ada@example.com"));
    assert!(html.contains("type=\"email\""));
}

#[test]
fn enum_keeps_null_and_unregistered_values_without_submitting_a_default() {
    let kind = FieldKind::Enum {
        options: vec!["draft", "published"],
    };
    let html = widget(kind.clone(), value("legacy"));
    assert!(html.contains(
        "<option value=\"legacy\" selected disabled>legacy (not a registered option)</option>"
    ));
    assert!(html.contains("<option value=\"draft\">draft</option>"));

    let html = widget(kind.clone(), StoredValue::Null);
    assert!(html.contains("<option value=\"\" selected>NULL</option>"));
    assert!(html.contains("<option value=\"draft\">draft</option>"));

    let html = widget(kind.clone(), value("published"));
    assert!(html.contains("<option value=\"published\" selected>published</option>"));
    assert_eq!(html.matches("selected").count(), 1);

    // A new record keeps the first registered option as its default.
    let html = render_field_widget(
        &FieldMeta::new("field", "Field", kind),
        &StoredValue::Absent,
        false,
        "id",
    );
    assert!(!html.contains("selected"));
    assert!(
        html.starts_with("<select name=\"field\" class=\"nexus-input\"><option value=\"draft\">")
    );
}

#[tokio::test]
async fn forms_declare_their_mode_for_change_only_submission() {
    let entry = RegistryEntry {
        table: "articles",
        label: "Articles",
        icon: "A",
        pk: "id",
        tenant_column: None,
        fields: vec![FieldMeta::new("title", "Title", FieldKind::Text)],
    };
    let state = NexusState {
        registry: std::sync::Arc::new(vec![entry.clone()]),
        brand: std::sync::Arc::new("Nexus".to_owned()),
        audit_policy: crate::nexus::NexusAuditPolicy::Disabled,
    };
    let create = render_record_form(&state, &entry, None, None).await;
    assert!(create.contains("data-nexus-mode=\"create\""));
    let edit = form_html(&entry, Some("7"), None);
    assert!(edit.contains("data-nexus-mode=\"edit\""));
    assert!(edit.contains("data-nexus-action=\"/nexus/table/articles/7\""));
}

#[tokio::test]
async fn unloadable_records_render_an_error_instead_of_an_empty_edit_form() {
    let entry = RegistryEntry {
        table: "articles",
        label: "Articles",
        icon: "A",
        pk: "id",
        tenant_column: None,
        fields: vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("title", "Title", FieldKind::Text),
        ],
    };
    let state = NexusState {
        registry: std::sync::Arc::new(vec![entry.clone()]),
        brand: std::sync::Arc::new("Nexus".to_owned()),
        audit_policy: crate::nexus::NexusAuditPolicy::Disabled,
    };
    // A non-canonical numeric key names no record, before any query runs.
    assert_eq!(
        record_form(&state, &entry, Some("+7"), None).await,
        Err(RecordFormError::NotFound)
    );
    let html = render_record_form(&state, &entry, Some("+7"), None).await;
    assert_eq!(html, "<p class=\"nexus-error\">Record not found.</p>");
    assert!(!html.contains("<form"));
}

#[test]
fn readonly_checkboxes_and_selects_are_disabled_rather_than_interactive() {
    let enum_kind = FieldKind::Enum {
        options: vec!["draft", "published"],
    };
    for (kind, stored) in [(FieldKind::Boolean, "1"), (enum_kind, "draft")] {
        let field = FieldMeta::new("locked", "Locked", kind.clone()).readonly();
        let html = render_field_widget(&field, &value(stored), true, "id");
        assert!(!html.contains("name=\"locked\""), "{kind:?}");
        assert!(html.contains(" disabled"), "{kind:?}");
        assert!(!html.contains(" readonly"), "{kind:?}");

        let editable = FieldMeta::new("open", "Open", kind.clone());
        let html = render_field_widget(&editable, &value(stored), true, "id");
        assert!(html.contains("name=\"open\""), "{kind:?}");
        assert!(!html.contains("disabled"), "{kind:?}");
    }
    let text = FieldMeta::new("title", "Title", FieldKind::Text).readonly();
    let html = render_field_widget(&text, &value("kept"), true, "id");
    assert!(html.contains(" readonly") && !html.contains("disabled"));
}
