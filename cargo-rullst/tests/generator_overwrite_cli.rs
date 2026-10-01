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
            "[package]\nname = \"overwrite-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n",
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

#[test]
fn kubernetes_manifests_are_never_replaced_or_written_through_links() {
    let project = Project::new();
    project.succeeds(&["make:k8s"]);
    let customized = "# customized registry and secrets\n";
    fs::write(project.path("k8s/deployment.yaml"), customized).expect("custom manifest");
    fs::remove_file(project.path("k8s/ingress.yaml")).expect("remove one manifest");
    let refused = project.fails(&["make:k8s"]);
    assert!(refused.contains("k8s/deployment.yaml"), "{refused}");
    assert_unchanged(&project, "k8s/deployment.yaml", customized);
    assert!(
        !project.path("k8s/ingress.yaml").exists(),
        "a refused run must not write any manifest"
    );

    #[cfg(unix)]
    {
        let linked = Project::new();
        let outside = linked.path("outside");
        fs::create_dir_all(&outside).expect("outside directory");
        std::os::unix::fs::symlink(&outside, linked.path("k8s")).expect("k8s link");
        assert!(linked.fails(&["make:k8s"]).contains("symlink"));
        assert_eq!(fs::read_dir(&outside).expect("outside").count(), 0);
    }
}

#[test]
fn packaging_generators_refuse_to_replace_customized_files() {
    let project = Project::new();
    let customized = "# customized\n";
    for (command, path) in [
        ("dockerize", "Dockerfile"),
        ("nixify", ".envrc"),
        ("generate:buildah", "build_buildah.sh"),
    ] {
        fs::write(project.path(path), customized).expect("custom packaging file");
        let refused = project.fails(&[command]);
        assert!(refused.contains("refusing to overwrite"), "{refused}");
        assert_unchanged(&project, path, customized);
    }
    assert!(
        !project.path("flake.nix").exists(),
        "nixify must not write flake.nix when .envrc exists"
    );

    #[cfg(unix)]
    {
        let linked = Project::new();
        let outside = linked.path("outside-ignore");
        std::os::unix::fs::symlink(&outside, linked.path(".dockerignore"))
            .expect("dangling .dockerignore link");
        assert!(linked.fails(&["dockerize"]).contains("symlink"));
        assert!(!linked.path("Dockerfile").exists());
        assert!(!outside.exists());
    }
}

#[test]
fn auth_scaffold_never_replaces_account_files_or_duplicates_the_users_table() {
    let customized = Project::new();
    fs::create_dir_all(customized.path("src/models")).expect("models directory");
    let model = "// customized User with NexusModel\n";
    fs::write(customized.path("src/models/user.rs"), model).expect("custom model");
    let manifest = customized.read("Cargo.toml");
    let refused = customized.fails(&["auth"]);
    assert!(refused.contains("src/models/user.rs"), "{refused}");
    assert_unchanged(&customized, "src/models/user.rs", model);
    assert_unchanged(&customized, "Cargo.toml", &manifest);
    assert!(
        !customized
            .path("src/controllers/auth_controller.rs")
            .exists()
    );
    assert!(customized.migrations_ending_with(".rs").is_empty());

    let starter = Project::new();
    fs::create_dir_all(starter.path("src/migrations")).expect("migrations directory");
    let users = "// starter users table\n";
    let starter_migration = "src/migrations/m20260601000000_create_users_table.rs";
    fs::write(starter.path(starter_migration), users).expect("starter migration");
    let refused = starter.fails(&["auth"]);
    assert!(
        refused.contains("second users table migration"),
        "{refused}"
    );
    assert_eq!(
        starter
            .migrations_ending_with("_create_users_table.rs")
            .len(),
        1
    );
    assert!(!starter.path("src/models/user.rs").exists());

    let repeated = Project::new();
    repeated.succeeds(&["auth"]);
    let controller = repeated.read("src/controllers/auth_controller.rs");
    repeated.fails(&["auth"]);
    assert_eq!(
        repeated
            .migrations_ending_with("_create_users_table.rs")
            .len(),
        1
    );
    assert_unchanged(&repeated, "src/controllers/auth_controller.rs", &controller);
}

#[test]
fn auth_scaffold_enables_auth_registers_modules_and_rejects_turso() {
    let project = Project::new();
    project.succeeds(&["auth"]);
    let manifest: toml::Value = toml::from_str(&project.read("Cargo.toml")).expect("manifest");
    let features = manifest["dependencies"]["rullst"]["features"]
        .as_array()
        .expect("rullst features");
    for feature in ["orm", "auth"] {
        assert!(features.iter().any(|value| value.as_str() == Some(feature)));
    }
    let root = project.read("src/main.rs");
    for module in ["controllers", "middlewares", "models", "pages"] {
        assert!(root.contains(&format!("pub mod {module};")), "{root}");
    }
    for (index, module) in [
        ("src/controllers/mod.rs", "auth_controller"),
        ("src/middlewares/mod.rs", "auth_middleware"),
        ("src/models/mod.rs", "user"),
        ("src/pages/mod.rs", "auth"),
    ] {
        assert!(project.read(index).contains(&format!("pub mod {module};")));
    }
    syn::parse_file(&root).expect("registered root module parses");

    let turso = Project::new();
    fs::write(
        turso.path("Cargo.toml"),
        "[package]\nname = \"turso-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = { version = \"13\", features = [\"orm\", \"orm-turso\"] }\n",
    )
    .expect("Turso manifest");
    let refused = turso.fails(&["auth"]);
    assert!(refused.contains("Turso-primary"), "{refused}");
    assert!(!turso.path("src/migrations").exists());
    assert!(!turso.path("src/models").exists());
}
