//! The SQL login-challenge contract on a file-backed SQLite database.
#![cfg(feature = "sqlite")]

mod challenge_sql_support;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct TemporaryDatabase(PathBuf);

impl Drop for TemporaryDatabase {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_challenge_contract() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let database = TemporaryDatabase(std::env::temp_dir().join(format!(
        "rullst-connect-challenges-{}-{nonce}.sqlite",
        std::process::id()
    )));
    let options = SqliteConnectOptions::new()
        .filename(&database.0)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(30));
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .unwrap();
    challenge_sql_support::exercise(pool.clone()).await;
    pool.close().await;
}
