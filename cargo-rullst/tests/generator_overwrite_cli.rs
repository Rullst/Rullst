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
