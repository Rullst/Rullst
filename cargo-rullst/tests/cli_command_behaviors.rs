//! Process-level coverage for inexpensive public CLI commands.
//!
//! The generated-project suites prove representative Cargo builds. This suite
//! instead exercises the remaining file-producing and inspection commands in
//! an isolated fixture, including repeat/conflict behavior, without invoking
//! provider accounts or a network deployment.

#![allow(clippy::expect_used, clippy::panic)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

struct Fixture {
    root: PathBuf,
    /// Keeps a symlinked parent directory alive (see `under_symlinked_parent`).
    _parent: Option<tempfile::TempDir>,
}

impl Fixture {
    fn new(label: &str) -> Self {
        Self::create_in(&std::env::temp_dir(), label, None)
    }

    /// A fixture reached through a symlinked parent, as macOS spells its
    /// temporary directory (`/var` -> `/private/var`): the CLI and its child
    /// processes observe the resolved path.
    #[cfg(unix)]
    fn under_symlinked_parent(label: &str) -> Self {
        let parent = tempfile::tempdir().expect("fixture parent");
        let real = parent.path().join("real");
        fs::create_dir(&real).expect("real fixture parent");
        let link = parent.path().join("link");
        std::os::unix::fs::symlink(&real, &link).expect("symlinked fixture parent");
        Self::create_in(&link, label, Some(parent))
    }

    fn create_in(base: &Path, label: &str, parent: Option<tempfile::TempDir>) -> Self {
        let root = base.join(format!(
            "rullst-cli-behavior-{label}-{}",
            rand::random::<u64>()
        ));
        // HOME lives beside, not inside, the project the CLI inspects.
        fs::create_dir_all(root.with_extension("home")).expect("isolated home directory");
        fs::create_dir_all(root.join("src/controllers")).expect("controllers directory");
        fs::create_dir_all(root.join("src/models")).expect("models directory");
        fs::create_dir_all(root.join("static")).expect("static directory");
        fs::create_dir_all(root.join(".git/hooks")).expect("Git hooks directory");
        fs::create_dir_all(root.join("empty-bin")).expect("empty executable directory");
        fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "cli-fixture"
version = "0.1.0"
edition = "2024"

[dependencies]
rullst = "12"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
"#,
        )
        .expect("fixture manifest");
        fs::write(
            root.join("src/main.rs"),
            r#"pub mod controllers;
pub mod models;

fn authorize() {
    RbacGuard::authorize_owner_or_role();
}

fn routes() {
    // rullst-access: owner — the authenticated owner may read this user.
    get("/users/:id" => controllers::users::show),
    post("/users" => controllers::users::store),
    // rullst-access: owner — the authenticated owner may update this user.
    put("/users/:id" => controllers::users::update),
    // rullst-access: owner — the authenticated owner may delete this user.
    delete("/users/:id" => controllers::users::delete),
    // rullst-access: owner — the authenticated owner may patch this user.
    patch("/users/:id" => controllers::users::patch),
    options("/users" => controllers::users::options),
    head("/users" => controllers::users::head),
}
"#,
        )
        .expect("fixture entry point");
        fs::write(root.join("src/controllers/mod.rs"), "pub mod users;\n")
            .expect("controllers module");
        fs::write(
            root.join("src/controllers/users.rs"),
            r#"/// Fetches one user without exposing internal state.
pub async fn show() {}
/// Creates one user.
pub async fn store() {}
pub async fn update() {}
pub async fn delete() {}
pub async fn patch() {}
pub async fn options() {}
pub async fn head() {}
"#,
        )
        .expect("controller fixture");
        fs::write(root.join("src/models/mod.rs"), "pub mod user;\n").expect("models module");
        fs::write(
            root.join("src/models/user.rs"),
            "pub struct User {\n    pub id: i64,\n    pub name: String,\n}\n",
        )
        .expect("model fixture");
        fs::write(
            root.join("rullst-schema.json"),
            r#"{"models":[{"name":"User"}]}"#,
        )
        .expect("schema fixture");
        fs::write(root.join("static/app.css"), "body { color: #fff; }\n").expect("static fixture");
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"cli-fixture\"\nversion = \"0.1.0\"\n",
        )
        .expect("fixture lockfile");
        Self {
            root,
            _parent: parent,
        }
    }

    /// The isolated HOME: commands never read the developer's or runner's
    /// `~/.ssh`, `~/.cargo` or caches, so recorded arguments are deterministic.
    fn home(&self) -> PathBuf {
        self.root.with_extension("home")
    }

    fn command(&self, arguments: &[&str]) -> Output {
        self.command_with_path(arguments, &self.root.join("empty-bin"))
    }

    fn command_with_path(&self, arguments: &[&str], path: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rullst"))
            .current_dir(&self.root)
            .args(arguments)
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            // Deployment helpers must exercise their documented offline/manual
            // fallback even if a developer happens to have provider CLIs.
            .env("PATH", path)
            .env("HOME", self.home())
            .env("USERPROFILE", self.home())
            .env_remove("SSH_AUTH_SOCK")
            .env_remove("SSH_AGENT_PID")
            .output()
            .unwrap_or_else(|error| panic!("run {arguments:?}: {error}"))
    }

    fn succeeds(&self, arguments: &[&str]) -> String {
        let output = self.command(arguments);
        let text = output_text(&output);
        assert!(
            output.status.success(),
            "command {arguments:?} failed:\n{text}"
        );
        text
    }

    #[cfg(unix)]
    fn succeeds_with_path(&self, arguments: &[&str], path: &Path) -> String {
        let output = self.command_with_path(arguments, path);
        let text = output_text(&output);
        assert!(
            output.status.success(),
            "command {arguments:?} failed:\n{text}"
        );
        text
    }

    #[cfg(unix)]
    fn install_successful_tool_fixtures(&self) -> PathBuf {
        let directory = self.root.join("fake-tools");
        fs::create_dir_all(&directory).expect("fake tool directory");
        for (name, body) in [
            (
                "cargo",
                // A `foundry:deploy` build reports its binary outside `target/`,
                // as with CARGO_TARGET_DIR or a configured build target.
                "#!/bin/sh\nfor argument in \"$@\"; do\n  if [ \"$argument\" = --message-format=json-render-diagnostics ]; then\n    printf '{\"reason\":\"compiler-artifact\",\"manifest_path\":\"%s/Cargo.toml\",\"target\":{\"kind\":[\"bin\"],\"name\":\"cli-fixture\"},\"profile\":{\"test\":false},\"executable\":\"%s/custom-target/release/cli-fixture\"}\\n' \"$PWD\" \"$PWD\"\n  fi\ndone\necho 'Launching Omni interface...'\nexit 0\n",
            ),
            ("rustup", "#!/bin/sh\nexit 0\n"),
            ("rustc", "#!/bin/sh\necho 'rustc 1.98.1 (fixture)'\n"),
            ("docker", "#!/bin/sh\necho 'Docker version fixture'\n"),
            ("git", "#!/bin/sh\necho 'git version fixture'\n"),
            (
                "npm",
                "#!/bin/sh\nif [ \"$1\" = \"install\" ]; then\n  /bin/mkdir -p node_modules/@tauri-apps/cli\n  printf '{}' > node_modules/@tauri-apps/cli/package.json\nfi\nif [ \"$5 $6\" = \"android init\" ]; then\n  /bin/mkdir -p gen/android/app\n  printf '// fixture' > gen/android/app/build.gradle.kts\nfi\nexit 0\n",
            ),
            ("ssh", "#!/bin/sh\n/bin/cat >/dev/null\nexit 0\n"),
            (
                "scp",
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"${0%/*}/scp-arguments\"\nexit 0\n",
            ),
            // An empty TCP listener inventory for `audit --network`.
            ("ss", "#!/bin/sh\nexit 0\n"),
        ] {
            let path = directory.join(name);
            fs::write(&path, body).expect("fake tool");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .expect("fake tool permissions");
        }
        directory
    }

    fn fails(&self, arguments: &[&str]) -> String {
        let output = self.command(arguments);
        let text = output_text(&output);
        assert!(
            !output.status.success(),
            "command {arguments:?} unexpectedly passed:\n{text}"
        );
        text
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
        let _ = fs::remove_dir_all(self.home());
    }
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_files(root: &Path, paths: &[&str]) {
    for path in paths {
        assert!(root.join(path).is_file(), "missing generated file {path}");
    }
}

#[test]
fn public_help_inventory_renders_every_command_group() {
    let groups = cargo_rullst::ui::get_help_groups();
    assert_eq!(groups.len(), 8);
    assert!(
        groups
            .iter()
            .flat_map(|(_, commands)| commands)
            .any(|(command, _)| *command == "cargo rullst upgrade")
    );
    cargo_rullst::ui::show_help_reference();

    assert!(cargo_rullst::ui::execute_command(Vec::new()).is_err());
    #[cfg(unix)]
    {
        assert!(
            cargo_rullst::ui::execute_command(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "exit 0".to_string(),
            ])
            .is_ok()
        );
        assert!(
            cargo_rullst::ui::execute_command(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "exit 9".to_string(),
            ])
            .is_err()
        );
    }
}

#[test]
fn public_generators_materialize_and_preserve_reviewable_outputs() {
    let fixture = Fixture::new("generators");

    fixture.succeeds(&["make:controller", "Account"]);
    fixture.succeeds(&["make:controller", "ApiAccount", "--api"]);
    fixture.succeeds(&["make:controller", "Account"]);
    fixture.succeeds(&["make:middleware", "RequestAudit"]);
    fixture.succeeds(&["make:middleware", "RequestAudit"]);
    fixture.succeeds(&["make:worker", "EmailDelivery"]);
    fixture.succeeds(&["make:island", "ProgressChart"]);
    fixture.succeeds(&["make:live", "LiveCounter"]);
    fixture.succeeds(&["make:grpc", "Course"]);

    // Names that cannot be Rust identifiers never reach the filesystem.
    let live_index = fs::read_to_string(fixture.root.join("src/live/mod.rs")).expect("live index");
    let controller_index =
        fs::read_to_string(fixture.root.join("src/controllers/mod.rs")).expect("controller index");
    let middleware_index =
        fs::read_to_string(fixture.root.join("src/middlewares/mod.rs")).expect("middleware index");
    for invalid in [
        &["make:live", "../../notes"][..],
        &["make:live", "self"],
        &["make:controller", "Bad.Name"],
        &["make:middleware", "../escape"],
    ] {
        assert!(
            fixture.fails(invalid).contains("Rust identifier"),
            "{invalid:?}"
        );
    }
    assert!(!fixture.root.join("notes.rs").exists());
    assert!(!fixture.root.join("src/live/self.rs").exists());
    for (index, before) in [
        ("src/live/mod.rs", &live_index),
        ("src/controllers/mod.rs", &controller_index),
        ("src/middlewares/mod.rs", &middleware_index),
    ] {
        assert_eq!(
            &fs::read_to_string(fixture.root.join(index)).expect("module index"),
            before,
            "{index} changed"
        );
    }

    assert_files(
        &fixture.root,
        &[
            "src/controllers/account_controller.rs",
            "src/controllers/api_account_controller.rs",
            "src/middlewares/request_audit_middleware.rs",
            "src/workers/email_delivery_worker.rs",
            "src/islands/progress_chart.rs",
            "src/live/live_counter.rs",
            "proto/course.proto",
            "src/grpc/course.rs",
        ],
    );
    assert!(
        fixture
            .fails(&["make:live", "LiveCounter"])
            .contains("already exists")
    );
    assert!(
        fixture
            .fails(&["make:grpc", "Course"])
            .contains("overwrite")
    );

    fixture.succeeds(&["generate:openapi"]);
    fixture.succeeds(&["generate:ts"]);
    fixture.succeeds(&["generate:diagram"]);
    fixture.succeeds(&["generate:ai-context"]);
    assert_files(
        &fixture.root,
        &[
            "openapi.json",
            "rullst-client.ts",
            "diagram.md",
            ".llms.txt",
        ],
    );

    let openapi = fs::read_to_string(fixture.root.join("openapi.json")).expect("OpenAPI output");
    assert!(openapi.contains("/users/{id}"));
    assert!(openapi.contains("Fetches one user"));
    let sdk = fs::read_to_string(fixture.root.join("rullst-client.ts")).expect("TypeScript output");
    assert!(sdk.contains("users_show"));
    assert!(sdk.contains("id: string | number"));
}

#[test]
fn diagram_generator_discovers_all_relation_shapes_and_missing_source() {
    let fixture = Fixture::new("diagram-relations");
    fs::write(
        fixture.root.join("src/models/relations.rs"),
        r#"#[derive(Orm)]
pub struct Course {
    pub id: i64,
    pub lessons: HasMany<Lesson>,
    pub owner: BelongsTo<User>,
    pub profile: HasOne<Profile>,
    pub tags: BelongsToMany<Tag>,
}
"#,
    )
    .expect("relation model fixture");
    fixture.succeeds(&["generate:diagram"]);
    let diagram = fs::read_to_string(fixture.root.join("diagram.md")).expect("Mermaid diagram");
    for expected in [
        "Course {",
        "i64 id",
        "Course ||--o{ Lesson : \"lessons\"",
        "Course }o--|| User : \"owner\"",
        "Course ||--o| Profile : \"profile\"",
        "Course }o--o{ Tag : \"tags\"",
    ] {
        assert!(
            diagram.contains(expected),
            "missing `{expected}` in:\n{diagram}"
        );
    }

    fs::remove_dir_all(fixture.root.join("src")).expect("remove source fixture");
    assert!(
        fixture
            .fails(&["generate:diagram"])
            .contains("No src/ directory found")
    );
}

#[test]
fn scaffolds_keep_a_hand_written_diagram_and_refresh_a_generated_one() {
    let fixture = Fixture::new("diagram-ownership");
    let diagram = fixture.root.join("diagram.md");
    let architecture = "# Architecture\n\nHand-written decisions.\n";
    fs::write(&diagram, architecture).expect("hand-written diagram");

    let output = fixture.succeeds(&["make:controller", "Account"]);
    assert!(output.contains("diagram.md was not refreshed"), "{output}");
    assert_eq!(
        fs::read_to_string(&diagram).expect("kept diagram"),
        architecture
    );
    assert!(
        fixture
            .fails(&["generate:diagram"])
            .contains("move it aside")
    );

    fs::remove_file(&diagram).expect("move the hand-written diagram aside");
    fixture.succeeds(&["make:middleware", "RequestAudit"]);
    let generated = fs::read_to_string(&diagram).expect("generated diagram");
    assert!(generated.starts_with("<!-- Generated by `cargo rullst generate:diagram`"));
    fixture.succeeds(&["generate:diagram"]);
}

#[test]
fn inspection_packaging_and_deploy_scaffolds_cover_safe_offline_paths() {
    let fixture = Fixture::new("operations");

    for target in [
        "routes",
        "models",
        "schema",
        "src/main.rs",
        "does-not-exist",
    ] {
        fixture.succeeds(&["inspect", target]);
    }
    // Without a project-provided snapshot, the schema comes from the models.
    fs::remove_file(fixture.root.join("rullst-schema.json")).expect("remove schema snapshot");
    fs::write(
        fixture.root.join("src/models/person.rs"),
        "#[derive(Orm)]\n#[orm(soft_delete, table_name = \"people\")]\npub struct Person {\n    pub id: i64,\n    pub nickname: Option<String>,\n}\n",
    )
    .expect("ORM model fixture");
    let schema = fixture.succeeds(&["inspect", "schema"]);
    assert!(schema.contains("\"table\": \"people\""), "{schema}");
    assert!(schema.contains("\"optional\": true"), "{schema}");
    assert!(!schema.contains("not found"), "{schema}");

    fixture.succeeds(&["dockerize"]);
    fixture.succeeds(&["generate:buildah"]);
    fixture.succeeds(&["nixify"]);
    fixture.succeeds(&["make:k8s"]);
    fixture.succeeds(&["make:scalar"]);
    fixture.succeeds(&["foundry:init"]);
    fixture.succeeds(&["hook:install"]);
    fixture.succeeds(&["eject"]);

    assert_files(
        &fixture.root,
        &[
            "Dockerfile",
            "build_buildah.sh",
            "flake.nix",
            ".envrc",
            "k8s/deployment.yaml",
            "src/controllers/docs_controller.rs",
            "Foundry.toml",
            ".git/hooks/pre-commit",
            "src/ejected_main.rs",
        ],
    );
    assert!(fixture.fails(&["eject"]).contains("overwrite"));

    for platform in ["render", "vps", "fly", "railway"] {
        fixture.succeeds(&["deploy", "--platform", platform]);
    }
    assert!(
        fixture
            .fails(&["deploy", "--platform", "unsupported"])
            .contains("Unknown platform")
    );
    assert_files(
        &fixture.root,
        &[
            "render.yaml",
            "docker-compose.prod.yml",
            "Caddyfile",
            "fly.toml",
            "railway.json",
        ],
    );
}

#[test]
fn unknown_commands_completions_info_and_json_views_are_scriptable() {
    let fixture = Fixture::new("cli-ux");

    let unknown = fixture.command(&["make:modek", "Post"]);
    assert_eq!(
        unknown.status.code(),
        Some(2),
        "unknown commands are usage errors"
    );
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert!(
        stderr.contains("error: Unknown command `make:modek`"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Did you mean `cargo rullst make:model`?"),
        "{stderr}"
    );
    assert!(
        !stderr.contains('\u{1b}'),
        "NO_COLOR keeps the report plain"
    );
    assert!(unknown.stdout.is_empty());

    let completions = fixture.succeeds(&["completions", "bash"]);
    assert!(completions.contains("_rullst()") && completions.contains("make:model"));

    let info = fixture.command(&["info", "--json"]);
    assert!(info.status.success());
    let info: serde_json::Value = serde_json::from_slice(&info.stdout).expect("info JSON");
    assert_eq!(info["schema_version"], "rullst.cli-info.v1");
    assert_eq!(info["project"]["name"], "cli-fixture");
    assert_eq!(info["project"]["rullst"], "12");

    let routes = fixture.command(&["inspect", "routes", "--json"]);
    assert!(routes.status.success());
    let routes: serde_json::Value = serde_json::from_slice(&routes.stdout).expect("routes JSON");
    assert_eq!(routes["schema_version"], "rullst.cli-routes.v1");
    assert_eq!(routes["routes"][0]["path"], "/users/:id");
    assert_eq!(routes["routes"][0]["handler"], "controllers::users::show");

    // `audit --json` keeps stdout to one document; progress goes to stderr.
    let audit = fixture.command(&["audit", "--json"]);
    let summary: serde_json::Value =
        serde_json::from_slice(&audit.stdout).expect("audit --json prints one JSON document");
    assert_eq!(summary["schema_version"], "rullst.cli-audit.v1");
    assert_eq!(summary["checks"][1]["id"], "dependency_audit");
    assert_eq!(summary["checks"][1]["status"], "not_checked");
    assert!(String::from_utf8_lossy(&audit.stderr).contains("Audit finished"));

    // Generators end with concrete next steps; the friendly report covers
    // project commands run outside a project.
    let model = fixture.succeeds(&["make:model", "Invoice", "-m"]);
    assert!(model.contains("Next steps") && model.contains("cargo rullst db:migrate"));
    let outside = tempfile::tempdir().expect("outside directory");
    let output = Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(outside.path())
        .args(["make:controller", "Users"])
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
        .env("NO_COLOR", "1")
        .output()
        .expect("run outside a project");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("error: Not inside a Rullst project\n"),
        "{stderr}"
    );
    assert!(stderr.contains("cargo rullst new <name>"));
    assert!(stderr.contains("https://rullst.github.io/Rullst/book/start-here.html"));
}

#[cfg(unix)]
#[test]
fn diagnostics_audit_and_build_are_exercised_with_controlled_tool_processes() {
    // The symlinked parent reproduces the macOS runner's temporary directory.
    let fixture = Fixture::under_symlinked_parent("diagnostics");

    // A missing compiler is a failed check: the doctor exits with status 1.
    let missing = fixture.command(&["doctor"]);
    assert_eq!(missing.status.code(), Some(1));
    let missing = output_text(&missing);
    assert!(missing.contains("✗ Rust compiler"), "{missing}");
    assert!(missing.contains("rustc not found"));
    assert!(missing.contains("cli_reference.html#doctor-toolchain"));
    let failed_fix = output_text(&fixture.command(&["doctor", "--fix"]));
    assert!(
        failed_fix.contains("--fix could not install them"),
        "{failed_fix}"
    );

    let tools = fixture.install_successful_tool_fixtures();
    let healthy = fixture.succeeds_with_path(&["doctor"], &tools);
    assert!(healthy.contains("✓ Rust compiler"), "{healthy}");
    assert!(healthy.contains("0 failed"), "{healthy}");
    // The fixture depends on rullst 12, older than this CLI.
    assert!(healthy.contains("! Rullst version"), "{healthy}");
    let json = fixture.command_with_path(&["doctor", "--json"], &tools);
    assert!(json.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("doctor --json prints one JSON document");
    assert_eq!(report["schema_version"], "rullst.cli-doctor.v1");
    assert_eq!(report["ok"], true);

    fixture.succeeds_with_path(
        &[
            "audit",
            "--ai",
            "--compliance",
            "--idor",
            "--geiger",
            "--sbom",
            "--network",
        ],
        &tools,
    );
    let governed = fixture.succeeds_with_path(
        &[
            "audit",
            "--audit-ignore",
            "RUSTSEC-2099-0001",
            "--audit-ignore",
            "RUSTSEC-2099-0002",
        ],
        &tools,
    );
    assert!(
        governed.contains("exceptions remain unresolved: RUSTSEC-2099-0001, RUSTSEC-2099-0002")
    );
    assert!(
        fixture
            .fails(&["audit", "--audit-ignore", "RUSTSEC-2099-0001 --quiet"])
            .contains("expected RUSTSEC-YYYY-NNNN")
    );
    fixture.succeeds_with_path(&["build", "--debug"], &tools);
    fixture.succeeds_with_path(&["build"], &tools);
    fs::create_dir_all(fixture.root.join("src/migrations")).expect("migration directory");
    let dev = fixture.command_with_path(&["dev"], &tools);
    let dev_output = output_text(&dev);
    assert!(
        !dev.status.success(),
        "a mock Cargo process without compiler-artifact metadata must not launch dev:\n{dev_output}"
    );
    assert!(
        !fixture
            .command_with_path(&["dash"], &tools)
            .status
            .success()
    );

    fixture.succeeds_with_path(&["foundry:init"], &tools);
    let foundry_path = fixture.root.join("Foundry.toml");
    let foundry = fs::read_to_string(&foundry_path)
        .expect("Foundry manifest")
        .replace(
            "APP_KEY = \"CHANGE_ME_TO_A_SECURE_RANDOM_KEY\"",
            "APP_KEY = \"fixture-secret-with-adequate-length\"",
        );
    fs::write(foundry_path, foundry).expect("configured Foundry manifest");
    // The fake Cargo builds nothing and reports custom-target/release; deploy
    // hashes and uploads that binary, never a stale one under target/.
    for (directory, contents) in [
        ("custom-target/release", &b"fixture binary"[..]),
        ("target/release", &b"stale binary"[..]),
    ] {
        let release = fixture.root.join(directory);
        fs::create_dir_all(&release).expect("release directory");
        fs::write(release.join("cli-fixture"), contents).expect("release binary fixture");
    }
    fixture.succeeds_with_path(&["foundry:deploy"], &tools);
    let uploaded = fs::read_to_string(tools.join("scp-arguments")).expect("scp arguments");
    // Cargo (here `$PWD`) reports the resolved path, not the symlinked spelling.
    let reported = format!(
        "{}/custom-target/release/cli-fixture",
        fixture
            .root
            .canonicalize()
            .expect("resolved fixture root")
            .display()
    );
    assert!(uploaded.lines().any(|line| line == reported), "{uploaded}");
    // The configured `~/.ssh/id_rsa` expands under the isolated HOME only.
    let key = format!("{}/.ssh/id_rsa", fixture.home().display());
    assert!(uploaded.lines().any(|line| line == key), "{uploaded}");

    assert_files(
        &fixture.root,
        &[
            "SECURITY_COMPLIANCE.md",
            "sbom-cyclonedx.json",
            "static/app.css.br",
            "static/app.css.zst",
        ],
    );
}

#[cfg(unix)]
#[test]
fn audit_reports_missing_project_sources_as_not_checked() {
    let fixture = Fixture::new("audit-missing-source");
    fs::remove_dir_all(fixture.root.join("src")).expect("remove fixture source tree");
    let tools = fixture.install_successful_tool_fixtures();

    let output = fixture.succeeds_with_path(&["audit", "--compliance"], &tools);
    assert!(output.contains("bounded unsafe scan was not executed"));

    let report = fs::read_to_string(fixture.root.join("SECURITY_COMPLIANCE.md"))
        .expect("security evidence report");
    assert!(report.contains(
        "| Project-source unsafe scan | **NOT CHECKED** | no src directory was available to inspect |"
    ));
    assert!(report.contains(
        "| Parameterized-route IDOR heuristic | **NOT CHECKED** | no scannable Rust source was available |"
    ));
}

#[cfg(unix)]
#[test]
fn audit_network_check_fails_when_the_listener_inventory_cannot_run() {
    let fixture = Fixture::new("audit-network-inventory");
    let tools = fixture.install_successful_tool_fixtures();

    fixture.succeeds_with_path(&["audit", "--network", "--compliance"], &tools);
    let report = fs::read_to_string(fixture.root.join("SECURITY_COMPLIANCE.md"))
        .expect("security evidence report");
    assert!(report.contains("| Local network-surface scan | **OBSERVED** |"));

    fs::remove_file(tools.join("ss")).expect("remove the listener inventory tool");
    let result = fixture.command_with_path(&["audit", "--network", "--compliance"], &tools);
    let output = output_text(&result);
    assert!(
        !result.status.success(),
        "an unavailable listener inventory must fail the requested check:\n{output}"
    );
    assert!(output.contains("Network check incomplete"), "{output}");
    assert!(!output.contains("No open local listening ports detected"));
    let report = fs::read_to_string(fixture.root.join("SECURITY_COMPLIANCE.md"))
        .expect("updated security evidence report");
    assert!(report.contains("| Local network-surface scan | **ERROR** |"));
    assert!(report.contains("ss -ltnH"));
}

#[cfg(unix)]
#[test]
fn audit_fails_closed_for_source_secret_tool_and_evidence_findings() {
    let fixture = Fixture::new("audit-findings");
    fs::write(
        fixture.root.join(".env"),
        "# ignored\n\nEMPTY_SECRET=\"\"\nSAFE_KEY=fixture-secret-long-enough\nAPI_KEY=short\n",
    )
    .expect("audit environment fixture");
    fs::write(
        fixture.root.join("src/unsafe_route.rs"),
        r#"pub unsafe fn unchecked() {}

pub fn routes() {
    get("/accounts/:id", show);
}
"#,
    )
    .expect("unsafe and unclassified route fixture");

    let output = fixture.fails(&[
        "audit",
        "--ai",
        "--compliance",
        "--idor",
        "--geiger",
        "--sbom",
    ]);
    for expected in [
        "Weak or short secret detected for key 'API_KEY'",
        "cargo-audit not installed",
        "Unsafe Rust detected",
        "cargo-geiger is unavailable",
        "missing an adjacent",
        "IDOR/BOLA audit found 1 unclassified or unguarded parameterized route",
    ] {
        assert!(
            output.contains(expected),
            "missing `{expected}` in:\n{output}"
        );
    }
    let report =
        fs::read_to_string(fixture.root.join("SECURITY_COMPLIANCE.md")).expect("audit report");
    assert!(report.contains("| Local secret-strength heuristic | **FINDINGS** | 1 finding(s)"));
    assert!(report.contains("| Project-source unsafe scan | **FINDINGS** | 1 finding(s)"));
    assert!(report.contains("| Parameterized-route IDOR heuristic | **FINDINGS** | 1 finding(s)"));

    fs::remove_file(fixture.root.join(".env")).expect("remove environment fixture");
    fs::create_dir(fixture.root.join(".env")).expect("unreadable environment fixture");
    let output = fixture.fails(&["audit", "--compliance"]);
    assert!(output.contains("Could not read .env for the bounded secret scan"));
    let report =
        fs::read_to_string(fixture.root.join("SECURITY_COMPLIANCE.md")).expect("updated report");
    assert!(report.contains("| Local secret-strength heuristic | **ERROR** |"));
}

#[cfg(unix)]
#[test]
fn audit_surfaces_nonzero_dependency_and_geiger_tool_results() {
    let fixture = Fixture::new("audit-tool-failures");
    let tools = fixture.install_successful_tool_fixtures();
    let cargo = tools.join("cargo");
    fs::write(
        &cargo,
        r#"#!/bin/sh
if [ "$1" = "audit" ] && [ "$2" = "--version" ]; then
  exit 0
fi
if [ "$1" = "audit" ]; then
  echo 'fixture advisory result' >&2
  exit 9
fi
if [ "$1" = "geiger" ] && [ "$2" = "--version" ]; then
  exit 0
fi
if [ "$1" = "geiger" ]; then
  exit 8
fi
exit 0
"#,
    )
    .expect("failing cargo tool fixture");
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755))
        .expect("cargo tool fixture permissions");

    let result = fixture.command_with_path(&["audit", "--compliance", "--geiger"], &tools);
    let output = output_text(&result);
    assert!(
        !result.status.success(),
        "failing tools must fail the audit"
    );
    assert!(output.contains("cargo-audit did not complete successfully"));
    assert!(output.contains("cargo-geiger reported findings or failed with status"));
    let report =
        fs::read_to_string(fixture.root.join("SECURITY_COMPLIANCE.md")).expect("audit report");
    assert!(report.contains("fixture advisory result"));

    fs::remove_file(fixture.root.join("Cargo.lock")).expect("remove lockfile fixture");
    let result = fixture.command_with_path(&["audit", "--sbom"], &tools);
    let output = output_text(&result);
    assert!(
        !result.status.success(),
        "missing lockfile must fail SBOM audit"
    );
    assert!(output.contains("Failed to generate SBOM"));
}

#[cfg(unix)]
#[test]
fn doctor_reports_outdated_unrecognized_and_failing_toolchains() {
    let fixture = Fixture::new("doctor-failures");
    let tools = fixture.install_successful_tool_fixtures();
    let cargo = tools.join("cargo");
    fs::write(
        &cargo,
        "#!/bin/sh\ncase \"$1\" in fmt|clippy|audit|geiger|deny|mutants|kani|llvm-cov) exit 7;; esac\nexit 0\n",
    )
    .expect("unavailable Cargo tools fixture");
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755))
        .expect("Cargo fixture permissions");
    for program in ["docker", "git"] {
        let path = tools.join(program);
        fs::write(&path, "#!/bin/sh\nexit 8\n").expect("failing tool fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("tool fixture permissions");
    }

    let rustc = tools.join("rustc");
    fs::write(&rustc, "#!/bin/sh\necho 'rustc 1.80.0 (fixture)'\n")
        .expect("outdated rustc fixture");
    fs::set_permissions(&rustc, fs::Permissions::from_mode(0o755))
        .expect("rustc fixture permissions");
    let outdated = fixture.command_with_path(&["doctor"], &tools);
    assert_eq!(
        outdated.status.code(),
        Some(1),
        "an outdated compiler fails"
    );
    let outdated = output_text(&outdated);
    assert!(
        outdated.contains("is older than the MSRV 1.96.0"),
        "{outdated}"
    );
    assert!(outdated.contains("cargo install cargo-audit"));
    assert!(outdated.contains("https://docs.docker.com/get-docker/"));

    fs::write(&rustc, "#!/bin/sh\necho 'unexpected fixture version'\n")
        .expect("unrecognized rustc fixture");
    let unrecognized = fixture.succeeds_with_path(&["doctor"], &tools);
    assert!(unrecognized.contains("! Rust compiler"), "{unrecognized}");
    assert!(unrecognized.contains("unrecognized version output"));

    fs::write(&rustc, "#!/bin/sh\nexit 9\n").expect("failing rustc fixture");
    let failing = fixture.command_with_path(&["doctor"], &tools);
    assert_eq!(failing.status.code(), Some(1));
    let failing = output_text(&failing);
    assert!(failing.contains("✗ Rust compiler"), "{failing}");
    assert!(failing.contains("rustc --version failed"));

    fs::write(&rustc, "#!/bin/sh\necho 'rustc 1.98.1 (fixture)'\n").expect("healthy rustc fixture");
    let repaired = fixture.root.join("components-repaired");
    fs::write(
        &cargo,
        format!(
            "#!/bin/sh\nif [ \"$1\" = fmt ] || [ \"$1\" = clippy ]; then [ -f '{}' ]; else exit 0; fi\n",
            repaired.display()
        ),
    )
    .expect("repair-aware Cargo fixture");
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755))
        .expect("repair-aware Cargo fixture permissions");
    let rustup = tools.join("rustup");
    fs::write(
        &rustup,
        format!("#!/bin/sh\n: > '{}'\n", repaired.display()),
    )
    .expect("repairing rustup fixture");
    fs::set_permissions(&rustup, fs::Permissions::from_mode(0o755))
        .expect("rustup fixture permissions");
    let fixed = fixture.succeeds_with_path(&["doctor", "--fix"], &tools);
    assert!(
        fixed.contains("installed by --fix"),
        "doctor did not report the repaired components:\n{fixed}"
    );
}

#[cfg(unix)]
#[test]
fn omni_scaffold_and_desktop_runner_use_pinned_local_tooling() {
    let fixture = Fixture::new("omni");
    let tools = fixture.install_successful_tool_fixtures();

    fixture.succeeds_with_path(
        &[
            "make:omni",
            "--platform",
            "desktop,android",
            "--backend-url",
            "https://api.example.com",
            "--product-name",
            "Rullst Fixture",
            "--identifier",
            "com.rullst.fixture",
            "--app-version",
            "1.2.3",
        ],
        &tools,
    );
    assert_files(
        &fixture.root,
        &[
            "omni-app/Cargo.toml",
            "omni-app/package.json",
            "omni-app/tauri.conf.json",
            "omni-app/src/lib.rs",
            "omni-app/icons/icon.svg",
            "omni-app/node_modules/@tauri-apps/cli/package.json",
        ],
    );

    // The runner must report the launch and keep the shell's and backend's
    // output instead of consuming it while it waits for the marker.
    fs::write(
        tools.join("cargo"),
        "#!/bin/sh\necho 'backend: serving on 3000'\necho 'Launching Omni interface...'\necho 'backend: GET / 200'\nexit 0\n",
    )
    .expect("Omni desktop cargo fixture");
    let desktop = fixture.succeeds_with_path(&["omni", "desktop"], &tools);
    assert!(
        desktop.contains("Omni window launched successfully"),
        "{desktop}"
    );
    assert!(desktop.contains("backend: serving on 3000"), "{desktop}");
    assert!(desktop.contains("backend: GET / 200"), "{desktop}");
    assert!(
        fixture
            .command_with_path(&["omni", "unsupported"], &tools)
            .status
            .code()
            .is_some_and(|code| code != 0)
    );
}
