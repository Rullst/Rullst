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

#[tokio::test]
#[cfg(not(miri))]
#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
// TM-STUDIO-06: a predicate that matches several rows is rolled back, so the
// conflict response never follows an already committed multi-row change.
async fn row_mutations_commit_only_when_exactly_one_row_changes() {
    let pool = crate::data_browser::pool::test_sqlite_pool().await;
    sqlx::query("DROP TABLE IF EXISTS studio_single_row_probe")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "CREATE TABLE studio_single_row_probe (grp INTEGER NOT NULL, qty INTEGER NOT NULL)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO studio_single_row_probe (grp, qty) VALUES (1, 10), (1, 20), (2, 30)")
        .execute(pool)
        .await
        .unwrap();
    let snapshot = || async {
        sqlx::query_as::<_, (i64, i64)>(
            "SELECT grp, qty FROM studio_single_row_probe ORDER BY grp, qty",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    };
    let original = vec![(1, 10), (1, 20), (2, 30)];
    let update = |group: i64| {
        let mut query = QueryBuilder::<rullst_orm::RullstDatabase>::new(
            "UPDATE studio_single_row_probe SET qty = qty + 1 WHERE grp = ",
        );
        query.push_bind(group);
        query
    };

    let mut shared_group = update(1);
    assert!(matches!(
        execute_single_row_mutation(pool, "sqlite", &mut shared_group).await,
        Err(MutationFailure::Conflict)
    ));
    assert_eq!(snapshot().await, original);

    let mut missing_group = update(9);
    assert!(matches!(
        execute_single_row_mutation(pool, "sqlite", &mut missing_group).await,
        Err(MutationFailure::NotFound)
    ));

    let mut delete_shared = QueryBuilder::<rullst_orm::RullstDatabase>::new(
        "DELETE FROM studio_single_row_probe WHERE grp = ",
    );
    delete_shared.push_bind(1_i64);
    assert!(matches!(
        execute_single_row_mutation(pool, "sqlite", &mut delete_shared).await,
        Err(MutationFailure::Conflict)
    ));
    assert_eq!(snapshot().await, original);

    let mut single_group = update(2);
    assert!(
        execute_single_row_mutation(pool, "sqlite", &mut single_group)
            .await
            .is_ok()
    );
    assert_eq!(snapshot().await, [(1, 10), (1, 20), (2, 31)]);
}

#[tokio::test]
#[cfg(not(miri))]
#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
// SQLite accepts NULL in a non-integer key. Its cell renders as `NULL`, which
// would bind as the text key 'NULL', so such rows must not offer row actions.
async fn rows_with_a_null_key_value_stay_read_only() {
    let pool = crate::data_browser::pool::test_sqlite_pool().await;
    sqlx::query("DROP TABLE IF EXISTS studio_null_key_probe")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE studio_null_key_probe (code TEXT PRIMARY KEY, label TEXT)")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO studio_null_key_probe (code, label) VALUES (NULL, 'missing'), ('NULL', 'literal')")
        .execute(pool)
        .await
        .unwrap();
    let schema = fetch_table_schema(pool, "sqlite", "studio_null_key_probe")
        .await
        .unwrap();
    assert!(schema.supports_mutations());
    let rows = sqlx::query(
        "SELECT CAST(code AS TEXT) AS code, CAST(label AS TEXT) AS label \
         FROM studio_null_key_probe ORDER BY label",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    let html = build_mutable_rows_html(&rows, &schema, "studio_null_key_probe");
    assert_eq!(html.matches("Read-only: NULL key").count(), 1);
    assert_eq!(html.matches("/rows/delete").count(), 1);
    assert!(html.contains("name=\"pk_code\" value=\"NULL\""));
}

#[tokio::test]
#[cfg(not(miri))]
#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
// A key longer than a mutation accepts could not be submitted back, so its row
// offers no actions, and long cells are displayed only up to the bound.
async fn rows_with_keys_longer_than_a_mutation_accepts_stay_read_only() {
    use crate::data_browser::limits::{MAX_DISPLAY_CHARS, bounded_text_expression};

    let pool = crate::data_browser::pool::test_sqlite_pool().await;
    sqlx::query("DROP TABLE IF EXISTS studio_long_key_probe")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE studio_long_key_probe (code TEXT PRIMARY KEY, label TEXT)")
        .execute(pool)
        .await
        .unwrap();
    let mut insert = QueryBuilder::<rullst_orm::RullstDatabase>::new(
        "INSERT INTO studio_long_key_probe VALUES (",
    );
    insert
        .push_bind("k".repeat(MAX_CELL_BYTES + 1))
        .push(", 'long'), (")
        .push_bind("short")
        .push(", ")
        .push_bind(format!("{}tail-marker", "x".repeat(MAX_DISPLAY_CHARS)))
        .push(")");
    insert.build().execute(pool).await.unwrap();
    let schema = fetch_table_schema(pool, "sqlite", "studio_long_key_probe")
        .await
        .unwrap();
    let mut select = QueryBuilder::<rullst_orm::RullstDatabase>::new(format!(
        "SELECT {}, {} FROM studio_long_key_probe ORDER BY label",
        bounded_text_expression("sqlite", "code", MAX_CELL_BYTES + 1),
        bounded_text_expression("sqlite", "label", MAX_DISPLAY_CHARS + 1),
    ));
    let rows = select.build().fetch_all(pool).await.unwrap();

    let html = build_mutable_rows_html(&rows, &schema, "studio_long_key_probe");
    assert_eq!(html.matches("Read-only: key longer than 16 KiB").count(), 1);
    assert_eq!(html.matches("/rows/delete").count(), 1);
    assert!(html.contains("name=\"pk_code\" value=\"short\""));
    assert!(!html.contains("tail-marker"));
    assert!(html.contains(&format!("{}…", "x".repeat(MAX_DISPLAY_CHARS))));
}

#[tokio::test]
#[cfg(not(miri))]
#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
// A key whose rendered text does not bind back to the same value could make a
// row's action change a different row, so such keys offer no actions.
async fn keys_whose_text_does_not_bind_back_stay_read_only() {
    let pool = crate::data_browser::pool::test_sqlite_pool().await;
    // SQLite renders REAL values with 15 significant digits.
    let (rounded, exact) =
        sqlx::query_as::<_, (String, String)>("SELECT CAST(0.3 AS TEXT), CAST(0.1 + 0.2 AS TEXT)")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(rounded, exact);
    for table in ["studio_real_key_probe", "studio_line_key_probe"] {
        QueryBuilder::<rullst_orm::RullstDatabase>::new(format!("DROP TABLE IF EXISTS {table}"))
            .build()
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query("CREATE TABLE studio_real_key_probe (k REAL PRIMARY KEY, note TEXT)")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO studio_real_key_probe VALUES (0.3, 'a'), (0.1 + 0.2, 'b')")
        .execute(pool)
        .await
        .unwrap();
    let real_key = fetch_table_schema(pool, "sqlite", "studio_real_key_probe")
        .await
        .unwrap();
    assert_eq!(real_key.columns[0].kind, StudioColumnKind::Float);
    assert!(!real_key.supports_mutations());

    sqlx::query("CREATE TABLE studio_line_key_probe (code TEXT PRIMARY KEY, label TEXT)")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO studio_line_key_probe VALUES ('x' || char(10) || 'y', 'a-lf'), \
         ('x' || char(13) || 'y', 'b-cr'), ('x' || char(13, 10) || 'y', 'c-crlf'), \
         ('plain', 'd-plain')",
    )
    .execute(pool)
    .await
    .unwrap();
    let schema = fetch_table_schema(pool, "sqlite", "studio_line_key_probe")
        .await
        .unwrap();
    assert!(schema.supports_mutations());
    let rows = sqlx::query(
        "SELECT CAST(code AS TEXT) AS code, CAST(label AS TEXT) AS label \
         FROM studio_line_key_probe ORDER BY label",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    let html = build_mutable_rows_html(&rows, &schema, "studio_line_key_probe");
    assert_eq!(
        html.matches("Read-only: key contains a line break or NUL")
            .count(),
        3
    );
    assert_eq!(html.matches("/rows/delete").count(), 1);
    assert!(html.contains("name=\"pk_code\" value=\"plain\""));
}
