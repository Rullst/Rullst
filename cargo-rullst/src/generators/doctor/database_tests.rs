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
    // A parallel test may bind the freed port before a probe connects, which
    // then passes; try a few fresh ports until neither probe was answered.
    let statuses = (0..5)
        .map(|_| {
            let port = closed_port();
            let url = format!("postgres://user:secret@127.0.0.1:{port}/app");
            let redis = format!("redis://127.0.0.1:{port}");
            let context = project(
                root.path(),
                &[("DATABASE_URL", &url), ("REDIS_URL", &redis)],
            );
            (
                primary_check(&context, &no_vars()).status,
                redis_check(&context, &no_vars()).unwrap().status,
            )
        })
        .find(|&(primary, redis)| primary != Status::Pass && redis != Status::Pass);
    assert_eq!(statuses, Some((Status::Fail, Status::Warn)));
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
fn the_header_names_the_journal() {
    let mut header = *b"SQLite format 3\0\x10\x00\x02\x02";
    assert!(header_records_wal(&header));
    header[18] = 1;
    header[19] = 1;
    assert!(!header_records_wal(&header));
    assert!(!header_records_wal(&[2_u8; 20]), "not a SQLite file");
    assert!(!header_records_wal(b"SQLite format 3\0"), "a short file");
}

#[test]
fn network_filesystems_are_named() {
    for (magic, kind) in [
        (0x6969, "NFS"),
        (0x517B, "SMB"),
        (0xFF53_4D42, "SMB"),
        (0xFE53_4D42, "SMB"),
        (0x0102_1997, "9P"),
        (0x5346_414F, "AFS"),
        (0x00C3_6400, "Ceph"),
    ] {
        assert_eq!(network_filesystem_kind(magic), Some(kind), "{magic:#x}");
    }
    assert_eq!(network_filesystem_kind(0xEF53), None, "ext4");
}

#[test]
fn wal_on_a_network_filesystem_is_a_warning() {
    let file = |wal, wal_configured, network| SqliteFile {
        bytes: 2048,
        wal,
        wal_configured,
        network,
    };
    let local = file(true, true, None).check("app.db", Source::DotEnv);
    assert_eq!(local.status, Status::Pass);
    assert_eq!(
        local.detail,
        "SQLite file app.db (2.0 KiB, WAL journal) from .env"
    );
    let rollback = file(false, false, Some("NFS")).check("app.db", Source::DotEnv);
    assert_eq!(rollback.status, Status::Pass);
    assert_eq!(rollback.detail, "SQLite file app.db (2.0 KiB) from .env");
    for (wal, configured) in [(true, false), (false, true)] {
        let check = file(wal, configured, Some("NFS")).check("app.db", Source::RullstToml);
        assert_eq!(check.status, Status::Warn);
        assert!(
            check
                .detail
                .ends_with("; WAL does not work on this NFS mount"),
            "{}",
            check.detail
        );
        assert!(
            check
                .fix
                .is_some_and(|fix| fix.contains("sqlite_journal_mode = \"delete\""))
        );
    }
}

/// Under WAL a committed migration can sit in the `-wal` file; the doctor's
/// read-only connection still sees it, and counts that file's bytes.
#[test]
fn a_wal_database_is_read_with_its_uncheckpointed_commits() {
    use sqlx::{ConnectOptions, Connection};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("app.db");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let writer = runtime.block_on(async {
        let mut writer = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .connect()
            .await
            .unwrap();
        for statement in [
            "PRAGMA journal_mode = WAL",
            "PRAGMA wal_autocheckpoint = 0",
            "CREATE TABLE migrations (migration TEXT NOT NULL)",
            "INSERT INTO migrations VALUES ('m20260101_000000_create_users')",
        ] {
            sqlx::query(statement).execute(&mut writer).await.unwrap();
        }
        writer
    });
    assert_eq!(
        applied_sqlite(&path).unwrap(),
        BTreeSet::from(["m20260101_000000_create_users".to_string()])
    );

    let mut context = project(root.path(), &[("DATABASE_URL", "sqlite://app.db")]);
    let size = std::fs::metadata(&path).unwrap().len();
    let unconfigured = SqliteFile::inspect(&path, size, &context);
    assert!(unconfigured.wal && !unconfigured.wal_configured);
    assert!(unconfigured.bytes > size, "the -wal file is counted");
    assert_eq!(unconfigured.network, None);
    context.rullst_toml = Some("[database]\nsqlite_journal_mode = \"wal\"\n".into());
    assert!(SqliteFile::inspect(&path, size, &context).wal_configured);
    let check = primary_check(&context, &no_vars());
    assert_eq!(check.status, Status::Pass);
    assert!(
        check.detail.ends_with(", WAL journal) from .env"),
        "{}",
        check.detail
    );
    runtime.block_on(writer.close()).unwrap();
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
