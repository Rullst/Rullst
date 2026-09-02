//! End-to-end compile proof for deterministic `cargo rullst new` profiles.
//!
//! The structural blueprint matrix proves every template variant parses. This
//! suite crosses the distinct primary-database, ORM, frontend, hot-reload,
//! API, AI, Redis and polyglot boundaries through the public CLI and compiles
//! the materialized applications against the current workspace.

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

        let workspace_lock = workspace.join("Cargo.lock");
        if workspace_lock.is_file() {
            fs::copy(workspace_lock, project.path.join("Cargo.lock"))
                .expect("copy reproducible workspace lockfile");
        }
        let checked = Command::new(env!("CARGO"))
            .current_dir(&project.path)
            .args(["check", "--offline", "--all-targets"])
            .env("CARGO_TARGET_DIR", &target_root)
            // DuckDB's bundled C++ build can otherwise exhaust small developer
            // machines when this matrix is run outside the larger CI runners.
            .env("CARGO_BUILD_JOBS", "1")
            .output()
            .unwrap_or_else(|error| panic!("{}: run generated cargo check: {error}", case.name));
        assert!(
            checked.status.success(),
            "{}: generated profile did not compile\n{}",
            case.name,
            output_text(&checked)
        );
    }
}
