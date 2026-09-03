//! Behavioral coverage for composable application scaffolds.

#![allow(clippy::expect_used, clippy::panic)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

struct Project {
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("rullst-cli-scaffolds-{}", rand::random::<u64>()));
        fs::create_dir_all(root.join("src")).expect("project source");
        fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "scaffold-fixture"
version = "0.1.0"
edition = "2024"

[dependencies]
rullst = "12"
serde = { version = "1", features = ["derive"] }
"#,
        )
        .expect("project manifest");
        fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("project entry point");
        Self { root }
    }

    fn run(&self, arguments: &[&str]) -> Output {
        self.run_with_path(arguments, None)
    }

    fn run_with_path(&self, arguments: &[&str], path: Option<&Path>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rullst"));
        command
            .current_dir(&self.root)
            .args(arguments)
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1");
        if let Some(path) = path {
            command.env("PATH", path);
        }
        command
            .output()
            .unwrap_or_else(|error| panic!("run {arguments:?}: {error}"))
    }

    fn succeeds(&self, arguments: &[&str]) {
        let output = self.run(arguments);
        assert!(
            output.status.success(),
            "command {arguments:?} failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn assert_files(&self, paths: &[&str]) {
        for path in paths {
            assert!(self.root.join(path).is_file(), "missing {path}");
        }
    }

    #[cfg(unix)]
    fn install_wasm_tools(&self) -> PathBuf {
        let directory = self.root.join("wasm-tools");
        fs::create_dir_all(&directory).expect("Wasm tool directory");
        for (name, body) in [
            ("rustup", "#!/bin/sh\nexit 0\n"),
            (
                "cargo",
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> .cargo-invocations\nif [ \"$1\" = \"build\" ]; then\n  /bin/mkdir -p target/wasm32-unknown-unknown/debug\n  printf '\\000asm' > target/wasm32-unknown-unknown/debug/scaffold_fixture.wasm\nfi\nexit 0\n",
            ),
            (
                "wasm-bindgen",
                "#!/bin/sh\nif [ \"$1\" != \"--version\" ]; then\n  printf 'export default async function init() {}' > static/scaffold_fixture.js\nfi\nexit 0\n",
            ),
        ] {
            let path = directory.join(name);
            fs::write(&path, body).expect("Wasm tool fixture");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .expect("Wasm tool permissions");
        }
        directory
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn auth_resource_and_service_scaffolds_compose_in_one_project() {
    let project = Project::new();

    project.succeeds(&["make:chat-session"]);
    assert!(!project.run(&["make:chat-session"]).status.success());
    project.succeeds(&["make:mail", "ReleaseNotice"]);
    project.succeeds(&["make:mail", "WelcomeLearner", "--welcome"]);
    project.succeeds(&["make:mail", "PasswordRecovery", "--reset"]);
    project.succeeds(&["make:mail", "LoginCode", "--otp"]);
    project.succeeds(&["make:mail", "BillingReceipt", "--invoice"]);
    project.succeeds(&["make:mail-invoice", "FiscalReceipt"]);
    project.succeeds(&["make:mail-dunning", "PaymentRecovery"]);
    assert!(
        !project
            .run(&["make:mail", "InvalidFlags", "--welcome", "--reset"])
            .status
            .success()
    );
    assert!(
        !project
            .run(&["make:mail", "ReleaseNotice"])
            .status
            .success()
    );
    assert!(!project.run(&["make:mail", "type"]).status.success());
    project.succeeds(&["auth"]);
    project.succeeds(&["make:mfa"]);
    project.succeeds(&["make:billing", "--model", "Team"]);
    project.succeeds(&["make:resource", "Product"]);
    project.succeeds(&["make:resource", "ApiEvent", "--api"]);
    project.succeeds(&["make:model", "AuditLog", "--migration"]);
    project.succeeds(&["make:model", "ReadModel"]);
    project.succeeds(&["make:migration", "add_lookup_to_products"]);
    project.succeeds(&["make:cors"]);
    project.succeeds(&["make:jwt"]);
    project.succeeds(&["make:cors"]);
    project.succeeds(&["make:jwt"]);
    project.succeeds(&["make:iot", "TemperatureSensor"]);

    project.assert_files(&[
        "src/controllers/auth_controller.rs",
        "src/controllers/mfa.rs",
        "src/models/chat.rs",
        "src/ai/chat_service.rs",
        "src/mail/release_notice.rs",
        "src/mail/welcome_learner.rs",
        "src/mail/password_recovery.rs",
        "src/mail/login_code.rs",
        "src/mail/billing_receipt.rs",
        "src/mail/fiscal_receipt.rs",
        "src/mail/payment_recovery.rs",
        "src/models/product.rs",
        "src/controllers/product_controller.rs",
        "views/product/index.html",
        "src/models/audit_log.rs",
        "src/middlewares/cors_middleware.rs",
        "src/middlewares/jwt_middleware.rs",
        "src/iot/temperature_sensor.rs",
    ]);

    let migrations =
        fs::read_to_string(project.root.join("src/migrations/mod.rs")).expect("migration registry");
    assert!(migrations.contains("get_migrations"));
    assert!(migrations.contains("products"));
}

#[test]
fn academy_evidence_and_forced_ejection_cover_success_and_failure_boundaries() {
    let project = Project::new();

    let absent = project.run(&["academy:doctor", "--evidence", "missing-evidence.json"]);
    assert!(!absent.status.success());
    fs::write(project.root.join("invalid-evidence.json"), "not-json")
        .expect("invalid evidence fixture");
    let invalid = project.run(&["academy:doctor", "--evidence", "invalid-evidence.json"]);
    assert!(!invalid.status.success());
    assert!(!project.run(&["academy:doctor", "--json"]).status.success());

    let requirements = [
        "authenticated_identity",
        "school_membership",
        "active_entitlement",
        "object_authorization",
        "tenant_isolation",
        "server_validated_assessment",
        "idempotent_score_events",
        "durable_automation",
        "durable_admin_audit",
        "safe_content_pipeline",
        "privacy_lifecycle",
        "distributed_abuse_control",
    ];
    let checks = requirements
        .into_iter()
        .map(|requirement| {
            serde_json::json!({
                "requirement": requirement,
                "status": "PASS",
                "evidence": [format!("test:{requirement}")]
            })
        })
        .collect::<Vec<_>>();
    let evidence = serde_json::json!({
        "schema_version": "rullst.academy-evidence.v1",
        "checks": checks
    });
    fs::write(
        project.root.join("academy-evidence.json"),
        serde_json::to_vec_pretty(&evidence).expect("serialize evidence"),
    )
    .expect("Academy evidence fixture");
    project.succeeds(&[
        "academy:doctor",
        "--evidence",
        "academy-evidence.json",
        "--json",
    ]);
    project.succeeds(&["academy:doctor", "--evidence", "academy-evidence.json"]);

    project.succeeds(&["eject", "--force"]);
    assert!(project.root.join("src/main.rs.rullst-backup").is_file());
    assert!(
        fs::read_to_string(project.root.join("src/main.rs"))
            .expect("ejected entry point")
            .contains("EJECTED AXUM SERVER")
    );
    assert!(!project.run(&["eject", "--force"]).status.success());
}

#[test]
fn package_and_database_introspection_commands_report_real_local_state() {
    let project = Project::new();
    project.succeeds(&["pkg", "add", "rullst-example"]);
    project.succeeds(&["pkg", "add", "rullst-example"]);
    project.succeeds(&["pkg", "list"]);
    assert!(!project.run(&["pkg", "add", "serde"]).status.success());

    let database = project.root.join("introspection.sqlite");
    let database_url = format!("sqlite://{}?mode=rwc", database.display());
    tokio::runtime::Runtime::new()
        .expect("SQLite fixture runtime")
        .block_on(async {
            let pool = sqlx::SqlitePool::connect(&database_url)
                .await
                .expect("SQLite fixture connection");
            sqlx::query(
                "CREATE TABLE accounts (\
                    id INTEGER PRIMARY KEY, \
                    total BIGINT NOT NULL, \
                    priority SMALLINT NOT NULL, \
                    enabled BOOLEAN NOT NULL, \
                    ratio REAL, \
                    precise DOUBLE, \
                    name TEXT NOT NULL, \
                    payload BLOB, \
                    created_at DATETIME NOT NULL, \
                    metadata JSON\
                )",
            )
            .execute(&pool)
            .await
            .expect("SQLite fixture schema");
            pool.close().await;
        });
    fs::create_dir_all(project.root.join("src/models")).expect("models directory");
    fs::write(
        project.root.join("src/models/schema_contract.rs"),
        r#"#[derive(Orm)]
#[orm(table = "accounts")]
pub struct Account {
    pub id: i32,
    pub name: String,
    pub newly_added: Option<String>,
    #[orm(skip)]
    pub transient: Vec<u8>,
    pub composite: (String, String),
}

#[derive(Orm)]
pub struct Course {
    pub id: i64,
    pub title: String,
    pub created_at: i64,
}

#[derive(Orm)]
pub struct Marker;
"#,
    )
    .expect("AST schema fixture");
    project.succeeds(&[
        "generate:models",
        "--driver",
        "sqlite",
        "--url",
        &database_url,
        "--output",
        "src/introspected",
    ]);
    fs::write(
        project.root.join(".env"),
        format!("DATABASE_URL={database_url}\n"),
    )
    .expect("database environment");
    project.succeeds(&["make:migration:auto"]);

    let manifest = fs::read_to_string(project.root.join("Cargo.toml")).expect("manifest");
    assert!(manifest.contains("rullst-example"));
    assert!(Path::new(&database).is_file());
    let model = fs::read_to_string(project.root.join("src/introspected/accounts.rs"))
        .expect("introspected model");
    assert!(model.contains("pub id: i32"));
    assert!(model.contains("pub total: i64"));
    assert!(model.contains("pub payload: Option<Vec<u8>>"));
    let migrations = fs::read_to_string(project.root.join("src/migrations/mod.rs"))
        .expect("auto-migration registry");
    assert!(migrations.contains("auto_sync"));
    let auto_migration = fs::read_dir(project.root.join("src/migrations"))
        .expect("migration directory")
        .filter_map(Result::ok)
        .find(|entry| entry.file_name().to_string_lossy().contains("auto_sync"))
        .expect("generated auto migration");
    let auto_migration = fs::read_to_string(auto_migration.path()).expect("auto migration source");
    assert!(auto_migration.contains("Schema::create(\"courses\""));
    assert!(auto_migration.contains("ALTER TABLE accounts ADD COLUMN newly_added"));
    assert!(auto_migration.contains("Destructive operation detected"));
}

#[test]
fn automatic_migration_reports_missing_and_unsupported_configuration() {
    let missing = Project::new();
    missing.succeeds(&["make:migration:auto"]);
    assert!(!missing.root.join("src/migrations").exists());

    let unsupported = Project::new();
    fs::write(
        unsupported.root.join(".env"),
        "DATABASE_URL=postgres://localhost/example\n",
    )
    .expect("unsupported database environment");
    unsupported.succeeds(&["make:migration:auto"]);
    assert!(!unsupported.root.join("src/migrations").exists());
}

#[cfg(unix)]
#[test]
fn database_forwarding_and_complete_project_packaging_use_controlled_cargo() {
    let project = Project::new();
    let tools = project.install_wasm_tools();

    for operation in ["db:migrate", "db:rollback", "db:status", "db:seed"] {
        let output = project.run_with_path(&[operation], Some(&tools));
        assert!(output.status.success(), "{operation} must be forwarded");
    }
    let invocations = fs::read_to_string(project.root.join(".cargo-invocations"))
        .expect("recorded database invocations");
    for operation in ["db:migrate", "db:rollback", "db:status", "db:seed"] {
        assert!(invocations.contains(&format!("run -- {operation}")));
    }

    let generated = project.run_with_path(
        &[
            "new",
            "packaged-app",
            "--default",
            "--docker",
            "--buildah",
            "--nix",
        ],
        Some(&tools),
    );
    assert!(generated.status.success());
    for path in [
        "packaged-app/Cargo.toml",
        "packaged-app/Dockerfile",
        "packaged-app/build_buildah.sh",
        "packaged-app/flake.nix",
        "packaged-app/.cargo-invocations",
    ] {
        assert!(project.root.join(path).is_file(), "missing {path}");
    }
    assert!(
        !project
            .run_with_path(&["new", "packaged-app", "--default"], Some(&tools))
            .status
            .success()
    );
}

#[cfg(unix)]
#[test]
fn wasm_client_pipeline_uses_reviewable_tool_outputs() {
    let project = Project::new();
    let tools = project.install_wasm_tools();

    let output = project.run_with_path(&["build:client", "--debug"], Some(&tools));
    assert!(
        output.status.success(),
        "Wasm build failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    project.assert_files(&[
        "target/wasm32-unknown-unknown/debug/scaffold_fixture.wasm",
        "static/scaffold_fixture.js",
        "static/rullst-islands.js",
    ]);
    let manifest = fs::read_to_string(project.root.join("Cargo.toml")).expect("Wasm manifest");
    assert!(manifest.contains("cdylib"));
    let orchestrator = fs::read_to_string(project.root.join("static/rullst-islands.js"))
        .expect("hydration orchestrator");
    assert!(orchestrator.contains("await initialization"));
    assert!(orchestrator.contains("./scaffold_fixture.js"));
}
