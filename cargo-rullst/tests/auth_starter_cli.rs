//! `cargo rullst auth` on freshly generated starters: Blank and API keep the
//! starter's users migration and add the account columns in a new one; SaaS
//! already has accounts and is left unchanged. No project is built.

#![allow(clippy::expect_used, clippy::panic)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const STARTER_USERS: &str = "src/migrations/m20260601000000_create_users_table.rs";
const DATABASES: [&str; 3] = ["sqlite", "postgres", "mysql"];

fn rullst(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(directory)
        .args(arguments)
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|error| panic!("run {arguments:?}: {error}"))
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn succeeds(directory: &Path, arguments: &[&str]) -> String {
    let output = rullst(directory, arguments);
    let text = text(&output);
    assert!(output.status.success(), "{arguments:?} failed:\n{text}");
    text
}

/// Generates `blueprint` with `database` and returns the project root.
fn starter(parent: &Path, blueprint: &str, database: &str) -> PathBuf {
    let name = format!("{blueprint}-{database}");
    succeeds(
        parent,
        &[
            "new",
            &name,
            "--default",
            "--blueprint",
            blueprint,
            "--database",
            database,
            "--skip-initial-migration",
            "--vcs",
            "none",
        ],
    );
    parent.join(name)
}

/// Every file of the project, by relative path.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(directory).expect("project directory") {
            let path = entry.expect("project entry").path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                let relative = path.strip_prefix(root).expect("project file").to_path_buf();
                files.insert(relative, fs::read(&path).expect("project file contents"));
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

fn migrations_ending_with(root: &Path, suffix: &str) -> Vec<String> {
    let mut names = fs::read_dir(root.join("src/migrations"))
        .expect("migrations directory")
        .map(|entry| {
            entry
                .expect("migration")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .filter(|name| name.ends_with(suffix))
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn auth_extends_the_users_table_of_fresh_blank_and_api_starters() {
    let parent = tempfile::tempdir().expect("temporary directory");
    for blueprint in ["blank", "api"] {
        for database in DATABASES {
            let case = format!("{blueprint} with {database}");
            let root = starter(parent.path(), blueprint, database);
            let starter_users =
                fs::read_to_string(root.join(STARTER_USERS)).expect("starter users");
            assert!(
                fs::read_to_string(root.join("src/main.rs"))
                    .expect("starter main.rs")
                    .contains("pub struct User {"),
                "{case}"
            );

            let output = succeeds(&root, &["auth"]);
            assert!(output.contains("Kept the starter's"), "{case}: {output}");
            assert!(
                output.contains("user-<id>@no-email.invalid"),
                "{case}: {output}"
            );

            // The starter's migration stays as it was, so an applied one is
            // never edited, and no second users table is created.
            assert_eq!(
                fs::read_to_string(root.join(STARTER_USERS)).expect("starter users"),
                starter_users,
                "{case}"
            );
            assert_eq!(
                migrations_ending_with(&root, "_create_users_table.rs"),
                ["m20260601000000_create_users_table.rs"],
                "{case}"
            );
            let added = migrations_ending_with(&root, "_add_account_columns_to_users_table.rs");
            assert_eq!(added.len(), 1, "{case}");
            let resets = migrations_ending_with(&root, "_create_password_reset_tokens_table.rs");
            assert_eq!(resets.len(), 1, "{case}");
            let added_name = added[0].trim_end_matches(".rs");
            let migration = fs::read_to_string(root.join("src/migrations").join(&added[0]))
                .expect("account columns migration");
            syn::parse_file(&migration).expect("account columns migration parses");
            assert!(migration.contains(&format!("\"{added_name}\"")), "{case}");
            assert!(migration.contains("Schema::table(\"users\""), "{case}");

            // Applied in order: the starter's table, its account columns,
            // then the reset tokens.
            let registry =
                fs::read_to_string(root.join("src/migrations/mod.rs")).expect("registry");
            let position = |module: &str| {
                registry
                    .find(&format!("Box::new({module}::MigrationImpl)"))
                    .unwrap_or_else(|| panic!("{case}: {module} not registered"))
            };
            let resets_name = resets[0].trim_end_matches(".rs");
            assert!(position("m20260601000000_create_users_table") < position(added_name));
            assert!(position(added_name) < position(resets_name), "{case}");

            // One `User` for `users`: the account model, re-exported where the
            // starter declared its two-field model.
            let model = fs::read_to_string(root.join("src/models/user.rs")).expect("account model");
            for field in [
                "pub email: String,",
                "pub password_hash: Option<String>,",
                "pub session_epoch: i64,",
            ] {
                assert!(model.contains(field), "{case}: {field}");
            }
            let main = fs::read_to_string(root.join("src/main.rs")).expect("main.rs");
            assert!(!main.contains("pub struct User {"), "{case}");
            assert!(
                main.contains("\npub use crate::models::user::User;\n"),
                "{case}"
            );
            assert!(main.contains("pub mod models;"), "{case}");
            syn::parse_file(&main).expect("main.rs parses");

            // A second run has nothing to do.
            let before = snapshot(&root);
            let again = succeeds(&root, &["auth"]);
            assert!(
                again.contains("Authentication is already scaffolded"),
                "{case}: {again}"
            );
            assert_eq!(
                snapshot(&root),
                before,
                "{case}: a second run changed files"
            );
        }
    }
}

#[test]
fn auth_leaves_the_saas_starter_and_its_accounts_unchanged() {
    let parent = tempfile::tempdir().expect("temporary directory");
    for database in DATABASES {
        let root = starter(parent.path(), "saas", database);
        let before = snapshot(&root);
        let output = succeeds(&root, &["auth"]);
        assert!(
            output.contains("Authentication is already scaffolded"),
            "{database}: {output}"
        );
        assert!(
            output.contains("nothing was written"),
            "{database}: {output}"
        );
        assert!(!output.contains("db:migrate"), "{database}: {output}");
        assert_eq!(
            snapshot(&root),
            before,
            "{database}: auth changed the SaaS starter"
        );
    }
}

#[test]
fn auth_refuses_a_changed_starter_users_migration() {
    let parent = tempfile::tempdir().expect("temporary directory");
    let root = starter(parent.path(), "blank", "sqlite");
    let changed = fs::read_to_string(root.join(STARTER_USERS))
        .expect("starter users")
        .replace(
            "table.timestamps();",
            "table.string(\"email\").not_null();\n            table.timestamps();",
        );
    fs::write(root.join(STARTER_USERS), &changed).expect("changed users migration");
    let before = snapshot(&root);
    let output = rullst(&root, &["auth"]);
    let text = text(&output);
    assert!(!output.status.success(), "{text}");
    assert!(
        text.contains("refusing to change the users table"),
        "{text}"
    );
    assert!(text.contains("cargo rullst make:migration"), "{text}");
    assert_eq!(snapshot(&root), before, "a refused run changed files");
}
