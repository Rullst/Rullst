use super::*;

#[test]
fn turso_migration_template_is_reversible_and_panic_free() {
    let source = render_migration(
        "m20260829000000_create_widgets",
        "widgets",
        ProjectOrmBackend::Turso,
    );
    assert!(source.contains("TursoMigration::new"));
    assert!(source.contains("CREATE TABLE widgets"));
    assert!(source.contains("DROP TABLE widgets"));
    assert!(!source.contains("unwrap("));
    syn::parse_file(&source).expect("generated Turso migration should parse");
}

#[test]
fn framework_tables_and_comment_only_diffs_do_not_produce_migrations() {
    use super::super::schema_diff::{ParsedField, ParsedTable};
    let field = |name: &str| ParsedField {
        name: name.to_string(),
        rust_type: "String".to_string(),
        is_option: false,
    };
    let posts = |fields: Vec<ParsedField>| ParsedTable {
        table_name: "posts".to_string(),
        struct_name: "Post".to_string(),
        fields,
    };
    let columns = |names: &[&str]| names.iter().map(|name| name.to_string()).collect();
    let mut db_schema = std::collections::HashMap::new();
    db_schema.insert("posts".to_string(), columns(&["id", "title"]));
    db_schema.insert(
        "migrations".to_string(),
        columns(&["id", "migration", "batch"]),
    );
    db_schema.insert("rullst_jobs".to_string(), columns(&["id", "payload"]));

    let synced = vec![posts(vec![field("id"), field("title")])];
    assert!(
        render_auto_migration("m0_auto_sync", &synced, &db_schema)
            .unwrap()
            .is_none()
    );
    assert!(destructive_differences(&synced, &db_schema).is_empty());

    db_schema.insert("legacy".to_string(), columns(&["id"]));
    let removed_column = vec![posts(vec![field("id")])];
    assert!(
        render_auto_migration("m0_auto_sync", &removed_column, &db_schema)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        destructive_differences(&removed_column, &db_schema),
        [
            ("legacy".to_string(), None),
            ("posts".to_string(), Some("title".to_string()))
        ]
    );

    let added = vec![posts(vec![field("id"), field("title"), field("body")])];
    let source = render_auto_migration("m0_auto_sync", &added, &db_schema)
        .unwrap()
        .expect("an added column produces a migration");
    assert!(source.contains("ALTER TABLE posts ADD COLUMN body TEXT NOT NULL DEFAULT ''"));
    assert!(source.contains("// Schema::drop_if_exists(\"legacy\")"));
    assert!(!source.contains("\"migrations\""));
    assert!(!source.contains("rullst_jobs"));
    syn::parse_file(&source).expect("auto migration should parse");
}

#[test]
fn migration_names_keep_leading_m_and_reject_invalid_module_names() {
    assert_eq!(
        migration_snake_name("modify_users_email").unwrap(),
        "modify_users_email"
    );
    assert_eq!(
        migration_snake_name("Migrate-Legacy-Data").unwrap(),
        "migrate_legacy_data"
    );
    for invalid in ["add_index.v2", "", "drop table", "café"] {
        assert!(
            migration_snake_name(invalid).is_err(),
            "{invalid} must be rejected"
        );
    }
    assert!(is_migration_module_name("m20261001000000_modify_users"));
    assert!(!is_migration_module_name("m20261001000000_add_index.v2"));
    assert!(!is_migration_module_name("match"));
}

#[test]
fn auto_migration_for_a_new_table_parses_with_balanced_braces() {
    use super::super::schema_diff::{ParsedField, ParsedTable};
    let field = |name: &str| ParsedField {
        name: name.to_string(),
        rust_type: "String".to_string(),
        is_option: false,
    };
    let tables = vec![ParsedTable {
        table_name: "posts".to_string(),
        struct_name: "Post".to_string(),
        fields: vec![field("id"), field("title"), field("created_at")],
    }];
    let source = render_auto_migration(
        "m20261001000000_auto_sync",
        &tables,
        &std::collections::HashMap::new(),
    )
    .unwrap()
    .expect("a missing table produces a migration");
    assert!(source.contains("Schema::create(\"posts\", |table| {"));
    assert!(source.contains("        }).await?;"));
    assert!(!source.contains("}})"));
    syn::parse_file(&source).expect("generated auto migration should parse");
    assert!(
        render_auto_migration("m0_auto_sync", &[], &std::collections::HashMap::new())
            .unwrap()
            .is_none()
    );
}

#[test]
fn auto_migration_columns_follow_the_rust_field_types() {
    use super::super::schema_diff::{ParsedField, ParsedTable};
    let field = |name: &str, rust_type: &str, is_option: bool| ParsedField {
        name: name.to_string(),
        rust_type: rust_type.to_string(),
        is_option,
    };
    let table = |name: &str, fields: Vec<ParsedField>| ParsedTable {
        table_name: name.to_string(),
        struct_name: "Model".to_string(),
        fields,
    };
    let posts = table(
        "posts",
        vec![
            field("id", "i32", false),
            field("title", "String", false),
            field("views", "i64", false),
            field("rank", "i32", true),
            field("score", "f64", false),
            field("published", "bool", false),
            field("published_at", "NaiveDateTime", true),
        ],
    );
    let source = render_auto_migration(
        "m0_auto_sync",
        std::slice::from_ref(&posts),
        &std::collections::HashMap::new(),
    )
    .unwrap()
    .expect("a missing table produces a migration");
    for line in [
        "table.string(\"title\").not_null();",
        "table.big_integer(\"views\").not_null();",
        "table.integer(\"rank\");",
        "table.float(\"score\").not_null();",
        "table.boolean(\"published\").not_null();",
        "table.string(\"published_at\");",
    ] {
        assert!(source.contains(line), "{line}\n{source}");
    }
    assert!(!source.contains("table.string(\"views\")"));
    syn::parse_file(&source).expect("typed auto migration should parse");

    // Existing rows get a typed backfill for required columns.
    let mut db_schema = std::collections::HashMap::new();
    db_schema.insert(
        "posts".to_string(),
        vec!["id".to_string(), "title".to_string()],
    );
    let source = render_auto_migration("m0_auto_sync", &[posts], &db_schema)
        .unwrap()
        .expect("added columns produce a migration");
    for statement in [
        "ALTER TABLE posts ADD COLUMN views BIGINT NOT NULL DEFAULT 0",
        "ALTER TABLE posts ADD COLUMN rank INTEGER\")",
        "ALTER TABLE posts ADD COLUMN score REAL NOT NULL DEFAULT 0.0",
        "ALTER TABLE posts ADD COLUMN published INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE posts ADD COLUMN published_at TEXT\")",
    ] {
        assert!(source.contains(statement), "{statement}\n{source}");
    }
    assert!(source.contains("// Existing rows receive DEFAULT 0; review before applying."));
    assert!(!source.contains("ADD COLUMN views TEXT"));
    syn::parse_file(&source).expect("typed ALTER migration should parse");

    // Unmappable types and required columns without a neutral value fail
    // before anything is written, naming every offending field.
    let error = render_auto_migration(
        "m0_auto_sync",
        &[
            table(
                "files",
                vec![
                    field("payload", "Vec", false),
                    field("id_ref", "Uuid", true),
                ],
            ),
            table("posts", vec![field("archived_at", "NaiveDateTime", false)]),
        ],
        &db_schema,
    )
    .expect_err("unsupported columns are refused");
    let message = error.to_string();
    assert!(
        message.contains("`files.payload` has type `Vec`"),
        "{message}"
    );
    assert!(
        message.contains("`files.id_ref` has type `Uuid`"),
        "{message}"
    );
    assert!(
        message.contains("`posts.archived_at` is a required `NaiveDateTime`"),
        "{message}"
    );
}
