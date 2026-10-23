//! Exact size limits and character rules of Nexus form input.

use super::*;

fn field(kind: FieldKind) -> FieldMeta {
    FieldMeta::new("value", "Value", kind)
}

fn reason(result: Result<(), FormInputError>) -> Option<&'static str> {
    match result {
        Ok(()) => None,
        Err(FormInputError::InvalidField { reason, .. }) => Some(reason),
        Err(other) => panic!("unexpected error {other:?}"),
    }
}

const TOO_LARGE: Option<&str> = Some("exceeds the bounded input size");

#[test]
fn each_kind_accepts_exactly_its_documented_size() {
    let text = field(FieldKind::Text);
    assert_eq!(
        reason(validate_semantic_value(&text, &"t".repeat(4_096))),
        None
    );
    assert_eq!(
        reason(validate_semantic_value(&text, &"t".repeat(4_097))),
        TOO_LARGE
    );

    let textarea = field(FieldKind::Textarea);
    assert_eq!(
        reason(validate_semantic_value(&textarea, &"t".repeat(65_536))),
        None
    );
    assert_eq!(
        reason(validate_semantic_value(&textarea, &"t".repeat(65_537))),
        TOO_LARGE
    );
    let json = field(FieldKind::Json);
    let document = format!("\"{}\"", "j".repeat(65_534));
    assert_eq!(reason(validate_semantic_value(&json, &document)), None);
    let document = format!("\"{}\"", "j".repeat(65_535));
    assert_eq!(reason(validate_semantic_value(&json, &document)), TOO_LARGE);

    let email = field(FieldKind::Email);
    let address = format!("a@{}", "b".repeat(318));
    assert_eq!(address.len(), 320);
    assert_eq!(reason(validate_semantic_value(&email, &address)), None);
    let address = format!("a@{}", "b".repeat(319));
    assert_eq!(reason(validate_semantic_value(&email, &address)), TOO_LARGE);

    let url = field(FieldKind::Url);
    let link = format!("https://example.com/{}", "p".repeat(2_028));
    assert_eq!(link.len(), 2_048);
    assert_eq!(reason(validate_semantic_value(&url, &link)), None);
    let link = format!("https://example.com/{}", "p".repeat(2_029));
    assert_eq!(reason(validate_semantic_value(&url, &link)), TOO_LARGE);
}

#[test]
fn only_multiline_kinds_accept_line_breaks_and_tabs() {
    const CONTROL: Option<&str> = Some("contains a forbidden control character");
    for value in ["a\nb", "a\rb", "a\tb"] {
        assert_eq!(
            reason(validate_semantic_value(&field(FieldKind::Text), value)),
            CONTROL,
            "{value:?}"
        );
        assert_eq!(
            reason(validate_semantic_value(&field(FieldKind::Textarea), value)),
            None,
            "{value:?}"
        );
    }
    assert_eq!(
        reason(validate_semantic_value(
            &field(FieldKind::Textarea),
            "a\u{0}b"
        )),
        CONTROL
    );
}

#[test]
fn integers_need_digits_before_their_range_is_checked() {
    let integer = field(FieldKind::Integer { min: -5, max: 5 });
    for value in ["", "+", "-", "1.5", "1e3", " 1"] {
        assert_eq!(
            reason(validate_integer(&integer, value, -5, 5)),
            Some("must be a whole number"),
            "{value:?}"
        );
    }
    assert_eq!(reason(validate_integer(&integer, "+5", -5, 5)), None);
    assert_eq!(reason(validate_integer(&integer, "-5", -5, 5)), None);
    assert_eq!(
        reason(validate_integer(&integer, "6", -5, 5)),
        Some("is outside the field's integer range")
    );
}

#[test]
fn a_form_holds_at_most_256_values() {
    let entry = RegistryEntry {
        table: "articles",
        label: "Articles",
        icon: "A",
        pk: "id",
        pk_columns: Vec::new(),
        tenant_column: None,
        fields: vec![FieldMeta::new("title", "Title", FieldKind::Text)],
    };
    let pairs = |count: usize| {
        (0..count)
            .map(|_| ("_token".to_string(), "csrf".to_string()))
            .collect::<Vec<_>>()
    };
    assert!(validate_form_values(&entry, pairs(256), FormMode::Create).is_ok());
    assert!(matches!(
        validate_form_values(&entry, pairs(257), FormMode::Create),
        Err(FormInputError::TooManyFields)
    ));
}
