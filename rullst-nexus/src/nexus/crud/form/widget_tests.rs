//! Boolean, enum and URL widgets for stored values.

use super::*;

fn widget(kind: FieldKind, stored: StoredValue) -> String {
    render_field_widget(
        &FieldMeta::new("field", "Field", kind),
        &stored,
        true,
        &["id"],
    )
}

#[test]
fn boolean_widgets_are_checked_for_one_or_true() {
    for (stored, checked) in [("1", true), ("true", true), ("0", false), ("yes", false)] {
        let html = widget(FieldKind::Boolean, StoredValue::Value(stored.to_string()));
        assert_eq!(html.contains(" checked"), checked, "{stored}");
    }
}

#[test]
fn an_unreadable_enum_value_is_selected_but_never_submitted() {
    let html = widget(
        FieldKind::Enum {
            options: vec!["draft", "published"],
        },
        StoredValue::Unreadable,
    );
    assert!(
        html.contains("<option value=\"\" selected disabled>Stored value not shown</option>"),
        "{html}"
    );
}

#[test]
fn url_inputs_hold_only_values_the_browser_keeps_unchanged() {
    assert_eq!(
        typed_input(&FieldKind::Url, "https://example.com/a"),
        ("url", "https://example.com/a".to_string())
    );
    for padded in [" https://example.com/a", "https://example.com/a\n"] {
        assert_eq!(
            typed_input(&FieldKind::Url, padded),
            ("text", padded.to_string()),
            "{padded:?}"
        );
    }
}
