use super::*;
use crate::generators::auth::models;

const NAME: &str = "m20261029120000_add_account_columns_to_users_table";

/// The string literals of `source` that start with `prefix`.
fn literals(source: &str, prefix: &str) -> Vec<String> {
    struct Strings(Vec<String>);
    impl<'ast> syn::visit::Visit<'ast> for Strings {
        fn visit_lit_str(&mut self, literal: &'ast syn::LitStr) {
            self.0.push(literal.value());
        }
    }
    let file = syn::parse_file(source).expect("migration parses");
    let mut strings = Strings(Vec::new());
    syn::visit::visit_file(&mut strings, &file);
    strings
        .0
        .into_iter()
        .filter(|literal| literal.starts_with(prefix))
        .collect()
}

#[test]
fn only_the_unchanged_starter_users_migration_is_extended() {
    assert!(is_starter_users_migration(STARTER_USERS_MIGRATION));
    // Formatting and comments do not matter.
    let reformatted = STARTER_USERS_MIGRATION
        .replace("    ", "  ")
        .replace("}).await", "})\n        .await")
        .replace("table.id();", "table.id(); // primary key");
    assert!(is_starter_users_migration(&reformatted));

    let with_email = STARTER_USERS_MIGRATION.replace(
        "table.timestamps();",
        "table.string(\"email\").not_null();\n            table.timestamps();",
    );
    let renamed = STARTER_USERS_MIGRATION.replace(
        "m20260601000000_create_users_table",
        "m20260601000000_create_people_table",
    );
    let complete = models::user_migration_source("m20260601000000_create_users_table");
    for other in [
        with_email.as_str(),
        renamed.as_str(),
        complete.as_str(),
        "// starter users table\n",
        "not rust {",
    ] {
        assert!(!is_starter_users_migration(other), "{other}");
    }
}

#[test]
fn the_account_columns_migration_matches_the_complete_users_table() {
    let source = render_account_columns_migration(NAME);
    syn::parse_file(&source).expect("account columns migration parses");
    assert!(!source.contains("__"), "unreplaced placeholder");
    assert!(source.contains(&format!("\"{NAME}\"")));
    // `Orm::pool()` would escape the migration's transaction.
    assert!(source.contains("fn within_transaction(&self) -> bool {\n        true\n    }"));
    assert!(!source.contains("pool()"));
    assert!(!source.contains("rullst_orm"));
    assert!(source.contains("Schema::table(\"users\", |table| {"));
    // Every column the `User` model reads beyond the starter's, declared as
    // the complete users migration declares it (SQLite needs the email
    // default to add a NOT NULL column).
    let complete = models::user_migration_source("m20260601000000_create_users_table");
    for (added, created) in [
        (
            "                .not_null()\n                .default(ColumnDefault::Text(String::new()))\n                .col_type = \"VARCHAR(255)\".to_string();",
            "table.string(\"email\").not_null().col_type = \"VARCHAR(255)\".to_string();",
        ),
        (
            "table.string(\"password_hash\").nullable();",
            "table.string(\"password_hash\").nullable();",
        ),
        (
            "table.string(\"oauth_provider\").nullable();",
            "table.string(\"oauth_provider\").nullable();",
        ),
        (
            "table.string(\"oauth_id\").nullable();",
            "table.string(\"oauth_id\").nullable();",
        ),
        (
            "table.boolean(\"is_admin\").not_null().default(ColumnDefault::Integer(0));",
            "table.boolean(\"is_admin\").not_null().default(ColumnDefault::Integer(0));",
        ),
        (
            "table.big_integer(\"session_epoch\").not_null().default(ColumnDefault::Integer(0));",
            "table.big_integer(\"session_epoch\").not_null().default(ColumnDefault::Integer(0));",
        ),
    ] {
        assert!(source.contains(added), "{added}");
        assert!(complete.contains(created), "{created}");
    }
    assert!(source.contains("table.unique_index(&[\"email\"]);"));
    assert!(complete.contains("CREATE UNIQUE INDEX users_email_unique ON users(email)"));
    // MySQL/MariaDB concatenate with CONCAT; SQLite and PostgreSQL with ||.
    assert_eq!(literals(&source, "UPDATE users SET email = ").len(), 2);
    assert!(source.contains(
        "\"UPDATE users SET email = CONCAT('user-', id, '@no-email.invalid') WHERE email = ''\""
    ));
    assert_eq!(literals(&source, "ALTER TABLE users DROP COLUMN ").len(), 6);
    assert_eq!(
        literals(&source, "DROP INDEX users_email_unique"),
        [
            "DROP INDEX users_email_unique ON users",
            "DROP INDEX users_email_unique"
        ]
    );
}

/// The migration's own SQL, with the `ALTER TABLE` statements `Schema::table`
/// renders for SQLite, keeps existing rows, gives each a distinct placeholder
/// email and rolls back to the starter's table.
#[tokio::test]
async fn sqlite_extends_a_starter_table_that_holds_rows_and_rolls_back() {
    use sqlx::Row;
    let source = render_account_columns_migration(NAME);
    let not_mysql = |statements: Vec<String>| {
        statements
            .into_iter()
            .filter(|statement| !statement.contains("CONCAT") && !statement.ends_with(" ON users"))
            .collect::<Vec<_>>()
    };
    let backfill = not_mysql(literals(&source, "UPDATE users SET email = "));
    let mut down = not_mysql(literals(&source, "DROP INDEX "));
    down.extend(literals(&source, "ALTER TABLE users DROP COLUMN "));
    assert_eq!((backfill.len(), down.len()), (1, 7));

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory SQLite");
    let run = |sql: String| {
        let pool = pool.clone();
        async move {
            sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                .execute(&pool)
                .await
                .unwrap_or_else(|error| panic!("{sql}: {error}"));
        }
    };
    // The starter's table after its first `db:migrate`, with two rows.
    run("CREATE TABLE IF NOT EXISTS \"users\" (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, created_at TEXT DEFAULT CURRENT_TIMESTAMP, updated_at TEXT DEFAULT CURRENT_TIMESTAMP);".into()).await;
    run("INSERT INTO users (name) VALUES ('Ada'), ('Grace')".into()).await;

    for statement in [
        "ALTER TABLE \"users\" ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';",
        "ALTER TABLE \"users\" ADD COLUMN password_hash TEXT;",
        "ALTER TABLE \"users\" ADD COLUMN oauth_provider TEXT;",
        "ALTER TABLE \"users\" ADD COLUMN oauth_id TEXT;",
        "ALTER TABLE \"users\" ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0;",
        "ALTER TABLE \"users\" ADD COLUMN session_epoch BIGINT NOT NULL DEFAULT 0;",
    ] {
        run(statement.into()).await;
    }
    run(backfill[0].clone()).await;
    run("CREATE UNIQUE INDEX IF NOT EXISTS users_email_unique ON \"users\" (email);".into()).await;

    let rows = sqlx::query(
        "SELECT name, email, password_hash, is_admin, session_epoch FROM users ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .expect("extended rows");
    let accounts = rows
        .iter()
        .map(|row| {
            (
                row.get::<String, _>("name"),
                row.get::<String, _>("email"),
                row.get::<Option<String>, _>("password_hash"),
                row.get::<i64, _>("is_admin"),
                row.get::<i64, _>("session_epoch"),
            )
        })
        .collect::<Vec<_>>();
    let expected = |name: &str, id: i64| {
        (
            name.to_string(),
            format!("user-{id}@no-email.invalid"),
            None::<String>,
            0_i64,
            0_i64,
        )
    };
    assert_eq!(accounts, [expected("Ada", 1), expected("Grace", 2)]);
    let duplicate =
        sqlx::query("INSERT INTO users (name, email) VALUES ('Eve', 'user-1@no-email.invalid')")
            .execute(&pool)
            .await;
    assert!(duplicate.is_err(), "emails stay unique");

    for statement in down {
        run(statement).await;
    }
    let columns = sqlx::query("SELECT name FROM pragma_table_info('users') ORDER BY cid")
        .fetch_all(&pool)
        .await
        .expect("starter columns")
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();
    assert_eq!(columns, ["id", "name", "created_at", "updated_at"]);
    let names = sqlx::query("SELECT name FROM users ORDER BY id")
        .fetch_all(&pool)
        .await
        .expect("starter rows")
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();
    assert_eq!(names, ["Ada", "Grace"]);
}

#[test]
fn the_unchanged_starter_user_model_becomes_a_re_export() {
    let directory = tempfile::tempdir().expect("temporary project");
    let manifest = crate::blueprints::blank::file_manifest(
        "shop",
        "shop",
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    );
    let main = manifest
        .iter()
        .find(|(path, _)| *path == "src/main.rs")
        .map(|(_, source)| source.clone())
        .expect("starter main.rs");
    assert!(main.contains(STARTER_USER_MODEL));
    let root = directory.path().join("main.rs");

    for (variant, starter) in [
        ("as generated", main.clone()),
        (
            "imports sorted by rustfmt",
            main.replace("{Orm, FromRow, RowSnapshot}", "{FromRow, Orm, RowSnapshot}"),
        ),
    ] {
        fs::write(&root, &starter).expect("starter root");
        assert!(
            replace_starter_user_model(&root).expect("replace"),
            "{variant}"
        );
        let replaced = fs::read_to_string(&root).expect("replaced root");
        assert!(!replaced.contains("pub struct User"), "{variant}");
        assert!(replaced.contains("\npub use crate::models::user::User;\n"));
        // The status probe keeps compiling against the account model.
        assert!(replaced.contains("User::query().first().await"));
        syn::parse_file(&replaced).expect("replaced root parses");
        // A second run finds nothing to replace.
        assert!(!replace_starter_user_model(&root).expect("second run"));
        assert_eq!(fs::read_to_string(&root).expect("root"), replaced);
    }

    let customized = main.replace(
        "    pub name: String,\n",
        "    pub name: String,\n    pub email: String,\n",
    );
    fs::write(&root, &customized).expect("customized root");
    assert!(!replace_starter_user_model(&root).expect("customized"));
    assert_eq!(fs::read_to_string(&root).expect("root"), customized);
}
