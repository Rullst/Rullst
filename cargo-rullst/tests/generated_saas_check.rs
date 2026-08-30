//! Compile representative generated projects across every public blueprint.
//!
//! The structural suite covers all 270 combinations. This slower suite selects
//! the smallest set that crosses every public blueprint plus the distinct ORM,
//! frontend, API, database, hot-reload and release-build boundaries. The LMS
//! case runs its generated tests as well, so template-only authorization
//! regressions cannot hide behind a successful `cargo check`.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use cargo_rullst::blueprints::{
    self, BLANK_BLUEPRINT_ID, BLOG_BLUEPRINT_ID, ERP_BLUEPRINT_ID, LMS_BLUEPRINT_ID,
    PORTFOLIO_BLUEPRINT_ID, SAAS_BLUEPRINT_ID,
};
use cargo_rullst::generators::project::cargo_toml::build_cargo_toml;
use std::{fs, path::Path, path::PathBuf, process::Command};

#[derive(Clone, Copy)]
struct GeneratedCase {
    name: &'static str,
    blueprint: usize,
    api: bool,
    hot_reload: bool,
    db_needed: bool,
    orm_pattern: &'static str,
    frontend: &'static str,
    release: bool,
}

const GENERATED_CASES: [GeneratedCase; 7] = [
    GeneratedCase {
        name: "blank-minimal",
        blueprint: BLANK_BLUEPRINT_ID,
        api: false,
        hot_reload: false,
        db_needed: false,
        orm_pattern: "Active Record",
        frontend: "Zero-Bundle HTMX",
        release: false,
    },
    GeneratedCase {
        name: "blank-api-hot-wasm",
        blueprint: BLANK_BLUEPRINT_ID,
        api: true,
        hot_reload: true,
        db_needed: false,
        orm_pattern: "Active Record",
        frontend: "Wasm Island",
        release: false,
    },
    GeneratedCase {
        name: "lms-repository-liveview",
        blueprint: LMS_BLUEPRINT_ID,
        api: false,
        hot_reload: false,
        db_needed: true,
        orm_pattern: "Repository",
        frontend: "LiveView",
        release: false,
    },
    GeneratedCase {
        name: "saas-active-htmx",
        blueprint: SAAS_BLUEPRINT_ID,
        api: false,
        hot_reload: false,
        db_needed: true,
        orm_pattern: "Active Record",
        frontend: "Zero-Bundle HTMX",
        release: false,
    },
    GeneratedCase {
        name: "blog-hybrid-tera",
        blueprint: BLOG_BLUEPRINT_ID,
        api: false,
        hot_reload: false,
        db_needed: true,
        orm_pattern: "Hybrid",
        frontend: "Tera Template",
        release: false,
    },
    GeneratedCase {
        name: "portfolio-repository-pico",
        blueprint: PORTFOLIO_BLUEPRINT_ID,
        api: false,
        hot_reload: false,
        db_needed: true,
        orm_pattern: "Repository",
        frontend: "Pico CSS",
        release: false,
    },
    GeneratedCase {
        name: "erp-hybrid-release",
        blueprint: ERP_BLUEPRINT_ID,
        api: false,
        hot_reload: false,
        db_needed: true,
        orm_pattern: "Hybrid",
        frontend: "Zero-Bundle HTMX",
        release: true,
    },
];

fn materialize(case: GeneratedCase, project_dir: &Path, workspace: &Path) {
    fs::create_dir_all(project_dir).expect("temporary generated project");
    let package_name = format!("generated-{}", case.name);
    let module_name = package_name.replace('-', "_");
    let manifest = build_cargo_toml(
        &package_name,
        case.hot_reload,
        case.db_needed,
        "Sqlite",
        &[],
        false,
        false,
        case.blueprint,
        case.frontend,
        workspace,
    )
    .unwrap_or_else(|error| panic!("{}: generated Cargo.toml: {error}", case.name));
    let manifest = if case.blueprint == SAAS_BLUEPRINT_ID {
        cargo_rullst::generators::cors_jwt::ensure_jwt_dependencies(&manifest)
            .unwrap_or_else(|error| panic!("{}: JWT dependencies: {error}", case.name))
    } else {
        manifest
    };
    fs::write(project_dir.join("Cargo.toml"), manifest)
        .unwrap_or_else(|error| panic!("{}: write Cargo.toml: {error}", case.name));

    let workspace_lock = workspace.join("Cargo.lock");
    if workspace_lock.exists() {
        let _ = fs::copy(&workspace_lock, project_dir.join("Cargo.lock"));
    }

    blueprints::apply(
        case.blueprint,
        project_dir,
        &package_name,
        &module_name,
        case.api,
        case.hot_reload,
        case.db_needed,
        case.orm_pattern,
        case.frontend,
    )
    .unwrap_or_else(|error| panic!("{}: apply blueprint: {error}", case.name));

    if case.blueprint == SAAS_BLUEPRINT_ID {
        add_saas_generator_smoke(project_dir);
    }
}

fn add_saas_generator_smoke(project_dir: &Path) {
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
    .expect("register generated worker lifecycle smoke");
}

fn cargo_verify(case: GeneratedCase, project_dir: &Path, workspace: &Path) {
    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("target"));
    let mut command = Command::new(env!("CARGO"));
    let cargo_operation = if case.blueprint == LMS_BLUEPRINT_ID {
        "test"
    } else {
        "check"
    };
    command
        .arg(cargo_operation)
        .arg("--offline")
        .arg("--all-targets");
    if case.release {
        command.arg("--release");
    }
    let output = command
        .arg("--manifest-path")
        .arg(project_dir.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            target_root.join("generated-scaffold-check"),
        )
        .output()
        .unwrap_or_else(|error| panic!("{}: run cargo check: {error}", case.name));

    if !output.status.success() {
        panic!(
            "{} failed cargo {}\nstdout:\n{}\nstderr:\n{}",
            case.name,
            cargo_operation,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
// TM-ACADEMY-02: the materialized LMS executes its owner/cross-user denial test.
// TM-ACADEMY-03: it fails closed on prerequisite, release, expiry and policy conflicts.
// TM-ACADEMY-04: authenticated school membership scopes database and HTTP mutations.
// TM-ACADEMY-05: it grades server-side and rejects cross-user/tampered quiz submissions.
// TM-ACADEMY-06: it also executes score identity/schema/bounds negative tests.
// TM-ACADEMY-07: its outbox payload drives a strict, side-effect-free automation plan.
// TM-ACADEMY-08: independent review and enrollment pins prevent silent content promotion.
// TM-ACADEMY-10: the materialized LMS exercises minimized school-scoped privacy lifecycle state.
fn every_blueprint_and_distinct_generated_boundary_passes_cargo_verification() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir.parent().expect("workspace root");

    for case in GENERATED_CASES {
        let project_dir = std::env::temp_dir().join(format!(
            "rullst-generated-{}-{}",
            case.name,
            rand::random::<u64>()
        ));
        materialize(case, &project_dir, workspace);
        cargo_verify(case, &project_dir, workspace);
        fs::remove_dir_all(&project_dir)
            .unwrap_or_else(|error| panic!("{}: temporary cleanup: {error}", case.name));
    }
}
