#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(command: &mut Command, action: &str) -> Output {
    command
        .output()
        .unwrap_or_else(|error| panic!("could not {action}: {error}"))
}

fn assert_success(output: &Output, action: &str) {
    assert!(
        output.status.success(),
        "{action} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn install_workspace_lock(project: &Path, workspace: &Path) {
    fs::copy(workspace.join("Cargo.lock"), project.join("Cargo.lock"))
        .expect("copy workspace lockfile into generated IoT project");
}

fn clean_generated_package(project: &Path, workspace: &Path, package_name: &str) {
    let cleaned = run(
        Command::new("cargo")
            .current_dir(project)
            .args(["clean", "--package", package_name])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "clean generated IoT package",
    );
    assert_success(&cleaned, "generated IoT package cleanup");
}

#[test]
fn iot_scaffold_compiles_registers_the_module_and_fails_closed() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let project =
        std::env::temp_dir().join(format!("rullst-iot-scaffold-{}", rand::random::<u64>()));
    let cli = env!("CARGO_BIN_EXE_rullst");

    let generated = run(
        Command::new(cli)
            .current_dir(workspace)
            .arg("new")
            .arg(&project)
            .args([
                "--default",
                "--api",
                "--database",
                "sqlite",
                "--skip-initial-migration",
            ]),
        "generate base project",
    );
    assert_success(&generated, "base project generation");

    let scaffolded = run(
        Command::new(cli)
            .current_dir(&project)
            .args(["make:iot", "TemperatureSensor"]),
        "scaffold IoT telemetry module",
    );
    assert_success(&scaffolded, "IoT telemetry generation");

    let module_path = project.join("src/iot/temperature_sensor.rs");
    let module = fs::read_to_string(&module_path).expect("generated IoT module");
    assert!(module.contains("use rullst::iot::SensorTelemetry;"));
    assert!(module.contains("pub struct TemperatureSensorDevice"));
    assert!(
        fs::read_to_string(project.join("src/iot/mod.rs"))
            .expect("IoT registry")
            .contains("pub mod temperature_sensor;")
    );
    assert!(
        fs::read_to_string(project.join("src/main.rs"))
            .expect("project root")
            .contains("pub mod iot;")
    );

    let manifest = fs::read_to_string(project.join("Cargo.toml")).expect("generated manifest");
    let parsed: toml::Value = toml::from_str(&manifest).expect("valid generated manifest");
    let package_name = parsed["package"]["name"]
        .as_str()
        .expect("generated package name")
        .to_string();
    let features = parsed["dependencies"]["rullst"]["features"]
        .as_array()
        .expect("rullst feature array");
    assert_eq!(
        features
            .iter()
            .filter(|feature| feature.as_str() == Some("iot"))
            .count(),
        1,
        "iot feature must be enabled exactly once"
    );
    install_workspace_lock(&project, workspace);

    let checked = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "Clippy generated IoT project",
    );
    assert_success(&checked, "generated IoT project Clippy");

    let module_before = fs::read_to_string(&module_path).expect("generated module");
    let duplicate = run(
        Command::new(cli)
            .current_dir(&project)
            .args(["make:iot", "TemperatureSensor"]),
        "rerun IoT generator",
    );
    assert!(!duplicate.status.success(), "rerun must fail closed");
    assert_eq!(
        fs::read_to_string(&module_path).expect("preserved module"),
        module_before
    );

    let traversal = run(
        Command::new(cli)
            .current_dir(&project)
            .args(["make:iot", "../../escape"]),
        "try unsafe IoT name",
    );
    assert!(!traversal.status.success(), "unsafe name must be rejected");
    assert!(!project.join("escape.rs").exists());

    clean_generated_package(&project, workspace, &package_name);
    fs::remove_dir_all(&project).expect("remove generated project");
}
