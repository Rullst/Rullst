//! Real CLI, generated application, operator process and authenticated HTTP gates.
use cargo_rullst::{blueprints, generators::project::cargo_toml::build_cargo_toml};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
static BUILDS: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
fn materialize(root: &Path, workspace: &Path, hot: bool, database: &str) {
    fs::create_dir_all(root).unwrap();
    let mut manifest = build_cargo_toml(
        "supervision-consumer",
        hot,
        true,
        database,
        &[],
        false,
        false,
        blueprints::LMS_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        workspace,
    )
    .unwrap();
    manifest.push_str("\n[dev-dependencies]\naxum = \"0.8.9\"\ntower = { version = \"0.5\", features = [\"util\"] }\nhttp-body-util = \"0.1\"\n\n[profile.test]\ndebug = 0\nincremental = false\n");
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    fs::copy(workspace.join("Cargo.lock"), root.join("Cargo.lock")).unwrap();
    blueprints::apply(
        blueprints::LMS_BLUEPRINT_ID,
        root,
        "supervision-consumer",
        "supervision_consumer",
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
fn install(root: &Path, workspace: &Path) -> Output {
    install_collection(root, workspace, "visibility")
}
fn install_collection(root: &Path, workspace: &Path, collection: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(root)
        .args([
            "make:supervision",
            "--policy-version",
            "exam-v1",
            "--notice-version",
            "notice-v1",
            "--retention-seconds",
            "3600",
            "--session-seconds",
            "600",
            "--browser-observations",
            collection,
            "--supervision-source",
        ])
        .arg(workspace.join("rullst-supervision"))
        .output()
        .unwrap()
}
fn cargo(root: &Path, workspace: &Path) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_BUILD_JOBS", "1");
    command
}

#[test]
fn default_cli_project_composes_age_privacy_then_supervision() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let cli = env!("CARGO_BIN_EXE_rullst");
    success(
        Command::new(cli)
            .current_dir(temp.path())
            .env("RULLST_DISABLE_UPDATE_CHECK", "true")
            .args([
                "new",
                "packaged-lms",
                "--default",
                "--blueprint",
                "lms",
                "--skip-initial-migration",
            ])
            .output()
            .unwrap(),
    );
    let root = temp.path().join("packaged-lms");
    for args in [
        vec![
            "make:age-gate",
            "--blueprint",
            "lms",
            "--minimum-age",
            "18",
            "--policy-version",
            "archive-v1",
            "--replay-store",
            "sqlite",
        ],
        vec![
            "make:privacy",
            "--blueprint",
            "lms",
            "--purpose-version",
            "archive-v1",
            "--validity-seconds",
            "3600",
        ],
    ] {
        success(
            Command::new(cli)
                .current_dir(&root)
                .args(args)
                .output()
                .unwrap(),
        );
    }
    success(install(&root, workspace));
    assert!(root.join("SUPERVISION.md").is_file());
    let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
    for name in [
        "age_controller",
        "privacy_controller",
        "supervision_controller",
    ] {
        assert!(main.contains(name));
    }
    success(
        Command::new(cli)
            .current_dir(&root)
            .args(["generate:ai-context", "--check"])
            .output()
            .unwrap(),
    );
}

#[test]
fn supervision_refuses_custom_authorization_outputs_and_other_backends() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temp = tempfile::tempdir().unwrap();
    for database in ["Sqlite", "Postgres"] {
        let root = temp.path().join(database);
        materialize(&root, workspace, false, database);
        let original = fs::read(root.join("Cargo.toml")).unwrap();
        if database == "Postgres" {
            assert!(!install(&root, workspace).status.success());
        } else {
            let service = root.join("src/services/learning_service.rs");
            let saved = fs::read(&service).unwrap();
            fs::write(&service, "// custom authorization").unwrap();
            assert!(!install(&root, workspace).status.success());
            fs::write(&service, saved).unwrap();
            fs::write(
                root.join("src/controllers/supervision_controller.rs"),
                "// preserve application work",
            )
            .unwrap();
            assert!(!install(&root, workspace).status.success());
        }
        assert_eq!(fs::read(root.join("Cargo.toml")).unwrap(), original);
        assert!(!root.join("SUPERVISION.md").exists());
    }
}

#[test]
fn real_supervision_operator_and_lms_http_journey() {
    let _build = BUILDS.lock().unwrap_or_else(|error| error.into_inner());
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("app");
    materialize(&root, workspace, true, "Sqlite");
    success(install_collection(
        &root,
        workspace,
        "visibility,focus,clipboard,fullscreen",
    ));
    success(
        Command::new(env!("CARGO_BIN_EXE_rullst"))
            .current_dir(&root)
            .args(["generate:ai-context", "--check"])
            .output()
            .unwrap(),
    );
    let before = fs::read(root.join("Cargo.toml")).unwrap();
    assert!(!install(&root, workspace).status.success());
    assert_eq!(before, fs::read(root.join("Cargo.toml")).unwrap());
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("tests/supervision_journey.rs"),
        include_str!("fixtures/supervision_journey.rs.template"),
    )
    .unwrap();
    success(
        cargo(&root, workspace)
            .args(["test", "--no-run", "--test", "supervision_journey"])
            .output()
            .unwrap(),
    );
    let binary = workspace
        .join("target/debug")
        .join(format!("supervision-admin{}", std::env::consts::EXE_SUFFIX));
    let owned = temp
        .path()
        .join(format!("supervision-admin{}", std::env::consts::EXE_SUFFIX));
    fs::copy(binary, &owned).unwrap();
    let db = temp.path().join("supervision.sqlite");
    let init = temp.path().join("init.json");
    fs::write(&init,r#"{"tenant":"academy-demo","operator":"operator-7","evidence":"case-42","command":{"operation":"initialize"}}"#).unwrap();
    let operator = || {
        let mut command = Command::new(&owned);
        command
            .arg(&init)
            .env("RULLST_SUPERVISION_DATABASE", &db)
            .env("RULLST_SUPERVISION_EPOCH", "consumer-test-epoch");
        command
    };
    success(operator().output().unwrap());
    assert!(!operator().output().unwrap().status.success());
    let key: String = (0..4)
        .map(|_| format!("{:016x}", rand::random::<u64>()))
        .collect();
    let form_key: String = (0..4)
        .map(|_| format!("{:016x}", rand::random::<u64>()))
        .collect();
    for arguments in [
        vec!["test", "--lib"],
        vec![
            "clippy",
            "--lib",
            "--bins",
            "--",
            "-D",
            "warnings",
            "-D",
            "clippy::unwrap_used",
            "-D",
            "clippy::expect_used",
            "-D",
            "clippy::panic",
            "-D",
            "clippy::todo",
            "-D",
            "clippy::unimplemented",
        ],
    ] {
        success(
            cargo(&root, workspace)
                .args(arguments)
                .env("APP_KEY", &key)
                .env("RULLST_ENV", "development")
                .env("RULLST_SUPERVISION_DATABASE", &db)
                .env("RULLST_SUPERVISION_EPOCH", "consumer-test-epoch")
                .env("RULLST_SUPERVISION_FORM_KEY_HEX", &form_key)
                .output()
                .unwrap(),
        );
    }
    success(
        cargo(&root, workspace)
            .args(["test", "--test", "supervision_journey", "--", "--nocapture"])
            .env("APP_KEY", &key)
            .env("RULLST_ENV", "development")
            .env("RULLST_SUPERVISION_DATABASE", &db)
            .env("RULLST_SUPERVISION_EPOCH", "consumer-test-epoch")
            .env("RULLST_SUPERVISION_FORM_KEY_HEX", &form_key)
            .env("RULLST_SUPERVISION_OPERATOR", &owned)
            .env(
                "RULLST_SUPERVISION_BROWSER_SCRIPT",
                workspace.join(".github/supervision-browser-smoke.mjs"),
            )
            .output()
            .unwrap(),
    );
}

#[test]
fn privacy_age_and_supervision_compose_without_replacing_application_instructions() {
    let _build = BUILDS.lock().unwrap_or_else(|error| error.into_inner());
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temp = tempfile::tempdir().unwrap();
    for (name, hot, order) in [
        (
            "linked",
            false,
            ["make:privacy", "make:supervision", "make:age-gate"],
        ),
        (
            "library",
            true,
            ["make:age-gate", "make:supervision", "make:privacy"],
        ),
    ] {
        let root = temp.path().join(name);
        materialize(&root, workspace, hot, "Sqlite");
        fs::write(
            root.join("AGENTS.md"),
            "Preserve application-owned guidance.\n",
        )
        .unwrap();
        for command in order {
            if command == "make:supervision" {
                success(install(&root, workspace));
            } else {
                let mut cli = Command::new(env!("CARGO_BIN_EXE_rullst"));
                cli.current_dir(&root)
                    .args([command, "--blueprint", "lms", "--privacy-source"])
                    .arg(workspace.join("rullst-privacy"));
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
                        "age-v1",
                        "--replay-store",
                        "sqlite",
                    ]);
                }
                success(cli.output().unwrap());
            }
            success(
                Command::new(env!("CARGO_BIN_EXE_rullst"))
                    .current_dir(&root)
                    .args(["generate:ai-context", "--check"])
                    .output()
                    .unwrap(),
            );
        }
        assert_eq!(
            fs::read_to_string(root.join("AGENTS.md")).unwrap(),
            "Preserve application-owned guidance.\n"
        );
        // Public directly linked shape compiles with all three consumers together.
        if !hot {
            success(
                cargo(&root, workspace)
                    .args(["check", "--all-targets"])
                    .output()
                    .unwrap(),
            );
        }
    }
}

#[test]
fn supervision_rejects_unknown_or_duplicate_browser_capabilities_before_writing() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temp = tempfile::tempdir().unwrap();
    for selection in [
        "",
        "camera",
        "visibility,visibility",
        "clipboard, keys",
        "visibility,",
        "screen",
        "focus,audio",
    ] {
        assert!(
            !install_collection(temp.path(), workspace, selection)
                .status
                .success()
        );
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    }
}
