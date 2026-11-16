//! Opening a broker whose file stays locked past an attempt's deadline is
//! tried again, on every platform.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::establish::retries;
use sqlx::{
    Connection, SqliteConnection,
    sqlite::{SqliteConnectOptions, SqliteLockingMode},
};
use std::{cell::Cell, path::PathBuf};

/// Attempts that a held lock outlives quickly. SQLite's busy handler waits
/// longer, so the deadline is what ends each attempt.
const SHORT: Waits = Waits {
    attempt: Duration::from_millis(300),
    lock: Duration::from_secs(5),
};

struct Fixture {
    path: PathBuf,
    url: String,
}

impl Fixture {
    async fn created() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rullst-sqlite-establish-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let url = format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"));
        let fixture = Self { path, url };
        fixture.connect(Waits::BROKER).await.unwrap().close().await;
        fixture
    }

    async fn connect(&self, waits: Waits) -> Result<SqliteBroker> {
        let config = BrokerConfig::try_new("locked").unwrap();
        let storage = StorageProfile::plaintext();
        SqliteBroker::connect_waiting(self.url.clone(), config, SystemClock, storage, waits).await
    }

    /// A connection in SQLite's exclusive locking mode that has taken the
    /// write lock, which that mode keeps until the connection closes: until
    /// then no other connection can even read the file.
    async fn exclusive_lock(&self) -> SqliteConnection {
        let options = SqliteConnectOptions::new()
            .filename(&self.path)
            .locking_mode(SqliteLockingMode::Exclusive);
        let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
        connection
            .begin_with("BEGIN IMMEDIATE")
            .await
            .unwrap()
            .commit()
            .await
            .unwrap();
        connection
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.path.display()));
        }
    }
}

/// The lock is released only once `connect` has tried again, so the first
/// attempt never saw the file unlocked.
#[tokio::test]
async fn connect_tries_again_when_an_attempt_outlives_its_deadline() {
    let fixture = Fixture::created().await;
    let holder = fixture.exclusive_lock().await;
    let before = retries();
    let finished = Cell::new(false);
    let connecting = async {
        // A second for each later attempt, in case a runner's disk stalls.
        let waits = Waits {
            attempt: Duration::from_secs(1),
            ..SHORT
        };
        let connected = fixture.connect(waits).await;
        finished.set(true);
        connected
    };
    let releasing = async {
        while retries() == before && !finished.get() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        holder.close().await.unwrap();
    };
    let (connected, ()) = tokio::join!(connecting, releasing);
    let broker = connected.unwrap_or_else(|error| panic!("connect: {error:?}"));
    assert!(retries() > before);
    assert_eq!(broker.config().namespace().as_str(), "locked");
    broker.close().await;
}

#[tokio::test]
async fn a_lock_held_past_every_attempt_is_a_storage_timeout() {
    let fixture = Fixture::created().await;
    let holder = fixture.exclusive_lock().await;
    let before = retries();
    let connected = fixture.connect(SHORT).await;
    holder.close().await.unwrap();
    assert_eq!(
        connected.err(),
        Some(MessagingError::StorageTimeout {
            operation: "connect"
        })
    );
    assert_eq!(retries() - before, 2);
}

#[tokio::test]
async fn other_failures_are_not_tried_again() {
    let fixture = Fixture::created().await;
    let before = retries();
    let other_limits = BrokerConfig::try_new("locked")
        .and_then(|config| config.with_limits(7, 1, 10, 1024))
        .unwrap();
    let refused = SqliteBroker::connect_waiting(
        fixture.url.clone(),
        other_limits,
        SystemClock,
        StorageProfile::plaintext(),
        SHORT,
    )
    .await;
    assert_eq!(refused.err(), Some(MessagingError::ConfigurationConflict));
    assert_eq!(retries(), before);
}
