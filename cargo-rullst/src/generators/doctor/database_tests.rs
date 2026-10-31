//! Database URL targets, reachability messages and SQLite/migration states.
use super::super::context::DotEnv;
use super::super::report::Status;
use super::*;
use std::collections::HashMap;

fn project(root: &Path, dotenv: &[(&str, &str)]) -> ProjectContext {
    ProjectContext {
        root: root.to_path_buf(),
        relative_root: None,
        name: "fixture".into(),
        manifest: String::new(),
        dotenv: DotEnv::Parsed(
            dotenv
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        ),
        example: DotEnv::Missing,
        rullst_toml: None,
    }
}

fn no_vars() -> HashMap<String, String> {
    HashMap::new()
}

/// A local port nothing listens on.
fn closed_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

#[test]
fn sqlite_paths_keep_their_spelling() {
    assert_eq!(
        parse_target("SQLITE://Data/App.db?mode=rwc"),
        Target::SqliteFile(PathBuf::from("Data/App.db"))
    );
}

// Not on Windows, which retries a refused loopback connection for longer
// than the probe waits and so reports a timeout.
#[cfg(not(windows))]
#[test]
fn a_refused_connection_is_named() {
    // A parallel test may bind the freed port before the probe connects, so
    // try a few fresh ports; a refusal must be reported by name.
    let refused = (0..5)
        .map(|_| tcp_reachable("127.0.0.1", closed_port()))
        .find(Result::is_err);
    assert_eq!(refused, Some(Err("connection refused".to_string())));
}

#[test]
fn the_primary_database_fails_and_redis_warns_when_unreachable() {
    let root = tempfile::tempdir().unwrap();
    let port = closed_port();
    let url = format!("postgres://user:secret@127.0.0.1:{port}/app");
    let redis = format!("redis://127.0.0.1:{port}");
    let context = project(
        root.path(),
        &[("DATABASE_URL", &url), ("REDIS_URL", &redis)],
    );
    assert_eq!(primary_check(&context, &no_vars()).status, Status::Fail);
    assert_eq!(
        redis_check(&context, &no_vars()).unwrap().status,
        Status::Warn
    );
}

#[test]
fn redis_urls_are_probed_reported_or_ignored() {
    let root = tempfile::tempdir().unwrap();
    let redis = |url: &str| redis_check(&project(root.path(), &[("REDIS_URL", url)]), &no_vars());
    assert!(redis("   ").is_none());
    assert!(redis_check(&project(root.path(), &[]), &no_vars()).is_none());
    assert_eq!(
        redis("mock_redis").unwrap().detail,
        "mock development fallback (not probed)"
    );
    assert_eq!(
        redis("sqlite://cache.db").unwrap().detail,
        "not a Redis URL (not probed)"
    );
}

#[test]
fn sqlite_files_directories_and_missing_paths_are_distinguished() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("data")).unwrap();
    let check =
        |url: &str| primary_check(&project(root.path(), &[("DATABASE_URL", url)]), &no_vars());
    assert_eq!(check("sqlite://data").status, Status::Fail);
    assert_eq!(check("sqlite://data/app.db").status, Status::Info);
    assert_eq!(check("sqlite://missing/app.db").status, Status::Fail);
    std::fs::write(root.path().join("data/app.db"), "").unwrap();
    assert_eq!(check("sqlite://data/app.db").status, Status::Pass);
    // An empty Turso URL is not a configured database.
    let turso = project(root.path(), &[("TURSO_DATABASE_URL", "  ")]);
    assert_eq!(
        primary_check(&turso, &no_vars()).detail,
        "no DATABASE_URL configured"
    );
}

#[test]
fn migration_states_name_their_cause() {
    let defined = BTreeSet::from(["m20260101_000000_create_users".to_string()]);
    assert_eq!(
        pending_check(&defined, Err("disk I/O error".to_string())).detail,
        "could not read the migrations table: disk I/O error"
    );
    let root = tempfile::tempdir().unwrap();
    let migrations = root.path().join("src/migrations");
    std::fs::create_dir_all(&migrations).unwrap();
    std::fs::write(migrations.join("m20260101_000000_create_users.rs"), "").unwrap();
    let context = project(root.path(), &[("DATABASE_URL", "sqlite://missing.db")]);
    let checks = migration_checks(&context, &no_vars());
    assert_eq!(checks[1].detail, "the SQLite database does not exist yet");
}

#[test]
fn sizes_stop_at_tebibytes() {
    assert_eq!(human_bytes(1 << 50), "1024.0 TiB");
    assert_eq!(human_bytes(1536), "1.5 KiB");
    assert_eq!(human_bytes(1023), "1023 B");
}
