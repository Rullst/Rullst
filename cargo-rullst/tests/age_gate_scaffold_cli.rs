//! Execute the opt-in declaration journey in a generated authenticated SaaS.
use cargo_rullst::{
    blueprints::{self, SAAS_BLUEPRINT_ID},
    generators::project::cargo_toml::build_cargo_toml,
};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn materialize(root: &Path, workspace: &Path, hot: bool) {
    fs::create_dir_all(root).unwrap();
    let mut manifest = build_cargo_toml(
        "age-gate-consumer",
        hot,
        true,
        "Sqlite",
        &[],
        false,
        false,
        SAAS_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        workspace,
    )
    .unwrap();
    manifest.push_str("\n[dev-dependencies]\ntower = { version = \"0.5\", features = [\"util\"] }\nhttp-body-util = \"0.1\"\n\n[profile.test]\ndebug = 0\nincremental = false\n");
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    fs::copy(workspace.join("Cargo.lock"), root.join("Cargo.lock")).unwrap();
    blueprints::apply(
        SAAS_BLUEPRINT_ID,
        root,
        "age-gate-consumer",
        "age_gate_consumer",
        false,
        hot,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
}

fn install(root: &Path, source: &Path, profile: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(root)
        .args(["make:age-gate", "--privacy-source"])
        .arg(source)
        .args([
            "--minimum-age",
            "18",
            "--policy-version",
            "dashboard-v1",
            "--tenant-ref",
            "tenant-alpha",
            "--replay-store",
            profile,
        ])
        .output()
        .unwrap()
}

#[test]
fn generated_age_gate_compiles_and_enforces_the_authenticated_journey() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("app");
    materialize(&project, workspace, true);
    success(
        Command::new("cargo")
            .current_dir(&project)
            .args(["fmt", "--all"])
            .output()
            .unwrap(),
    );
    success(install(
        &project,
        &workspace.join("rullst-privacy"),
        "sqlite",
    ));
    let original_manifest = fs::read(project.join("Cargo.toml")).unwrap();
    assert!(
        !install(&project, &workspace.join("rullst-privacy"), "sqlite")
            .status
            .success()
    );
    assert_eq!(
        fs::read(project.join("Cargo.toml")).unwrap(),
        original_manifest
    );
    fs::create_dir_all(project.join("tests")).unwrap();
    fs::write(
        project.join("tests/age_journey.rs"),
        include_str!("fixtures/age_gate_journey.rs.template"),
    )
    .unwrap();
    let app_key: String = (0..4)
        .map(|_| format!("{:016x}", rand::random::<u64>()))
        .collect();
    let privacy_key: String = (0..4)
        .map(|_| format!("{:016x}", rand::random::<u64>()))
        .collect();
    for configuration in ["valid", "missing"] {
        let mut cargo = Command::new("cargo");
        cargo
            .current_dir(&project)
            .args([
                "test",
                "--test",
                "age_journey",
                "--offline",
                "-j",
                "2",
                "--",
                "--nocapture",
            ])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("APP_KEY", &app_key)
            .env("RULLST_ENV", "development")
            .env("RULLST_AGE_KEY_ID", "test-epoch")
            .env("RULLST_AGE_TEST_CONFIGURATION", configuration)
            .env(
                "RULLST_AGE_REPLAY_DATABASE",
                temporary.path().join(format!("{configuration}.sqlite")),
            )
            .env("BILLING_PROVIDER", "stripe")
            .env("BILLING_API_KEY", "mock_age_gate_test")
            .env("BILLING_WEBHOOK_SECRET", "mock_age_gate_test");
        if configuration == "valid" {
            cargo.env("RULLST_AGE_KEY_HEX", &privacy_key);
        } else {
            cargo.env_remove("RULLST_AGE_KEY_HEX");
        }
        success(cargo.output().unwrap());
    }
    // The other storage profile is compiled against its actual public API.
    // Its database lifecycle is executed by the owned privacy PostgreSQL wrapper.
    let postgres = temporary.path().join("postgres-app");
    materialize(&postgres, workspace, false);
    success(
        Command::new("cargo")
            .current_dir(&postgres)
            .args(["fmt", "--all"])
            .output()
            .unwrap(),
    );
    success(install(
        &postgres,
        &workspace.join("rullst-privacy"),
        "postgres",
    ));
    success(
        Command::new("cargo")
            .current_dir(&postgres)
            .args(["check", "--offline", "-j", "2"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_PROFILE_DEV_DEBUG", "0")
            .output()
            .unwrap(),
    );
}

#[test]
fn unknown_authentication_and_missing_source_are_rejected_without_changing_the_app() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("app");
    materialize(&project, workspace, false);
    let original = fs::read(project.join("Cargo.toml")).unwrap();
    let auth = project.join("src/middlewares/auth_middleware.rs");
    fs::write(&auth, "// Application-owned custom authentication\n").unwrap();
    let result = install(&project, &workspace.join("rullst-privacy"), "sqlite");
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("recognized SaaS authentication"));
    assert_eq!(fs::read(project.join("Cargo.toml")).unwrap(), original);
    assert!(!project.join("src/controllers/age_controller.rs").exists());
    assert!(
        !install(&project, &temporary.path().join("missing"), "sqlite")
            .status
            .success()
    );
}
