use super::*;

fn entry() -> RegistryEntry {
    RegistryEntry {
        table: "articles",
        label: "Articles",
        icon: "A",
        pk: "id",
        tenant_column: None,
        fields: vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("body", "Body", FieldKind::Textarea),
            FieldMeta::new("active", "Active", FieldKind::Boolean),
            FieldMeta::new(
                "status",
                "Status",
                FieldKind::Enum {
                    options: vec!["draft", "published"],
                },
            ),
            FieldMeta::new("metadata", "Metadata", FieldKind::Json),
        ],
    }
}

fn semantic_entry() -> RegistryEntry {
    RegistryEntry {
        table: "profiles",
        label: "Profiles",
        icon: "P",
        pk: "id",
        tenant_column: None,
        fields: vec![
            FieldMeta::new("id", "ID", FieldKind::Number),
            FieldMeta::new("name", "Name", FieldKind::Text),
            FieldMeta::new("email", "Email", FieldKind::Email),
            FieldMeta::new("website", "Website", FieldKind::Url),
            FieldMeta::new("score", "Score", FieldKind::Number),
            FieldMeta::new("birthday", "Birthday", FieldKind::Date),
            FieldMeta::new("meeting", "Meeting", FieldKind::DateTime),
            FieldMeta::new("password", "Password", FieldKind::Password),
            FieldMeta::new(
                "team_id",
                "Team",
                FieldKind::ForeignKey {
                    table: "teams",
                    label_col: "name",
                },
            ),
            FieldMeta::new("secret", "Secret", FieldKind::Text).hidden(),
        ],
    }
}
#[test]
fn validates_and_normalizes_semantic_widgets() {
    let entry = entry();
    let values = validate_form_values(
        &entry,
        vec![
            ("body".to_string(), "first\nsecond".to_string()),
            ("active".to_string(), "0".to_string()),
            ("active".to_string(), "1".to_string()),
            ("status".to_string(), "published".to_string()),
            ("metadata".to_string(), "{\"safe\":true}".to_string()),
        ],
        FormMode::Create,
    )
    .expect("valid registered form");

    assert_eq!(values.len(), 4);
    assert_eq!(values[1].value, "1");
    assert_eq!(values[2].value, "published");
}

#[test]
fn rejects_unregistered_enum_duplicate_and_protected_fields() {
    let entry = entry();
    for pairs in [
        vec![("status".to_string(), "administrator".to_string())],
        vec![
            ("status".to_string(), "draft".to_string()),
            ("status".to_string(), "published".to_string()),
        ],
        vec![("id".to_string(), "7".to_string())],
        vec![("unknown".to_string(), "value".to_string())],
    ] {
        assert!(validate_form_values(&entry, pairs, FormMode::Update).is_err());
    }
}

#[test]
fn rejects_oversized_text_invalid_json_and_malformed_boolean() {
    let entry = entry();
    for pairs in [
        vec![("body".to_string(), "x".repeat(MAX_LONG_TEXT_BYTES + 1))],
        vec![("metadata".to_string(), "{invalid".to_string())],
        vec![("active".to_string(), "yes".to_string())],
    ] {
        assert!(validate_form_values(&entry, pairs, FormMode::Create).is_err());
    }
}

#[test]
fn validates_all_registered_semantic_field_kinds() {
    let entry = semantic_entry();
    let values = validate_form_values(
        &entry,
        vec![
            ("name".to_owned(), "Ada".to_owned()),
            ("email".to_owned(), "ada@example.com".to_owned()),
            ("website".to_owned(), "https://rullst.dev/ada".to_owned()),
            ("score".to_owned(), "9.75".to_owned()),
            ("birthday".to_owned(), "2000-02-29".to_owned()),
            ("meeting".to_owned(), "2026-09-01T23:59:59.123".to_owned()),
            ("password".to_owned(), "bounded secret".to_owned()),
            ("team_id".to_owned(), "42".to_owned()),
        ],
        FormMode::Create,
    )
    .expect("all registered semantic values");
    assert_eq!(values.len(), 8);
    assert_eq!(values[0].field.name, "name");
    assert_eq!(values[2].value, "https://rullst.dev/ada");

    let empty = validate_form_values(
        &entry,
        vec![("name".to_owned(), String::new())],
        FormMode::Create,
    )
    .expect("optional empty value");
    assert_eq!(empty[0].value, "");
}

#[test]
fn rejects_invalid_email_url_number_date_and_datetime_values() {
    let entry = semantic_entry();
    for (field, value) in [
        ("email", "missing-at.example.com"),
        ("email", "two@@example.com"),
        ("email", "space @example.com"),
        ("website", "relative/path"),
        ("website", "ftp://example.com"),
        ("website", "https://user:secret@example.com"),
        ("score", "NaN"),
        ("score", "not-a-number"),
        ("birthday", "1900-02-29"),
        ("birthday", "2026-13-01"),
        ("birthday", "2026-04-31"),
        ("birthday", "26-01-01"),
        ("meeting", "2026-09-01 12:00"),
        ("meeting", "2026-09-01T24:00"),
        ("meeting", "2026-09-01T12:60"),
        ("meeting", "2026-09-01T12:30:60"),
        ("meeting", "2026-09-01T12:30:01."),
        ("meeting", "2026-09-01T12:30:01.1234567890"),
    ] {
        assert!(
            validate_form_values(
                &entry,
                vec![(field.to_owned(), value.to_owned())],
                FormMode::Create,
            )
            .is_err(),
            "{field} accepted {value}"
        );
    }
}

#[test]
fn boolean_normalization_accepts_html_forms_and_rejects_ambiguous_duplicates() {
    let field = FieldMeta::new("active", "Active", FieldKind::Boolean);
    for (raw, expected) in [
        ("true", "1"),
        ("TRUE", "1"),
        ("on", "1"),
        ("1", "1"),
        ("false", "0"),
        ("OFF", "0"),
        ("0", "0"),
    ] {
        let normalized = normalize_values(&field, vec![raw.to_owned()]).unwrap();
        assert_eq!(normalized.value, expected);
    }
    assert_eq!(
        normalize_values(&field, vec!["false".to_owned(), "true".to_owned()])
            .unwrap()
            .value,
        "1"
    );
    assert!(normalize_values(&field, vec!["true".to_owned(), "false".to_owned()]).is_err());
    assert!(normalize_values(&field, Vec::new()).is_err());
    assert!(validate_semantic_value(&field, "true").is_err());
}

#[test]
fn bounds_controls_and_protected_metadata_fail_closed() {
    let profile_entry = semantic_entry();
    let too_many = (0..=MAX_FORM_PAIRS)
        .map(|_| ("name".to_owned(), "value".to_owned()))
        .collect();
    assert!(matches!(
        validate_form_values(&profile_entry, too_many, FormMode::Create),
        Err(FormInputError::TooManyFields)
    ));

    for pairs in [
        vec![("secret".to_owned(), "exposed".to_owned())],
        vec![("id".to_owned(), "7".to_owned())],
        vec![("name".to_owned(), "control\0value".to_owned())],
        vec![("name".to_owned(), "x".repeat(MAX_SHORT_TEXT_BYTES + 1))],
        vec![("email".to_owned(), "x".repeat(MAX_EMAIL_BYTES + 1))],
        vec![("website".to_owned(), "x".repeat(MAX_URL_BYTES + 1))],
    ] {
        assert!(validate_form_values(&profile_entry, pairs, FormMode::Update).is_err());
    }

    let article_entry = entry();
    let multiline = validate_form_values(
        &article_entry,
        vec![("body".to_owned(), "line one\r\nline two\tvalue".to_owned())],
        FormMode::Create,
    )
    .expect("safe multiline controls");
    assert!(multiline[0].value.contains("line two"));
}

#[test]
fn leap_year_rules_and_error_messages_are_stable() {
    let field = FieldMeta::new("date", "Date", FieldKind::Date);
    for valid in ["2000-02-29", "2024-02-29", "2026-04-30", "2026-12-31"] {
        assert!(validate_date(&field, valid).is_ok(), "rejected {valid}");
    }
    for invalid_date in ["2100-02-29", "2026-02-29", "2026-11-31", "2026-00-10"] {
        assert!(
            validate_date(&field, invalid_date).is_err(),
            "accepted {invalid_date}"
        );
    }

    let errors = [
        FormInputError::TooManyFields,
        FormInputError::UnknownOrProtectedField,
        FormInputError::DuplicateField { field: "name" },
        FormInputError::InvalidField {
            field: "email",
            reason: "must be valid",
        },
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}
