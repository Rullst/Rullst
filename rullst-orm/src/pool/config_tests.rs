use super::{OrmConfig, SqliteJournalMode, confirm_journal_mode, journal_mode_for};
use SqliteJournalMode::{Delete, Wal};

#[test]
fn journal_modes_name_their_pragma() {
    assert_eq!(Delete.as_str(), "delete");
    assert_eq!(Wal.as_str(), "wal");
    assert_eq!(Delete.pragma(), "PRAGMA journal_mode = DELETE");
    assert_eq!(Wal.pragma(), "PRAGMA journal_mode = WAL");
}

#[test]
fn rullst_toml_values_name_the_modes() {
    let parse = |value: &str| serde_json::from_str::<SqliteJournalMode>(value);
    assert_eq!(parse("\"wal\"").expect("wal"), Wal);
    assert_eq!(parse("\"delete\"").expect("delete"), Delete);
    for unknown in ["\"WAL\"", "\"truncate\"", "\"\""] {
        assert!(parse(unknown).is_err(), "{unknown}");
    }
}

#[test]
fn the_default_config_chooses_no_journal_mode() {
    assert_eq!(OrmConfig::default().sqlite_journal_mode(), None);
    let wal = OrmConfig::default().with_sqlite_journal_mode(Wal);
    assert_eq!(wal.sqlite_journal_mode(), Some(Wal));
    assert_eq!(
        wal.with_sqlite_journal_mode(Delete).sqlite_journal_mode(),
        Some(Delete)
    );
}

#[test]
fn only_sqlite_files_get_a_journal_mode() {
    let wal = OrmConfig::default().with_sqlite_journal_mode(Wal);
    for url in [
        "sqlite://db.sqlite?mode=rwc",
        "sqlite:data/app.db",
        "sqlite:///app/data/rullst.db?mode=rwc",
    ] {
        assert_eq!(journal_mode_for(url, &wal), Some(Wal), "{url}");
        assert_eq!(journal_mode_for(url, &OrmConfig::default()), None, "{url}");
    }
    for url in [
        "sqlite::memory:",
        "sqlite://:memory:",
        "sqlite:file:shared?mode=memory&cache=shared",
        "postgres://db/app",
        "postgresql://db/app",
        "mysql://db/app",
        "mariadb://db/app",
    ] {
        assert_eq!(journal_mode_for(url, &wal), None, "{url}");
    }
}

#[test]
fn a_journal_mode_sqlite_did_not_take_is_an_error() {
    assert!(confirm_journal_mode(Wal, "wal").is_ok());
    assert!(confirm_journal_mode(Wal, "WAL").is_ok());
    assert!(confirm_journal_mode(Delete, "delete").is_ok());
    for (requested, reported) in [
        (Wal, "delete"),
        (Wal, "memory"),
        (Delete, "wal"),
        (Delete, "truncate"),
    ] {
        let error = confirm_journal_mode(requested, reported).expect_err(reported);
        assert!(
            matches!(&error, crate::Error::DatabaseError(message)
                if message.contains(&format!("kept the {reported} journal"))
                    && message.contains("sqlite_journal_mode = \"delete\"")),
            "{error}"
        );
    }
}

/// Tests against SQLite itself, on a `SqliteConnection` so that they also run
/// when the pool type is PostgreSQL's (`--all-features`).
#[cfg(any(feature = "drivers-all", feature = "strict-sqlite", feature = "turso"))]
mod sqlite {
    use super::super::set_journal_mode;
    use super::{Delete, Wal};
    use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};
    use sqlx::{ConnectOptions, Connection};
    use std::path::{Path, PathBuf};

    /// A database path in a directory of its own, removed by `remove`.
    fn database(label: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "rullst-orm-journal-{label}-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).expect("temporary directory");
        directory.join("app.db")
    }

    fn remove(database: &Path) {
        if let Some(directory) = database.parent() {
            let _ = std::fs::remove_dir_all(directory);
        }
    }

    fn sidecar(database: &Path, suffix: &str) -> PathBuf {
        let mut name = database.as_os_str().to_owned();
        name.push(suffix);
        PathBuf::from(name)
    }

    async fn open(path: &Path) -> SqliteConnection {
        SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .connect()
            .await
            .expect("open the database")
    }

    async fn scalar<T>(connection: &mut SqliteConnection, sql: &'static str) -> T
    where
        T: Send + Unpin + for<'r> sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
    {
        sqlx::query_scalar(sql)
            .fetch_one(connection)
            .await
            .expect(sql)
    }

    #[tokio::test]
    async fn wal_is_recorded_in_the_file_and_keeps_full_synchronous() {
        let path = database("wal");
        let mut first = open(&path).await;
        let journal: String = scalar(&mut first, "PRAGMA journal_mode").await;
        assert_eq!(
            journal, "delete",
            "a new file starts with the rollback journal"
        );

        // A read-only connection cannot record another mode in the file.
        let mut read_only = SqliteConnectOptions::new()
            .filename(&path)
            .read_only(true)
            .connect()
            .await
            .expect("read-only connection");
        let error = set_journal_mode(&mut read_only, Wal)
            .await
            .expect_err("a read-only connection cannot switch to WAL");
        assert!(matches!(&error, crate::Error::DatabaseError(_)), "{error}");
        read_only.close().await.expect("close");

        set_journal_mode(&mut first, Wal)
            .await
            .expect("switch to WAL");
        let mut second = open(&path).await;
        let journal: String = scalar(&mut second, "PRAGMA journal_mode").await;
        assert_eq!(journal, "wal", "the file records WAL for every connection");
        // Durability is unchanged: FULL syncs the WAL at every commit.
        let synchronous: i64 = scalar(&mut second, "PRAGMA synchronous").await;
        assert_eq!(synchronous, 2);
        let busy_timeout: i64 = scalar(&mut second, "PRAGMA busy_timeout").await;
        assert_eq!(busy_timeout, 5_000);

        first.close().await.expect("close");
        set_journal_mode(&mut second, Delete)
            .await
            .expect("leave WAL");
        let journal: String = scalar(&mut second, "PRAGMA journal_mode").await;
        assert_eq!(journal, "delete");
        assert!(
            !sidecar(&path, "-wal").exists(),
            "leaving WAL removes the log"
        );
        second.close().await.expect("close");
        remove(&path);
    }

    #[tokio::test]
    async fn an_in_memory_database_cannot_take_wal() {
        let mut memory = SqliteConnection::connect("sqlite::memory:")
            .await
            .expect("in-memory database");
        let error = set_journal_mode(&mut memory, Wal)
            .await
            .expect_err("an in-memory database has no file to share");
        assert!(
            error.to_string().contains("kept the memory journal"),
            "{error}"
        );
    }

    /// Under WAL a committed transaction can live only in the `-wal` file:
    /// copying the database file alone loses it, `VACUUM INTO` (like
    /// `sqlite3 ".backup"`) does not.
    #[tokio::test]
    async fn a_wal_backup_includes_commits_not_yet_checkpointed() {
        let path = database("backup");
        let mut connection = open(&path).await;
        set_journal_mode(&mut connection, Wal).await.expect("WAL");
        for statement in [
            "PRAGMA wal_autocheckpoint = 0",
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, email TEXT NOT NULL)",
            "INSERT INTO accounts (email) VALUES ('a@example.test'), ('b@example.test')",
        ] {
            sqlx::query(statement)
                .execute(&mut connection)
                .await
                .expect(statement);
        }
        assert!(sidecar(&path, "-wal").metadata().expect("-wal").len() > 0);

        let copied = path.with_file_name("copied.db");
        std::fs::copy(&path, &copied).expect("copy the database file alone");
        let mut copy = open(&copied).await;
        let error = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM accounts")
            .fetch_one(&mut copy)
            .await
            .expect_err("the copied file lacks the commits in the -wal file");
        assert!(error.to_string().contains("no such table"), "{error}");
        copy.close().await.expect("close");

        let backup = path.with_file_name("backup.db");
        sqlx::query("VACUUM INTO ?")
            .bind(backup.to_string_lossy().into_owned())
            .execute(&mut connection)
            .await
            .expect("VACUUM INTO");
        let mut restored = open(&backup).await;
        let accounts: i64 = scalar(&mut restored, "SELECT COUNT(*) FROM accounts").await;
        assert_eq!(accounts, 2);
        restored.close().await.expect("close");
        connection.close().await.expect("close");
        remove(&path);
    }
}

/// The application pool: without a choice the file keeps its journal mode,
/// so existing applications are unchanged.
#[cfg(all(
    any(feature = "drivers-all", feature = "strict-sqlite"),
    not(any(feature = "strict-postgres", feature = "strict-mysql"))
))]
#[tokio::test]
async fn the_pool_sets_only_the_journal_mode_its_config_names() {
    use crate::pool::Orm;
    #[cfg(not(feature = "strict-sqlite"))]
    sqlx::any::install_default_drivers();
    let directory = std::env::temp_dir().join(format!(
        "rullst-orm-journal-pool-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    std::fs::create_dir_all(&directory).expect("temporary directory");
    let url = format!("sqlite://{}/app.db?mode=rwc", directory.display());
    let wal = OrmConfig::default().with_sqlite_journal_mode(Wal);
    let delete = OrmConfig::default().with_sqlite_journal_mode(Delete);
    let mut journals = Vec::new();
    for config in [&OrmConfig::default(), &wal, &OrmConfig::default(), &delete] {
        let pool = Orm::connect_primary(&url, Orm::pool_options(), config)
            .await
            .expect("connect");
        let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .expect("journal mode");
        journals.push(journal);
        pool.close().await;
    }
    // A pool without a choice keeps the file's mode, the rollback journal of
    // a new file or the WAL an earlier pool recorded.
    assert_eq!(journals, ["delete", "wal", "wal", "delete"]);
    let _ = std::fs::remove_dir_all(directory);
}
