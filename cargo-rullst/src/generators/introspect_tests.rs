#![allow(clippy::expect_used)]

use super::*;

#[test]
fn generated_names_are_safe_and_collisions_fail_closed() {
    assert_eq!(
        safe_snake_identifier("Audit Entries", "table", "table").expect("safe name"),
        "audit_entries"
    );
    assert_eq!(
        safe_snake_identifier("2026-events", "table", "table").expect("safe name"),
        "table_2026_events"
    );
    assert_eq!(
        safe_snake_identifier("type", "field", "column").expect("keyword should normalize"),
        "type_field"
    );
    assert!(plan_tables(&["audit-log".to_string(), "audit log".to_string()]).is_err());
    assert!(matches!(
        plan_tables(&["FooBar".to_string(), "foo_bar".to_string()]),
        Err(IntrospectionError::IdentifierCollision { kind: "table", .. })
    ));
    for invalid in ["", "9table", "table-name", &"x".repeat(65)] {
        assert!(matches!(
            plan_tables(&[invalid.to_string()]),
            Err(IntrospectionError::InvalidIdentifier { kind: "table", .. })
        ));
    }
    assert!(safe_snake_identifier("💣", "field", "column").is_err());
}

#[test]
fn generated_struct_rejects_unsupported_column_remapping() {
    let table = TablePlan {
        database_name: "users".to_string(),
        module_name: "users".to_string(),
        struct_name: "Users".to_string(),
    };
    let error = generate_struct(
        &table,
        &[ColumnInfo {
            name: "type".to_string(),
            data_type: "text".to_string(),
            not_null: true,
        }],
    )
    .expect_err("Rust keyword remapping must fail before writing a broken model");
    assert!(matches!(
        error,
        IntrospectionError::UnsupportedColumnMapping { .. }
    ));

    let code = generate_struct(
        &table,
        &[ColumnInfo {
            name: "account_id".to_string(),
            data_type: "integer".to_string(),
            not_null: false,
        }],
    )
    .expect("conventional identifiers should generate");
    assert!(code.contains("pub account_id: Option<i32>"));
    syn::parse_file(&code).expect("escaped output must remain valid Rust");

    let malicious = TablePlan {
        database_name: "users;DROP_TABLE".to_string(),
        module_name: "users".to_string(),
        struct_name: "Users".to_string(),
    };
    assert!(generate_struct(&malicious, &[]).is_err());

    let collision = generate_struct(
        &table,
        &[
            ColumnInfo {
                name: "foo_bar".to_string(),
                data_type: "text".to_string(),
                not_null: true,
            },
            ColumnInfo {
                name: "fooBar".to_string(),
                data_type: "text".to_string(),
                not_null: true,
            },
        ],
    );
    assert!(matches!(
        collision,
        Err(IntrospectionError::IdentifierCollision { kind: "column", .. })
    ));
    assert!(matches!(
        generate_struct(
            &table,
            &[ColumnInfo {
                name: "bad-name".to_string(),
                data_type: "text".to_string(),
                not_null: true,
            }],
        ),
        Err(IntrospectionError::InvalidIdentifier { kind: "column", .. })
    ));
}

#[test]
fn database_types_map_to_bounded_rust_shapes() {
    for (database_type, expected) in [
        ("INTEGER", "i32"),
        ("bigserial", "i64"),
        ("smallint", "i16"),
        ("tinyint", "i8"),
        ("REAL", "f32"),
        ("double precision", "f64"),
        ("boolean", "bool"),
        ("character varying", "String"),
        ("bytea", "Vec<u8>"),
        ("timestamp without time zone", "String"),
        ("provider_specific", "String"),
    ] {
        assert_eq!(map_db_type_to_rust(database_type, true), expected);
        assert_eq!(
            map_db_type_to_rust(database_type, false),
            format!("Option<{expected}>")
        );
    }
    assert_eq!(snake_to_pascal("audit_event"), "AuditEvent");
    assert_eq!(snake_to_pascal("__audit__event__"), "AuditEvent");
}

#[tokio::test]
async fn sqlite_column_lookup_binds_unusual_table_names() {
    sqlx::any::install_default_drivers();
    let mut connection = AnyConnection::connect("sqlite::memory:")
        .await
        .expect("SQLite should connect");
    sqlx::query(
        "CREATE TABLE \"odd table'); --\" (\"type\" TEXT NOT NULL, \"account-id\" INTEGER)",
    )
    .execute(&mut connection)
    .await
    .expect("unusual test table should be created");

    let columns = get_sqlite_columns(&mut connection, "odd table'); --")
        .await
        .expect("bound pragma lookup should succeed");
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0].name, "type");
    assert!(columns[0].not_null);
    assert_eq!(columns[1].name, "account-id");
}

#[test]
fn postgres_and_mysql_metadata_columns_are_cast_and_aliased() {
    // PostgreSQL 12+ `sql_identifier` (a domain over `name`) cannot be decoded
    // by the Any driver, and MySQL 8 labels unaliased columns in upper case.
    for (sql, cast) in [
        (POSTGRES_TABLES_SQL, "TEXT"),
        (POSTGRES_COLUMNS_SQL, "TEXT"),
        (MYSQL_TABLES_SQL, "CHAR"),
        (MYSQL_COLUMNS_SQL, "CHAR"),
    ] {
        let select_list = sql
            .split_once(" FROM ")
            .map(|(select, _)| select)
            .expect("metadata query has a FROM clause");
        let columns: &[&str] = if sql.contains("information_schema.columns") {
            &["column_name", "data_type", "is_nullable"]
        } else {
            &["table_name"]
        };
        let expressions = select_list
            .trim_start_matches("SELECT ")
            .split(", ")
            .map(str::trim)
            .collect::<Vec<_>>();
        assert_eq!(expressions.len(), columns.len(), "{sql}");
        for (expression, column) in expressions.iter().zip(columns) {
            let source = if cast == "CHAR" {
                column.to_ascii_uppercase()
            } else {
                (*column).to_string()
            };
            assert_eq!(
                *expression,
                format!("CAST({source} AS {cast}) AS {column}"),
                "{sql}"
            );
        }
    }
    assert!(POSTGRES_COLUMNS_SQL.contains("table_name = $1"));
    assert!(MYSQL_COLUMNS_SQL.contains("table_name = ?"));
}

#[tokio::test]
async fn metadata_rows_with_unexpected_labels_or_types_are_errors_not_panics() {
    sqlx::any::install_default_drivers();
    let mut connection = AnyConnection::connect("sqlite::memory:")
        .await
        .expect("SQLite should connect");

    let aliased = sqlx::query(
        "SELECT 'id' AS column_name, 'int' AS data_type, 'NO' AS is_nullable \
         UNION ALL SELECT 'note', 'text', 'YES'",
    )
    .fetch_all(&mut connection)
    .await
    .expect("aliased metadata rows");
    assert_eq!(
        map_information_schema_columns(&aliased).expect("aliased rows map"),
        [
            ColumnInfo {
                name: "id".to_string(),
                data_type: "int".to_string(),
                not_null: true,
            },
            ColumnInfo {
                name: "note".to_string(),
                data_type: "text".to_string(),
                not_null: false,
            },
        ]
    );

    // The labels MySQL 8 returns without aliases: previously `Row::get` panicked.
    let upper_case =
        sqlx::query("SELECT 'id' AS COLUMN_NAME, 'int' AS DATA_TYPE, 'NO' AS IS_NULLABLE")
            .fetch_all(&mut connection)
            .await
            .expect("upper-case metadata rows");
    assert!(matches!(
        map_information_schema_columns(&upper_case),
        Err(sqlx::Error::ColumnNotFound(_))
    ));
    let upper_tables = sqlx::query("SELECT 'users' AS TABLE_NAME")
        .fetch_all(&mut connection)
        .await
        .expect("upper-case table rows");
    assert!(matches!(
        string_column(&upper_tables, "table_name"),
        Err(sqlx::Error::ColumnNotFound(_))
    ));
    let wrong_type = sqlx::query("SELECT 7 AS table_name")
        .fetch_all(&mut connection)
        .await
        .expect("integer table rows");
    assert!(matches!(
        string_column(&wrong_type, "table_name"),
        Err(sqlx::Error::ColumnDecode { .. })
    ));
    // The error surfaces through the command's typed error.
    let error = IntrospectionError::from(
        string_column(&upper_tables, "table_name").expect_err("missing label"),
    );
    assert!(
        error
            .to_string()
            .starts_with("database introspection failed")
    );
}
