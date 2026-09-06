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
