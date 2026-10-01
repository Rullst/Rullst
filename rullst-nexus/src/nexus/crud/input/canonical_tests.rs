//! Values that validation stores in a canonical form.

use super::*;

fn entry(fields: Vec<FieldMeta>) -> RegistryEntry {
    RegistryEntry {
        table: "events",
        label: "Events",
        icon: "E",
        pk: "id",
        tenant_column: None,
        fields,
    }
}

fn stored(entry: &RegistryEntry, field: &str, value: &str) -> Option<String> {
    validate_form_values(
        entry,
        vec![(field.to_owned(), value.to_owned())],
        FormMode::Update,
    )
    .unwrap_or_else(|error| panic!("{field} rejected {value}: {error}"))
    .remove(0)
    .value
}

#[test]
fn local_date_times_are_stored_in_the_current_timestamp_text_form() {
    let entry = entry(vec![FieldMeta::new(
        "publish_at",
        "Publish at",
        FieldKind::DateTime,
    )]);
    for (submitted, expected) in [
        // What a `datetime-local` input submits.
        ("2026-10-01T08:00", "2026-10-01 08:00:00"),
        ("2026-10-01T08:00:30", "2026-10-01 08:00:30"),
        ("2026-10-01T08:00:30.125", "2026-10-01 08:00:30.125"),
        // The stored form, retyped in the text widget.
        ("2026-10-01 08:00", "2026-10-01 08:00:00"),
        ("2026-10-01 08:00:00.123456", "2026-10-01 08:00:00.123456"),
        // A value with an offset is kept as entered.
        ("2026-10-01T08:00:00Z", "2026-10-01T08:00:00Z"),
        ("2026-10-01 08:00:00+00:00", "2026-10-01 08:00:00+00:00"),
        ("2026-10-01T08:00-03:30", "2026-10-01T08:00-03:30"),
    ] {
        assert_eq!(
            stored(&entry, "publish_at", submitted).as_deref(),
            Some(expected),
            "{submitted}"
        );
    }

    // Same-day text comparisons now agree with `CURRENT_TIMESTAMP` rows.
    let edited = stored(&entry, "publish_at", "2026-10-01T08:00").unwrap_or_default();
    assert!(edited.as_str() <= "2026-10-01 09:00:00");
    assert!(edited.as_str() > "2026-10-01 07:59:59");
}

#[test]
fn integer_fields_accept_only_whole_numbers_in_range() {
    let entry = entry(vec![
        FieldMeta::new(
            "stock",
            "Stock",
            FieldKind::Integer {
                min: i64::from(i32::MIN),
                max: i64::from(i32::MAX),
            },
        ),
        FieldMeta::new(
            "views",
            "Views",
            FieldKind::Integer {
                min: 0,
                max: i64::from(u32::MAX),
            },
        ),
        FieldMeta::new("ratio", "Ratio", FieldKind::Number),
    ]);
    for (submitted, expected) in [
        ("42", "42"),
        ("+7", "7"),
        ("007", "7"),
        ("-0", "0"),
        ("-2147483648", "-2147483648"),
        ("2147483647", "2147483647"),
    ] {
        assert_eq!(
            stored(&entry, "stock", submitted).as_deref(),
            Some(expected),
            "{submitted}"
        );
    }
    assert_eq!(
        stored(&entry, "views", "4294967295").as_deref(),
        Some("4294967295")
    );

    for (field, value) in [
        ("stock", "2.5"),
        ("stock", "2.0"),
        ("stock", "1e3"),
        ("stock", "3000000000"),
        ("stock", "-2147483649"),
        ("stock", "99999999999999999999"),
        ("stock", " 5"),
        ("stock", "5 "),
        ("stock", "--1"),
        ("stock", "+"),
        ("stock", "0x10"),
        ("stock", "١٢"),
        ("views", "-1"),
        ("views", "4294967296"),
    ] {
        assert!(
            validate_form_values(
                &entry,
                vec![(field.to_owned(), value.to_owned())],
                FormMode::Update,
            )
            .is_err(),
            "{field} accepted {value}"
        );
    }
    // A floating-point field keeps accepting fractions.
    assert_eq!(stored(&entry, "ratio", "2.5").as_deref(), Some("2.5"));
}
