//! Exact bounds of registered model metadata.

use super::*;
use crate::nexus::FieldMeta;

fn reason<T>(result: Result<T, NexusBuildError>) -> Option<&'static str> {
    match result {
        Ok(_) => None,
        Err(NexusBuildError::InvalidModelMetadata { reason }) => Some(reason),
        Err(other) => panic!("unexpected error {other:?}"),
    }
}

fn entry(table: &'static str, fields: Vec<FieldMeta>) -> RegistryEntry {
    RegistryEntry {
        table,
        label: "Records",
        icon: "R",
        pk: "id",
        pk_columns: Vec::new(),
        tenant_column: None,
        fields,
    }
}

fn id() -> FieldMeta {
    FieldMeta::new("id", "ID", FieldKind::Number).readonly()
}

fn static_names(count: usize) -> Vec<&'static str> {
    // Table and field names are `&'static str`: borrow them from a static.
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES
        .get_or_init(|| (0..200).map(|index| format!("n{index}")).collect())
        .iter()
        .take(count)
        .map(String::as_str)
        .collect()
}

#[test]
fn a_registry_holds_at_most_128_models() {
    let registry = |count: usize| {
        static_names(count)
            .into_iter()
            .map(|table| entry(table, vec![id()]))
            .collect::<Vec<_>>()
    };
    assert_eq!(reason(validate_registry(&registry(128))), None);
    assert_eq!(
        reason(validate_registry(&registry(129))),
        Some("the registry exceeds 128 models")
    );
}

#[test]
fn a_model_exposes_between_1_and_128_fields() {
    const FIELDS: Option<&str> = Some("a registered model must expose between 1 and 128 fields");
    let fields = |count: usize| {
        let mut fields = vec![id()];
        fields.extend(
            static_names(count - 1)
                .into_iter()
                .map(|name| FieldMeta::new(name, "Field", FieldKind::Text)),
        );
        fields
    };
    assert_eq!(
        reason(validate_fields(&entry("records", fields(128)))),
        None
    );
    assert_eq!(
        reason(validate_fields(&entry("records", fields(129)))),
        FIELDS
    );
    assert_eq!(
        reason(validate_fields(&entry("records", Vec::new()))),
        FIELDS
    );
}

#[test]
fn enums_and_foreign_keys_are_validated() {
    let options = static_names(129);
    assert_eq!(
        reason(validate_field_kind(&FieldKind::Enum {
            options: options[..128].to_vec()
        })),
        None
    );
    assert_eq!(
        reason(validate_field_kind(&FieldKind::Enum { options })),
        Some("an enum widget must expose between 1 and 128 options")
    );
    assert_eq!(
        reason(validate_field_kind(&FieldKind::ForeignKey {
            table: "bad-table",
            label_col: "name",
        })),
        Some("a foreign-key table is not a bounded ASCII identifier")
    );
    assert_eq!(
        reason(validate_field_kind(&FieldKind::ForeignKey {
            table: "teams",
            label_col: "name;",
        })),
        Some("a foreign-key label column is not a bounded ASCII identifier")
    );
}

#[test]
fn identifiers_start_with_a_letter_or_underscore_and_hold_64_bytes() {
    let check = |value: &str| reason(validate_identifier(value, "bad identifier"));
    assert_eq!(check("_private"), None);
    assert_eq!(check("a1_b"), None);
    assert_eq!(check(&"a".repeat(64)), None);
    for invalid in ["", "1abc", "a-b", "é"] {
        assert_eq!(check(invalid), Some("bad identifier"), "{invalid:?}");
    }
    assert_eq!(check(&"a".repeat(65)), Some("bad identifier"));
}

#[test]
fn display_text_is_non_empty_bounded_and_free_of_controls() {
    let check = |value: &str, maximum| reason(validate_display_text(value, maximum, "bad text"));
    assert_eq!(check("Label", 8), None);
    assert_eq!(check(&"é".repeat(4), 8), None);
    assert_eq!(check(&"é".repeat(5), 8), Some("bad text"));
    assert_eq!(check("", 8), Some("bad text"));
    assert_eq!(check("tab\there", 64), Some("bad text"));
}
