//! `Schema::table` statements per driver, and the DDL execution helpers that
//! must fail before touching a database.
#![allow(clippy::expect_used)]

use super::super::schema_builder::{execute_statements, prepare_postgres_enums};
use super::super::{Blueprint, ColumnDefault, DatabaseEnum, ForeignKeyAction, Schema};
use crate::Error;

/// Additions to an existing `posts` table.
fn additions() -> Blueprint {
    let mut table = Blueprint::new();
    table.integer("author_id");
    table
        .foreign("author_id")
        .references("users", "id")
        .on_delete(ForeignKeyAction::SetNull);
    table.string("code").unique().col_type = "VARCHAR(32)".to_string();
    table.unique(&["code", "author_id"]);
    table.index(&["author_id"]);
    table
}

#[test]
fn postgres_and_mysql_alter_tables_with_add_constraint() {
    let mut postgres = additions();
    postgres.check("author_id > 0");
    assert_eq!(
        postgres
            .alter_statements("posts", "postgres", false)
            .expect("PostgreSQL alter"),
        [
            "ALTER TABLE \"posts\" ADD COLUMN author_id INTEGER;",
            "ALTER TABLE \"posts\" ADD COLUMN code VARCHAR(32) UNIQUE;",
            "ALTER TABLE \"posts\" ADD CONSTRAINT posts_code_author_id_unique UNIQUE (code, author_id);",
            "CREATE INDEX IF NOT EXISTS posts_author_id_index ON \"posts\" (author_id);",
            "ALTER TABLE \"posts\" ADD CONSTRAINT posts_author_id_foreign FOREIGN KEY (author_id) REFERENCES \"users\" (id) ON DELETE SET NULL;",
            "ALTER TABLE \"posts\" ADD CHECK (author_id > 0);",
        ]
    );
    assert_eq!(
        postgres
            .alter_statements("posts", "mysql", false)
            .expect("MySQL alter"),
        [
            "ALTER TABLE `posts` ADD COLUMN author_id INTEGER;",
            "ALTER TABLE `posts` ADD COLUMN code VARCHAR(32) UNIQUE;",
            "ALTER TABLE `posts` ADD CONSTRAINT posts_code_author_id_unique UNIQUE (code, author_id);",
            "CREATE INDEX posts_author_id_index ON `posts` (author_id);",
            "ALTER TABLE `posts` ADD CONSTRAINT posts_author_id_foreign FOREIGN KEY (author_id) REFERENCES `users` (id) ON DELETE SET NULL;",
            "ALTER TABLE `posts` ADD CHECK (author_id > 0);",
        ]
    );
}

#[test]
fn sqlite_alters_tables_with_column_references_and_unique_indexes() {
    let mut table = additions();
    table
        .foreign("reviewer_id")
        .references("users", "id")
        .name("posts_reviewer_fk");
    table.integer("reviewer_id");
    assert_eq!(
        table
            .alter_statements("posts", "sqlite", false)
            .expect("SQLite alter"),
        [
            "ALTER TABLE \"posts\" ADD COLUMN author_id INTEGER REFERENCES \"users\" (id) ON DELETE SET NULL;",
            "ALTER TABLE \"posts\" ADD COLUMN code VARCHAR(32);",
            "ALTER TABLE \"posts\" ADD COLUMN reviewer_id INTEGER CONSTRAINT posts_reviewer_fk REFERENCES \"users\" (id);",
            "CREATE UNIQUE INDEX IF NOT EXISTS posts_code_unique ON \"posts\" (code);",
            "CREATE UNIQUE INDEX IF NOT EXISTS posts_code_author_id_unique ON \"posts\" (code, author_id);",
            "CREATE INDEX IF NOT EXISTS posts_author_id_index ON \"posts\" (author_id);",
        ]
    );
}

#[test]
fn sqlite_alter_returns_a_typed_error_for_changes_it_cannot_make() {
    let unsupported: [fn(&mut Blueprint); 8] = [
        |table| {
            table.foreign("author_id").references("users", "id");
        },
        |table| {
            table.check("author_id > 0");
        },
        |table| {
            table.integer("score").primary();
        },
        |table| {
            table.integer("score").not_null();
        },
        |table| {
            table.timestamps();
        },
        |table| {
            table
                .integer("author_id")
                .default(ColumnDefault::Integer(1));
            table.foreign("author_id").references("users", "id");
        },
        |table| {
            table
                .timestamp_tz("seen_at")
                .default(ColumnDefault::CurrentTimestamp);
        },
        |table| {
            table.integer("author_id");
            table.foreign("author_id").references("users", "id");
            table.foreign("author_id").references("authors", "id");
        },
    ];
    for declare in unsupported {
        let mut table = Blueprint::new();
        declare(&mut table);
        assert!(
            matches!(
                table.alter_statements("posts", "sqlite", false),
                Err(Error::UnsupportedSchemaChange { ref driver, .. }) if driver == "sqlite"
            ),
            "SQLite must refuse the change"
        );
    }
    // A second foreign key on one added column is refused, not dropped:
    // nothing is returned to run.
    let mut twice = Blueprint::new();
    twice.integer("author_id");
    twice
        .foreign("author_id")
        .references("users", "id")
        .name("posts_author_user_fk");
    twice
        .foreign("author_id")
        .references("authors", "id")
        .name("posts_author_author_fk");
    assert!(matches!(
        twice.alter_statements("posts", "sqlite", false),
        Err(Error::UnsupportedSchemaChange { ref driver, ref change })
            if driver == "sqlite" && change.contains("more than one foreign key") && change.contains("author_id")
    ));
    // PostgreSQL adds both keys.
    assert_eq!(
        twice
            .alter_statements("posts", "postgres", false)
            .expect("PostgreSQL keeps both keys")
            .len(),
        3
    );

    // A NOT NULL column with a default can be added.
    let mut table = Blueprint::new();
    table
        .integer("score")
        .not_null()
        .default(ColumnDefault::Integer(0));
    assert_eq!(
        table
            .alter_statements("posts", "sqlite", false)
            .expect("NOT NULL with a default"),
        ["ALTER TABLE \"posts\" ADD COLUMN score INTEGER NOT NULL DEFAULT 0;"]
    );
    // PostgreSQL adds the same foreign key to an existing column.
    let mut table = Blueprint::new();
    table.foreign("author_id").references("users", "id");
    assert_eq!(
        table
            .alter_statements("posts", "postgres", false)
            .expect("PostgreSQL foreign key"),
        [
            "ALTER TABLE \"posts\" ADD CONSTRAINT posts_author_id_foreign FOREIGN KEY (author_id) REFERENCES \"users\" (id);"
        ]
    );
}

#[tokio::test]
async fn schema_table_validates_the_table_name_before_connecting() {
    let result = Schema::table("posts; DROP TABLE users", |table| {
        table.integer("score");
    })
    .await;
    assert!(matches!(result, Err(Error::Internal(_))), "{result:?}");
}

#[tokio::test]
async fn ddl_statements_run_only_on_an_initialized_orm() {
    // Library tests never initialize the global ORM.
    assert!(matches!(
        execute_statements(&["SELECT 1".to_string()]).await,
        Err(Error::NotInitialized)
    ));
    assert!(execute_statements(&[]).await.is_ok());
}

struct Status;

impl DatabaseEnum for Status {
    const TYPE_NAME: &'static str = "contract_status";
    const VARIANTS: &'static [&'static str] = &["draft", "live"];
}

struct OtherStatus;

impl DatabaseEnum for OtherStatus {
    const TYPE_NAME: &'static str = "contract_status";
    const VARIANTS: &'static [&'static str] = &["other"];
}

#[tokio::test]
async fn postgres_enum_preparation_validates_before_any_statement() {
    let mut conflicting = Blueprint::new();
    conflicting.native_enum::<Status>("status");
    conflicting.native_enum::<OtherStatus>("previous_status");
    assert!(matches!(
        prepare_postgres_enums(&conflicting, "postgres").await,
        Err(Error::Validation(_))
    ));
    // Other drivers have no enum type to prepare.
    assert!(prepare_postgres_enums(&conflicting, "sqlite").await.is_ok());
    assert!(prepare_postgres_enums(&conflicting, "mysql").await.is_ok());
    assert!(
        prepare_postgres_enums(&Blueprint::new(), "postgres")
            .await
            .is_ok()
    );
    // The Any pool refuses native enums; a strict pool needs the database.
    let mut valid = Blueprint::new();
    valid.native_enum::<Status>("status");
    assert!(prepare_postgres_enums(&valid, "postgres").await.is_err());
}
