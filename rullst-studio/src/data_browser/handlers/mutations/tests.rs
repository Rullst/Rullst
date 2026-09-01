use super::*;
use axum::http::StatusCode;
use rullst_orm::_sqlx::Execute;

fn column(name: &str, kind: StudioColumnKind, primary_key: bool, nullable: bool) -> StudioColumn {
    StudioColumn {
        name: name.to_string(),
        kind,
        primary_key,
        nullable,
    }
}

fn bound_sql(value: BoundValue) -> String {
    let mut query = QueryBuilder::<rullst_orm::RullstDatabase>::new("SELECT ");
    push_bound_value(&mut query, value);
    query.build().sql().as_str().to_string()
}

#[test]
fn mutation_values_are_typed_and_bounded() {
    assert!(matches!(
        parse_bound_value(StudioColumnKind::Text, "hello".to_string()),
        Ok(BoundValue::Text(value)) if value == "hello"
    ));
    assert!(matches!(
        parse_bound_value(StudioColumnKind::Integer, " 42 ".to_string()),
        Ok(BoundValue::Integer(42))
    ));
    assert!(matches!(
        parse_bound_value(StudioColumnKind::Float, "2.5".to_string()),
        Ok(BoundValue::Float(value)) if value == 2.5
    ));
    assert!(matches!(
        parse_bound_value(StudioColumnKind::Boolean, "TRUE".to_string()),
        Ok(BoundValue::Boolean(true))
    ));
    assert!(matches!(
        parse_bound_value(StudioColumnKind::Boolean, "0".to_string()),
        Ok(BoundValue::Boolean(false))
    ));

    for invalid in ["", "42.5", "9223372036854775808"] {
        assert!(parse_bound_value(StudioColumnKind::Integer, invalid.to_string()).is_err());
    }
    for invalid in ["NaN", "inf", "-inf", "not-a-number"] {
        assert!(parse_bound_value(StudioColumnKind::Float, invalid.to_string()).is_err());
    }
    assert!(parse_bound_value(StudioColumnKind::Boolean, "yes".to_string()).is_err());
    assert!(parse_bound_value(StudioColumnKind::Unsupported, "x".to_string()).is_err());
    assert!(parse_bound_value(StudioColumnKind::Text, "x".repeat(MAX_CELL_BYTES + 1)).is_err());
    assert!(parse_bound_value(StudioColumnKind::Text, "a\0b".to_string()).is_err());
}

#[test]
fn form_fields_and_composite_keys_are_unique_bounded_and_typed() {
    let mut fields = unique_fields(vec![
        ("pk_tenant".to_string(), "acme".to_string()),
        ("pk_id".to_string(), "7".to_string()),
    ])
    .expect("unique bounded fields");
    let values = take_primary_key(
        &mut fields,
        &[
            column("tenant", StudioColumnKind::Text, true, false),
            column("id", StudioColumnKind::Integer, true, false),
            column("title", StudioColumnKind::Text, false, false),
        ],
    )
    .expect("typed composite primary key");
    assert_eq!(values.len(), 2);
    assert!(fields.is_empty());

    assert!(
        unique_fields(vec![
            ("column".to_string(), "name".to_string()),
            ("column".to_string(), "email".to_string()),
        ])
        .is_err()
    );
    assert!(unique_fields(vec![("x".repeat(81), String::new())]).is_err());
    let excessive = (0..=MAX_FORM_FIELDS)
        .map(|index| (format!("f{index}"), String::new()))
        .collect();
    assert!(unique_fields(excessive).is_err());

    let mut missing = BTreeMap::new();
    assert!(take_required(&mut missing, "column").is_err());
    missing.insert("column".to_string(), String::new());
    assert!(take_required(&mut missing, "column").is_err());
    let mut incomplete = BTreeMap::from([("pk_id".to_string(), "1".to_string())]);
    assert!(
        take_primary_key(
            &mut incomplete,
            &[
                column("id", StudioColumnKind::Integer, true, false),
                column("tenant", StudioColumnKind::Text, true, false),
            ],
        )
        .is_err()
    );
}

#[test]
fn every_bound_value_and_primary_key_remains_parameterized() {
    for value in [
        BoundValue::Text("value".to_string()),
        BoundValue::Integer(42),
        BoundValue::Float(1.5),
        BoundValue::Boolean(true),
        BoundValue::Null(StudioColumnKind::Text),
        BoundValue::Null(StudioColumnKind::Integer),
        BoundValue::Null(StudioColumnKind::Float),
        BoundValue::Null(StudioColumnKind::Boolean),
        BoundValue::Null(StudioColumnKind::Unsupported),
    ] {
        let sql = bound_sql(value);
        assert!(sql.contains('?') || sql.contains('$'));
    }

    let mut query = QueryBuilder::<rullst_orm::RullstDatabase>::new("DELETE FROM records");
    push_primary_key_predicate(
        &mut query,
        "sqlite",
        vec![
            ("tenant".to_string(), BoundValue::Text("acme".to_string())),
            ("id".to_string(), BoundValue::Integer(7)),
        ],
    );
    let sql = query.build().sql().as_str().to_string();
    assert!(sql.contains("WHERE \"tenant\" = "));
    assert!(sql.contains(" AND \"id\" = "));
}

#[test]
fn mutation_failures_have_stable_non_secret_statuses() {
    let cases = [
        (
            MutationFailure::Invalid("invalid"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (MutationFailure::NotFound, StatusCode::NOT_FOUND),
        (MutationFailure::Conflict, StatusCode::CONFLICT),
        (MutationFailure::Database, StatusCode::INTERNAL_SERVER_ERROR),
    ];
    for (failure, expected) in cases {
        assert_eq!(mutation_error_response(failure).status(), expected);
    }
}
