//! End-to-end compile proof for deterministic `cargo rullst new` profiles.
//!
//! The structural blueprint matrix proves every template variant parses. This
//! suite crosses the distinct primary-database, ORM, frontend, hot-reload,
//! API, AI, Redis and polyglot boundaries through the public CLI. Ordinary
//! profiles execute their test targets and hot-reload routers; the additive
//! polyglot profile compiles here and relies on each ORM adapter's runtime
//! matrix, avoiding redundant generated-test linking and adapter execution.

#![allow(clippy::expect_used, clippy::panic)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct GeneratedProject {
    path: PathBuf,
}

impl GeneratedProject {
    fn new(case: &str) -> Self {
        Self {
            path: std::env::temp_dir().join(format!(
                "rullst-generated-cli-{case}-{}",
                rand::random::<u64>()
            )),
        }
    }
}

impl Drop for GeneratedProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Clone, Copy)]
struct ProfileCase {
    name: &'static str,
    arguments: &'static [&'static str],
    required_rullst_features: &'static [&'static str],
    rejected_rullst_features: &'static [&'static str],
    router_returns_result: Option<bool>,
    run_tests: bool,
}

const PROFILE_CASES: [ProfileCase; 5] = [
    ProfileCase {
        name: "blank-no-database-api-hot",
        arguments: &[
            "--blueprint",
            "blank",
            "--api",
            "--no-database",
            "--hot-reload",
        ],
        required_rullst_features: &["studio"],
        rejected_rullst_features: &["orm", "strict-sqlite", "strict-postgres", "strict-mysql"],
        router_returns_result: Some(false),
        run_tests: true,
    },
    ProfileCase {
        name: "blog-postgres-repository-tera",
        arguments: &[
            "--blueprint",
            "blog",
            "--database",
            "postgres",
            "--orm",
            "repository",
            "--frontend",
            "tera",
        ],
        required_rullst_features: &["orm", "strict-postgres", "studio", "nexus"],
        rejected_rullst_features: &["strict-sqlite", "strict-mysql"],
        router_returns_result: None,
        run_tests: true,
    },
    ProfileCase {
        name: "portfolio-mysql-hybrid-pico",
        arguments: &[
            "--blueprint",
            "portfolio",
            "--database",
            "mysql",
            "--orm",
            "hybrid",
            "--frontend",
            "pico",
        ],
        required_rullst_features: &["orm", "strict-mysql", "studio", "nexus"],
        rejected_rullst_features: &["strict-sqlite", "strict-postgres"],
        router_returns_result: None,
        run_tests: true,
    },
    ProfileCase {
        name: "erp-mariadb-liveview-ai-redis",
        arguments: &[
            "--blueprint",
            "erp",
            "--database",
            "mariadb",
            "--frontend",
            "liveview",
            "--ai",
            "--redis",
            "--hot-reload",
        ],
        required_rullst_features: &["orm", "strict-mysql", "studio", "nexus", "ai", "redis"],
        rejected_rullst_features: &["strict-sqlite", "strict-postgres"],
        router_returns_result: Some(true),
        run_tests: true,
    },
    ProfileCase {
        name: "blank-sqlite-polyglot",
        arguments: &[
            "--blueprint",
            "blank",
            "--database",
            "sqlite",
            "--mongodb",
            "--duckdb",
            "--surrealdb",
            "--turso",
            "--qdrant",
        ],
        required_rullst_features: &[
            "orm",
            "strict-sqlite",
            "orm-mongodb",
            "orm-duckdb",
            "orm-surrealdb",
            "orm-turso",
            "orm-qdrant",
        ],
        rejected_rullst_features: &["strict-postgres", "strict-mysql"],
        router_returns_result: None,
        // Runtime behavior for these native adapters is exercised in the ORM
        // matrices. Rebuilding bundled DuckDB inside this public-CLI proof can
        // consume several extra GiB without adding a distinct runtime claim.
        run_tests: false,
    },
];

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn rullst_features(project: &Path) -> Vec<String> {
    let manifest = fs::read_to_string(project.join("Cargo.toml")).expect("generated manifest");
    let parsed: toml::Value = toml::from_str(&manifest).expect("valid generated manifest");
    assert_eq!(
        parsed["dependencies"]["rullst"]["default-features"].as_bool(),
        Some(false),
        "generated profile must not inherit the umbrella database defaults"
    );
    parsed["dependencies"]["rullst"]["features"]
        .as_array()
        .expect("rullst feature list")
        .iter()
        .filter_map(toml::Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn add_router_runtime_contract(project: &Path, returns_result: bool) {
    let manifest = fs::read_to_string(project.join("Cargo.toml")).expect("generated manifest");
    let parsed: toml::Value = toml::from_str(&manifest).expect("valid generated manifest");
    let package = parsed["package"]["name"]
        .as_str()
        .expect("generated package name");
    let module = package.replace('-', "_");
    let assertion = if returns_result {
        format!(
            "let result = {module}::router();\n    assert!(result.is_ok(), \"generated router must use offline-safe defaults\");"
        )
    } else {
        format!("let _router = {module}::router();")
    };
    fs::create_dir_all(project.join("tests")).expect("generated tests directory");
    fs::write(
        project.join("tests/generated_router_contract.rs"),
        format!("#[test]\nfn generated_router_constructs() {{\n    {assertion}\n}}\n"),
    )
    .expect("generated router runtime contract");
}

#[test]
fn public_cli_profiles_compile_across_every_distinct_generation_axis() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("target"));

    for case in PROFILE_CASES {
        let project = GeneratedProject::new(case.name);
        let generated = Command::new(env!("CARGO_BIN_EXE_rullst"))
            .current_dir(workspace)
            .arg("new")
            .arg(&project.path)
            .arg("--default")
            .args(case.arguments)
            .arg("--skip-initial-migration")
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .output()
            .unwrap_or_else(|error| panic!("{}: run project generator: {error}", case.name));
        assert!(
            generated.status.success(),
            "{}: project generation failed\n{}",
            case.name,
            output_text(&generated)
        );

        let features = rullst_features(&project.path);
        for required in case.required_rullst_features {
            assert!(
                features.iter().any(|feature| feature == required),
                "{}: missing rullst feature {required}: {features:?}",
                case.name
            );
        }
        for rejected in case.rejected_rullst_features {
            assert!(
                features.iter().all(|feature| feature != rejected),
                "{}: conflicting rullst feature {rejected}: {features:?}",
                case.name
            );
        }
        if let Some(returns_result) = case.router_returns_result {
            add_router_runtime_contract(&project.path, returns_result);
        }

        let workspace_lock = workspace.join("Cargo.lock");
        if workspace_lock.is_file() {
            fs::copy(workspace_lock, project.path.join("Cargo.lock"))
                .expect("copy reproducible workspace lockfile");
        }
        let cargo_operation = if case.run_tests { "test" } else { "check" };
        let mut cargo = Command::new(env!("CARGO"));
        cargo
            .current_dir(&project.path)
            .args([cargo_operation, "--offline", "--all-targets"])
            .env("CARGO_TARGET_DIR", &target_root)
            // DuckDB's bundled C++ build can otherwise exhaust small developer
            // machines when this matrix is run outside the larger CI runners.
            .env("CARGO_BUILD_JOBS", "1");
        if case.run_tests {
            cargo
                .env("CARGO_PROFILE_DEV_DEBUG", "0")
                .env("CARGO_PROFILE_TEST_DEBUG", "0")
                .env("CARGO_PROFILE_TEST_INCREMENTAL", "false");
        }
        let tested = cargo.output().unwrap_or_else(|error| {
            panic!(
                "{}: run generated cargo {cargo_operation}: {error}",
                case.name
            )
        });
        assert!(
            tested.status.success(),
            "{}: generated profile cargo {cargo_operation} failed\n{}",
            case.name,
            output_text(&tested)
        );
    }
}
