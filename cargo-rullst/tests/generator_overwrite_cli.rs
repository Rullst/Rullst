//! Generators keep existing application files and schema history intact.

#![allow(clippy::expect_used, clippy::panic)]

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

struct Project {
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("rullst-cli-overwrite-{}", rand::random::<u64>()));
        fs::create_dir_all(root.join("src")).expect("project source");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"overwrite-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"12\"\n",
        )
        .expect("project manifest");
        fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("project entry point");
        Self { root }
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rullst"))
            .current_dir(&self.root)
            .args(arguments)
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            .output()
            .unwrap_or_else(|error| panic!("run {arguments:?}: {error}"))
    }

    fn succeeds(&self, arguments: &[&str]) -> String {
        let output = self.run(arguments);
        let text = text(&output);
        assert!(output.status.success(), "{arguments:?} failed:\n{text}");
        text
    }

    fn fails(&self, arguments: &[&str]) -> String {
        let output = self.run(arguments);
        let text = text(&output);
        assert!(!output.status.success(), "{arguments:?} passed:\n{text}");
        text
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative))
            .unwrap_or_else(|error| panic!("{relative}: {error}"))
    }

    fn migrations_ending_with(&self, suffix: &str) -> Vec<PathBuf> {
        let directory = self.path("src/migrations");
        if !directory.exists() {
            return Vec::new();
        }
        fs::read_dir(directory)
            .expect("migration directory")
            .map(|entry| entry.expect("migration entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(suffix))
            })
            .collect()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_unchanged(project: &Project, relative: &str, expected: &str) {
    assert_eq!(project.read(relative), expected, "{relative} was replaced");
}

#[test]
fn repeated_model_and_resource_scaffolds_keep_one_create_migration() {
    let project = Project::new();
    project.succeeds(&["make:model", "Post", "--migration"]);
    assert_eq!(project.migrations_ending_with("_create_posts.rs").len(), 1);
    let model = project.read("src/models/post.rs");

    let resource = project.succeeds(&["make:resource", "Post"]);
    assert!(
        resource.contains("Skipping the create migration"),
        "{resource}"
    );
    project.succeeds(&["make:model", "Post", "-m"]);
    assert_eq!(project.migrations_ending_with("_create_posts.rs").len(), 1);
    assert_unchanged(&project, "src/models/post.rs", &model);

    // A starter-owned `_table` migration also counts as the table's creator.
    fs::write(
        project.path("src/migrations/m20260601000000_create_users_table.rs"),
        "// starter migration\n",
    )
    .expect("starter migration");
    project.succeeds(&["make:model", "User", "--migration"]);
    assert!(
        project
            .migrations_ending_with("_create_users.rs")
            .is_empty()
    );
    assert!(project.path("src/models/user.rs").is_file());
}

fn sqlite_database(project: &Project) -> String {
    let path = project.path("introspection.sqlite");
    let portable_path = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    let url = format!("sqlite:///{portable_path}?mode=rwc");
    #[cfg(not(windows))]
    let url = format!("sqlite://{portable_path}?mode=rwc");
    tokio::runtime::Runtime::new()
        .expect("SQLite fixture runtime")
        .block_on(async {
            let pool = sqlx::SqlitePool::connect(&url)
                .await
                .expect("SQLite fixture connection");
            sqlx::query("CREATE TABLE accounts (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
                .execute(&pool)
                .await
                .expect("SQLite fixture schema");
            pool.close().await;
        });
    url
}

#[test]
fn database_model_generation_keeps_existing_models_and_module_declarations() {
    let project = Project::new();
    let url = sqlite_database(&project);
    fs::create_dir_all(project.path("src/models")).expect("models directory");
    let index = "pub mod user;\npub mod subscription;";
    fs::write(project.path("src/models/mod.rs"), index).expect("module index");
    fs::write(project.path("src/models/accounts.rs"), "// customized\n").expect("model");
    let arguments = ["generate:models", "--driver", "sqlite", "--url", &url];

    let refused = project.fails(&arguments);
    assert!(refused.contains("refusing to overwrite"), "{refused}");
    assert!(refused.contains("accounts.rs"), "{refused}");
    assert_unchanged(&project, "src/models/accounts.rs", "// customized\n");
    assert_unchanged(&project, "src/models/mod.rs", index);

    fs::remove_file(project.path("src/models/accounts.rs")).expect("move model aside");
    project.succeeds(&["make:models-from-db", "--driver", "sqlite", "--url", &url]);
    assert_eq!(
        project.read("src/models/mod.rs"),
        "pub mod user;\npub mod subscription;\npub mod accounts;\n"
    );
    assert!(
        project
            .read("src/models/accounts.rs")
            .contains("pub id: i32")
    );
}
