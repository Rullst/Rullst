#![allow(clippy::unwrap_used, clippy::expect_used)]

use cargo_rullst::blueprints::{BLANK_BLUEPRINT_ID, ERP_BLUEPRINT_ID, SAAS_BLUEPRINT_ID};
use cargo_rullst::generators::project::cargo_toml::build_cargo_toml;
use std::fs;
use std::process::Command;

#[test]
fn generated_saas_blueprint_passes_cargo_check() {
    let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir.parent().expect("workspace root");
    let project_dir =
        std::env::temp_dir().join(format!("rullst-generated-saas-{}", rand::random::<u64>()));
    fs::create_dir_all(&project_dir).expect("temporary generated project");

    let manifest = build_cargo_toml(
        "generated_saas",
        false,
        true,
        "Sqlite",
        false,
        false,
        SAAS_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        workspace,
    )
    .expect("generated Cargo.toml");
    let manifest = cargo_rullst::generators::cors_jwt::ensure_jwt_dependencies(&manifest)
        .expect("JWT dependencies");
    fs::write(project_dir.join("Cargo.toml"), manifest).expect("write generated manifest");
    cargo_rullst::blueprints::apply(
        SAAS_BLUEPRINT_ID,
        &project_dir,
        "generated-saas",
        "generated_saas",
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .expect("apply SaaS blueprint");

    let workers_dir = project_dir.join("src/workers");
    fs::create_dir_all(&workers_dir).expect("generated workers directory");
    let workers_module = format!(
        "pub mod smoke_worker;\npub fn register_workers(worker: &mut rullst::queue::Worker) {{\n    smoke_worker::register(worker);\n}}\n{}",
        cargo_rullst::generators::worker::worker_start_helper()
    );
    fs::write(workers_dir.join("mod.rs"), workers_module).expect("generated worker registry");
    fs::write(
        workers_dir.join("smoke_worker.rs"),
        cargo_rullst::generators::worker::render_worker_source("smoke"),
    )
    .expect("generated worker handler");
    fs::write(
        project_dir.join("src/middlewares/jwt_middleware.rs"),
        cargo_rullst::generators::cors_jwt::jwt_middleware_template(),
    )
    .expect("generated JWT middleware");
    let middlewares_mod_path = project_dir.join("src/middlewares/mod.rs");
    let middlewares_mod =
        fs::read_to_string(&middlewares_mod_path).expect("generated middlewares module");
    fs::write(
        middlewares_mod_path,
        format!("{middlewares_mod}pub mod jwt_middleware;\n"),
    )
    .expect("register generated JWT middleware");
    let main_path = project_dir.join("src/main.rs");
    let main_source = fs::read_to_string(&main_path).expect("generated main source");
    let worker_lifecycle_smoke = r#"
#[allow(dead_code)]
fn start_generated_workers(
    queue: &rullst::Queue,
) -> Result<rullst::queue::WorkerHandle, rullst::queue::QueueError> {
    let worker = rullst::queue::Worker::new(queue);
    let worker_handle = workers::start_workers(worker)?;
    Ok(worker_handle)
}
"#;
    fs::write(
        main_path,
        format!("mod workers;\n{worker_lifecycle_smoke}\n{main_source}"),
    )
    .expect("register generated worker module and lifecycle smoke");

    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--offline")
        .arg("--all-targets")
        .arg("--manifest-path")
        .arg(project_dir.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            workspace.join("target/generated-scaffold-check"),
        )
        .output()
        .expect("run cargo check for generated SaaS project");

    if !output.status.success() {
        panic!(
            "generated SaaS failed cargo check\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fs::remove_dir_all(project_dir).expect("temporary generated project cleanup");
}

#[test]
fn generated_hot_blank_with_island_passes_cargo_check() {
    let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir.parent().expect("workspace root");
    let project_dir =
        std::env::temp_dir().join(format!("rullst-generated-blank-{}", rand::random::<u64>()));
    fs::create_dir_all(&project_dir).expect("temporary generated project");

    let manifest = build_cargo_toml(
        "dummy-test",
        true,
        false,
        "Sqlite",
        false,
        false,
        BLANK_BLUEPRINT_ID,
        "Wasm Island",
        workspace,
    )
    .expect("generated Cargo.toml");
    fs::write(project_dir.join("Cargo.toml"), manifest).expect("write generated manifest");
    cargo_rullst::blueprints::apply(
        BLANK_BLUEPRINT_ID,
        &project_dir,
        "dummy-test",
        "dummy_test",
        false,
        true,
        false,
        "Active Record",
        "Wasm Island",
    )
    .expect("apply blank blueprint");

    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--offline")
        .arg("--all-targets")
        .arg("--manifest-path")
        .arg(project_dir.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            workspace.join("target/generated-scaffold-check"),
        )
        .output()
        .expect("run cargo check for generated blank project");

    if !output.status.success() {
        panic!(
            "generated hot blank failed cargo check\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fs::remove_dir_all(project_dir).expect("temporary generated project cleanup");
}

#[test]
fn generated_erp_admin_routes_pass_cargo_check() {
    let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir.parent().expect("workspace root");
    let project_dir =
        std::env::temp_dir().join(format!("rullst-generated-erp-{}", rand::random::<u64>()));
    fs::create_dir_all(&project_dir).expect("temporary generated ERP project");

    let manifest = build_cargo_toml(
        "generated-erp",
        false,
        true,
        "Sqlite",
        false,
        false,
        ERP_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        workspace,
    )
    .expect("generated ERP Cargo.toml");
    fs::write(project_dir.join("Cargo.toml"), manifest).expect("write generated ERP manifest");
    cargo_rullst::blueprints::apply(
        ERP_BLUEPRINT_ID,
        &project_dir,
        "generated-erp",
        "generated_erp",
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .expect("apply ERP blueprint");

    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--offline")
        .arg("--all-targets")
        .arg("--manifest-path")
        .arg(project_dir.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            workspace.join("target/generated-scaffold-check"),
        )
        .output()
        .expect("run cargo check for generated ERP project");

    if !output.status.success() {
        panic!(
            "generated ERP failed cargo check\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fs::remove_dir_all(project_dir).expect("temporary generated ERP project cleanup");
}
