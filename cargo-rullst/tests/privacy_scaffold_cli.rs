//! Exercise real generated privacy routes, authenticated scopes and SQL effects.
use cargo_rullst::{blueprints, generators::project::cargo_toml::build_cargo_toml};
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

fn materialize(root: &Path, workspace: &Path, blueprint: &str, hot: bool, database: &str) {
    fs::create_dir_all(root).unwrap();
    let id = if blueprint == "saas" {
        blueprints::SAAS_BLUEPRINT_ID
    } else {
        blueprints::LMS_BLUEPRINT_ID
    };
    let mut manifest = build_cargo_toml(
        "privacy-consumer",
        hot,
        true,
        database,
        &[],
        false,
        false,
        id,
        "Zero-Bundle HTMX",
        workspace,
    )
    .unwrap();
    manifest.push_str("\n[dev-dependencies]\ntower = { version = \"0.5\", features = [\"util\"] }\nhttp-body-util = \"0.1\"\n\n[profile.test]\ndebug = 0\nincremental = false\n");
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    fs::copy(workspace.join("Cargo.lock"), root.join("Cargo.lock")).unwrap();
    blueprints::apply(
        id,
        root,
        "privacy-consumer",
        "privacy_consumer",
        false,
        hot,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
    success(
        Command::new("cargo")
            .current_dir(root)
            .args(["fmt", "--all"])
            .output()
            .unwrap(),
    );
}

fn install(root: &Path, workspace: &Path, blueprint: &str, command: &str) -> Output {
    let mut cli = Command::new(env!("CARGO_BIN_EXE_rullst"));
    cli.current_dir(root)
        .args([command, "--blueprint", blueprint, "--privacy-source"])
        .arg(workspace.join("rullst-privacy"));
    if blueprint == "saas" {
        cli.args(["--tenant-ref", "tenant-alpha"]);
    }
    if command == "make:privacy" {
        cli.args([
            "--purpose-version",
            "greeting-v1",
            "--validity-seconds",
            "3600",
        ]);
    } else {
        cli.args([
            "--minimum-age",
            "18",
            "--policy-version",
            "dashboard-v1",
            "--replay-store",
            "sqlite",
        ]);
    }
    cli.output().unwrap()
}

fn cargo(root: &Path, workspace: &Path) -> Command {
    let mut cargo = Command::new("cargo");
    cargo
        .current_dir(root)
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_PROFILE_DEV_DEBUG", "0");
    cargo
}

#[test]
fn generated_privacy_choices_and_export_work_with_both_age_installation_orders() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let app_key: String = (0..4)
        .map(|_| format!("{:016x}", rand::random::<u64>()))
        .collect();
    let form_key: String = (0..4)
        .map(|_| format!("{:016x}", rand::random::<u64>()))
        .collect();
    for blueprint in ["saas", "lms"] {
        let project = temporary.path().join(blueprint);
        materialize(&project, workspace, blueprint, true, "Sqlite");
        let commands = if blueprint == "saas" {
            ["make:privacy", "make:age-gate"]
        } else {
            ["make:age-gate", "make:privacy"]
        };
        for command in commands {
            success(install(&project, workspace, blueprint, command));
        }
        let before = fs::read(project.join("Cargo.toml")).unwrap();
        assert!(
            !install(&project, workspace, blueprint, "make:privacy")
                .status
                .success()
        );
        assert_eq!(before, fs::read(project.join("Cargo.toml")).unwrap());
        fs::create_dir_all(project.join("tests")).unwrap();
        fs::write(
            project.join("tests/privacy_journey.rs"),
            include_str!("fixtures/privacy_journey.rs.template"),
        )
        .unwrap();
        // Build once, then retain this project's exact executable. Alternating
        // cargo run/test would repeatedly rebuild different dependency features.
        // Each bootstrap still runs in a fresh process with its own environment.
        success(
            cargo(&project, workspace)
                .args(["build", "--bin", "privacy-init", "-j", "2"])
                .output()
                .unwrap(),
        );
        let binary = format!("privacy-init{}", std::env::consts::EXE_SUFFIX);
        let bootstrap = project.join(format!("privacy-bootstrap{}", std::env::consts::EXE_SUFFIX));
        fs::copy(workspace.join("target/debug").join(binary), &bootstrap).unwrap();
        for configuration in ["valid", "missing-key", "missing-store"] {
            eprintln!("privacy consumer: {blueprint}, {configuration}");
            let database = temporary
                .path()
                .join(format!("{blueprint}-{configuration}.sqlite"));
            if configuration != "missing-store" {
                success(
                    Command::new(&bootstrap)
                        .current_dir(&project)
                        .env("RULLST_PRIVACY_DATABASE", &database)
                        .output()
                        .unwrap(),
                );
                assert!(
                    !Command::new(&bootstrap)
                        .current_dir(&project)
                        .env("RULLST_PRIVACY_DATABASE", &database)
                        .output()
                        .unwrap()
                        .status
                        .success()
                );
            }
            let mut test = cargo(&project, workspace);
            test.args([
                "test",
                "--test",
                "privacy_journey",
                "-j",
                "2",
                "--",
                "--nocapture",
            ])
            .env("APP_KEY", &app_key)
            .env("RULLST_ENV", "development")
            .env("RULLST_PRIVACY_TEST_BLUEPRINT", blueprint)
            .env("RULLST_PRIVACY_TEST_CONFIGURATION", configuration)
            .env("RULLST_PRIVACY_DATABASE", &database)
            .env("BILLING_PROVIDER", "stripe")
            .env("BILLING_API_KEY", "mock_privacy_test")
            .env("BILLING_WEBHOOK_SECRET", "mock_privacy_test");
            if configuration == "missing-key" {
                test.env_remove("RULLST_PRIVACY_FORM_KEY_HEX");
            } else {
                test.env("RULLST_PRIVACY_FORM_KEY_HEX", &form_key);
            }
            success(test.output().unwrap());
            if configuration == "missing-store" {
                assert!(!database.exists());
            }
        }
    }
    // QueryBuilder infers the actual strict backend and binds its placeholders.
    let postgres = temporary.path().join("postgres");
    materialize(&postgres, workspace, "saas", false, "Postgres");
    success(install(&postgres, workspace, "saas", "make:privacy"));
    success(
        cargo(&postgres, workspace)
            .args(["check", "--all-targets", "-j", "2"])
            .output()
            .unwrap(),
    );
    // All owned children exited; clean only the generated test package.
    success(
        cargo(&postgres, workspace)
            .args(["clean", "--package", "privacy-consumer"])
            .output()
            .unwrap(),
    );
}

#[test]
fn generator_rejects_custom_authentication_and_existing_outputs_without_mutation() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("app");
    materialize(&project, workspace, "saas", false, "Sqlite");
    let before = fs::read(project.join("Cargo.toml")).unwrap();
    let auth = project.join("src/middlewares/auth_middleware.rs");
    let original_auth = fs::read(&auth).unwrap();
    fs::write(&auth, "// application-owned authentication").unwrap();
    assert!(
        !install(&project, workspace, "saas", "make:privacy")
            .status
            .success()
    );
    assert_eq!(before, fs::read(project.join("Cargo.toml")).unwrap());
    fs::write(auth, original_auth).unwrap();
    let custom = project.join("src/controllers/privacy_controller.rs");
    fs::write(&custom, "// preserve this file").unwrap();
    assert!(
        !install(&project, workspace, "saas", "make:privacy")
            .status
            .success()
    );
    assert_eq!(before, fs::read(project.join("Cargo.toml")).unwrap());
    assert_eq!(fs::read_to_string(custom).unwrap(), "// preserve this file");
    assert!(!project.join("PRIVACY.md").exists());
}
