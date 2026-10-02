#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::context::DotEnv;
use super::database::{Target, parse_target, pending_check, tcp_reachable};
use super::probe::{Probe, version_token};
use super::project::{requirement_major, version_check};
use super::report::Status;
use super::toolchain::{
    RULLST_MSRV, RustVersion, components_check, optional_check, parse_rustc_version, rustc_check,
    wasm_check,
};
use super::*;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::PathBuf;

fn ok(line: &str) -> Probe {
    Probe {
        found: true,
        success: true,
        first_line: line.to_string(),
        stdout: format!("{line}\n"),
    }
}

fn failed() -> Probe {
    Probe {
        found: true,
        ..Probe::default()
    }
}

fn healthy_probes() -> Probes {
    Probes {
        rustc: ok("rustc 1.98.1 (fixture)"),
        cargo: ok("cargo 1.98.1 (fixture)"),
        rustfmt: ok("rustfmt 1.8.0"),
        clippy: ok("clippy 0.1.98"),
        git: ok("git version 2.43.0"),
        cargo_audit: ok("cargo-audit 0.21.0"),
        optional: vec![Probe::default(); 6],
        targets: None,
    }
}

#[test]
fn rustc_version_parser_enforces_the_declared_msrv() {
    assert_eq!(
        parse_rustc_version("rustc 1.96.0 (abcdef 2026-01-01)"),
        Some(RULLST_MSRV)
    );
    assert_eq!(
        parse_rustc_version("rustc 1.97.1-nightly (abcdef 2026-01-01)"),
        Some(RustVersion(1, 97, 1))
    );
    assert!(parse_rustc_version("not-rustc").is_none());
    assert!(RustVersion(1, 95, 9) < RULLST_MSRV);
    assert_eq!(
        version_token("cargo 1.98.1 (abc 2026)").as_deref(),
        Some("1.98.1")
    );
    assert_eq!(version_token("Launching Omni interface..."), None);
}

#[test]
fn rust_compiler_states_map_to_pass_warn_and_fail() {
    assert_eq!(rustc_check(&Probe::default()).status, Status::Fail);
    assert_eq!(rustc_check(&failed()).status, Status::Fail);
    let outdated = rustc_check(&ok("rustc 1.80.0 (fixture)"));
    assert_eq!(outdated.status, Status::Fail);
    assert!(outdated.fix.unwrap().contains("rustup update stable"));
    assert_eq!(rustc_check(&ok("unexpected")).status, Status::Warn);
    let current = rustc_check(&ok("rustc 1.98.1 (fixture)"));
    assert_eq!(current.status, Status::Pass);
    assert_eq!(current.fix, None);
}

#[test]
fn components_wasm_and_optional_tools_are_classified() {
    let missing = components_check(&ok("rustfmt"), &failed());
    assert_eq!(missing.status, Status::Warn);
    assert_eq!(missing.detail, "missing: clippy");
    assert_eq!(components_check(&ok("a"), &ok("b")).status, Status::Pass);

    let installed = ok("wasm32-unknown-unknown\nx86_64-unknown-linux-gnu");
    let installed = Probe {
        stdout: "x86_64-unknown-linux-gnu\nwasm32-unknown-unknown\n".to_string(),
        ..installed
    };
    assert_eq!(wasm_check(&installed).status, Status::Pass);
    let absent = wasm_check(&ok("x86_64-unknown-linux-gnu"));
    assert_eq!(absent.status, Status::Warn);
    assert_eq!(
        absent.fix.as_deref(),
        Some("rustup target add wasm32-unknown-unknown")
    );

    let mut probes = vec![Probe::default(); 6];
    probes[5] = ok("Docker version 27");
    let optional = optional_check(&probes);
    assert_eq!(optional.status, Status::Info);
    assert_eq!(
        optional.detail,
        "installed: docker · not installed: cargo-deny, cargo-geiger, cargo-mutants, kani, cargo-llvm-cov"
    );
    assert!(optional.fix.unwrap().contains("cargo install cargo-deny"));
}

#[test]
fn project_versions_are_compared_by_major() {
    assert_eq!(requirement_major("13"), Some(13));
    assert_eq!(requirement_major("^13.0.0-alpha.1"), Some(13));
    assert_eq!(requirement_major(">=12, <14"), Some(12));
    assert_eq!(requirement_major("*"), None);
    assert_eq!(
        version_check(Some("13.0.0-alpha.1"), "13.0.0-alpha.1").status,
        Status::Pass
    );
    let old = version_check(Some("12"), "13.0.0");
    assert_eq!(old.status, Status::Warn);
    assert!(old.fix.unwrap().contains("cargo rullst upgrade"));
    assert_eq!(version_check(Some("14"), "13.0.0").status, Status::Warn);
    assert_eq!(version_check(Some("path"), "13.0.0").status, Status::Info);
    assert_eq!(version_check(None, "13.0.0").status, Status::Info);
}

#[test]
fn database_urls_reduce_to_targets_without_credentials() {
    assert_eq!(parse_target("sqlite::memory:"), Target::SqliteMemory);
    assert_eq!(
        parse_target("sqlite://data/app.db?mode=rwc"),
        Target::SqliteFile(PathBuf::from("data/app.db"))
    );
    assert_eq!(
        parse_target("sqlite:///var/lib/app.db"),
        Target::SqliteFile(PathBuf::from("/var/lib/app.db"))
    );
    assert_eq!(
        parse_target("postgres://user:hunter2@db.internal/app"),
        Target::Server {
            kind: "PostgreSQL",
            host: "db.internal".to_string(),
            port: 5432
        }
    );
    assert_eq!(
        parse_target("mysql://root:pw@[::1]:3307/app"),
        Target::Server {
            kind: "MySQL/MariaDB",
            host: "::1".to_string(),
            port: 3307
        }
    );
    assert_eq!(
        parse_target("redis://:secret@cache:6380/0"),
        Target::Server {
            kind: "Redis",
            host: "cache".to_string(),
            port: 6380
        }
    );
    for (url, reason) in [
        ("libsql://edge.turso.io", "remote endpoint"),
        ("mock_local", "mock development fallback"),
        (
            "postgres:///app?host=/run/postgresql",
            "socket or host parameter",
        ),
        ("postgres://h:notaport/app", "invalid port"),
        ("mongodb://x", "unsupported scheme"),
        ("nonsense", "unrecognized URL"),
    ] {
        assert_eq!(parse_target(url), Target::NotProbed(reason), "{url}");
    }
}

#[test]
fn tcp_reachability_reports_open_and_closed_ports() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("local address").port();
    assert_eq!(tcp_reachable("127.0.0.1", port), Ok(()));
    drop(listener);
    // Tests run in parallel, so another test may bind the released ephemeral
    // port before it is probed; a closed port must be seen at least once.
    let closed = (0..8).any(|_| {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let port = listener.local_addr().expect("local address").port();
        drop(listener);
        tcp_reachable("127.0.0.1", port).is_err()
    });
    assert!(closed, "every released loopback port was still reachable");
}

#[test]
fn pending_migrations_are_counted_from_the_applied_names() {
    let defined: BTreeSet<String> = ["m1_a", "m2_b", "m3_c"].map(str::to_string).into();
    let applied: BTreeSet<String> = ["m1_a"].map(str::to_string).into();
    let pending = pending_check(&defined, Ok(applied));
    assert_eq!(pending.status, Status::Warn);
    assert_eq!(pending.detail, "2 pending (first: m2_b)");
    assert_eq!(pending.fix.as_deref(), Some("cargo rullst db:migrate"));
    assert_eq!(
        pending_check(&defined, Ok(defined.clone())).status,
        Status::Pass
    );
    let fresh = pending_check(&defined, Err("no such table: migrations".to_string()));
    assert_eq!(fresh.detail, "no migration has been applied yet");
}

#[test]
fn disk_thresholds_fail_below_one_gib_and_warn_below_five() {
    let gib = 1024 * 1024 * 1024;
    assert_eq!(disk::classify(gib / 2, ".").status, Status::Fail);
    assert_eq!(disk::classify(2 * gib, ".").status, Status::Warn);
    let fine = disk::classify(40 * gib, "../..");
    assert_eq!(fine.status, Status::Pass);
    assert_eq!(fine.detail, "40.0 GiB available on the filesystem of ../..");
}

fn sample_project() -> (tempfile::TempDir, ProjectContext) {
    let directory = tempfile::tempdir().expect("temporary project");
    let root = directory.path();
    fs::create_dir_all(root.join("src/migrations")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"shop\"\nversion = \"0.1.0\"\n[dependencies]\nrullst = \"12\"\n",
    )
    .unwrap();
    for name in [
        "mod.rs",
        "m20260101000000_create_users.rs",
        "m20260201000000_create_items.rs",
    ] {
        fs::write(root.join("src/migrations").join(name), "").unwrap();
    }
    fs::write(
        root.join(".env"),
        "APP_KEY=short\nRULLST_ENV=sandbox\nDATABASE_URL=sqlite://app.db?mode=rwc&password=hunter2\n",
    )
    .unwrap();
    fs::write(
        root.join(".env.example"),
        "APP_KEY=REPLACE_WITH_YOUR_32_CHAR_RANDOM_KEY\nRULLST_ENV=development\nDATABASE_URL=\nQDRANT_URL=\n",
    )
    .unwrap();
    let project = ProjectContext::detect(root).expect("fixture project");
    (directory, project)
}

fn create_sqlite(path: &Path, applied: &[&str]) {
    use sqlx::ConnectOptions;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut connection = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .connect()
            .await
            .unwrap();
        sqlx::query("CREATE TABLE migrations (id INTEGER PRIMARY KEY, migration TEXT NOT NULL UNIQUE, batch INTEGER NOT NULL)")
            .execute(&mut connection)
            .await
            .unwrap();
        for name in applied {
            sqlx::query("INSERT INTO migrations (migration, batch) VALUES (?, 1)")
                .bind(*name)
                .execute(&mut connection)
                .await
                .unwrap();
        }
    });
}

#[test]
fn a_project_report_groups_real_findings_without_leaking_values() {
    let (directory, project) = sample_project();
    create_sqlite(
        &directory.path().join("app.db"),
        &["m20260101000000_create_users"],
    );
    let vars: HashMap<String, String> = HashMap::new();
    let report = collect(
        directory.path(),
        Some(&project),
        &healthy_probes(),
        &vars,
        FixOutcome::NotAttempted,
    );
    let ids: Vec<&str> = report.groups.iter().map(|group| group.id).collect();
    assert_eq!(
        ids,
        [
            "toolchain",
            "project",
            "config",
            "database",
            "migrations",
            "security",
            "disk"
        ]
    );
    let status = |id: &str| report.check(id).map(|check| check.status);
    assert_eq!(status("project.rullst_version"), Some(Status::Warn));
    assert_eq!(status("config.env_file"), Some(Status::Pass));
    assert_eq!(status("config.env_keys"), Some(Status::Warn));
    assert_eq!(
        report.check("config.env_keys").unwrap().detail,
        "missing: QDRANT_URL"
    );
    assert_eq!(status("config.environment"), Some(Status::Fail));
    assert_eq!(status("database.primary"), Some(Status::Pass));
    assert_eq!(status("migrations.files"), Some(Status::Pass));
    assert_eq!(
        report.check("migrations.applied").unwrap().detail,
        "1 pending (first: m20260201000000_create_items)"
    );
    assert_eq!(status("security.app_key"), Some(Status::Warn));
    assert_eq!(status("security.lockfile"), Some(Status::Warn));
    assert!(!report.ok, "an unknown RULLST_ENV fails the doctor");
    assert_eq!(report.summary.fail, 1);

    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("hunter2") && !json.contains("short\""));
    let text = render::render(&report, Style::PLAIN);
    assert!(!text.contains("hunter2"));
    assert!(text.contains("  ✗ Environment"));
    assert!(text.contains(
        "      docs  https://rullst.github.io/Rullst/book/cli_reference.html#doctor-config-and-env"
    ));
    assert!(text.contains("Fix the ✗ items above"));
}

#[test]
fn the_json_report_has_the_documented_shape() {
    let report = collect(
        Path::new("."),
        None,
        &healthy_probes(),
        &HashMap::<String, String>::new(),
        FixOutcome::Fixed,
    );
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["schema_version"], "rullst.cli-doctor.v1");
    assert_eq!(value["cli_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["ok"], true);
    for key in ["pass", "warn", "fail", "info"] {
        assert!(value["summary"][key].is_u64(), "{key}");
    }
    let groups = value["groups"].as_array().unwrap();
    assert_eq!(groups[0]["id"], "toolchain");
    assert_eq!(groups[0]["title"], "Toolchain");
    assert!(groups[0].get("anchor").is_none());
    let check = &groups[0]["checks"][0];
    for key in ["id", "title", "status", "detail", "fix", "docs"] {
        assert!(check.get(key).is_some(), "{key}");
    }
    assert_eq!(check["status"], "pass");
    assert_eq!(
        report.check("toolchain.components").unwrap().detail,
        "installed by --fix"
    );
    let outside = report.check("project.detected").unwrap();
    assert_eq!(outside.status, Status::Info);
}

#[test]
fn a_failed_fix_keeps_the_warning_with_a_manual_command() {
    let mut probes = healthy_probes();
    probes.clippy = failed();
    let report = collect(
        Path::new("."),
        None,
        &probes,
        &HashMap::<String, String>::new(),
        FixOutcome::Failed,
    );
    let check = report.check("toolchain.components").unwrap();
    assert_eq!(check.status, Status::Warn);
    assert!(check.detail.starts_with("--fix could not install them"));
    assert_eq!(
        check.fix.as_deref(),
        Some("Install them manually: rustup component add rustfmt clippy")
    );
    assert!(report.ok, "warnings do not fail the doctor");
}

#[test]
fn dotenv_parsing_distinguishes_missing_invalid_and_parsed() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(
        DotEnv::load(&directory.path().join(".env")),
        DotEnv::Missing
    );
    fs::write(directory.path().join(".env"), "A=1\n").unwrap();
    assert_eq!(
        DotEnv::load(&directory.path().join(".env")).get("A"),
        Some("1")
    );
}
