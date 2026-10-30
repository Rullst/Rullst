#![allow(clippy::unwrap_used, clippy::expect_used)]
//! A real bootstrap connection that outlives the pool wait is tried again.
//!
//! `open` establishes its first connection while another connection keeps
//! the file locked, so that establishment waits past the pool's 5-second
//! wait and times out. The lock is released only once the store has tried
//! the step again, so the first attempt never saw the file unlocked. The
//! lock wait ends with SQLite's 5-second busy timeout, which outlasts the
//! equal pool wait reliably only on Windows (see the Labs test of the same
//! name for the timing), so these tests run on Windows only.

use sqlx::{
    Connection, SqliteConnection,
    sqlite::{SqliteConnectOptions, SqliteLockingMode},
};
use std::{
    cell::Cell,
    fmt::Debug,
    future::Future,
    path::Path,
    time::{Duration, Instant},
};

/// A connection in SQLite's exclusive locking mode that has taken the write
/// lock, which that mode keeps until the connection closes: until then no
/// other connection can even read the file.
async fn exclusive_lock(path: &Path) -> SqliteConnection {
    let options = SqliteConnectOptions::new()
        .filename(path)
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

/// Runs `open` while `path` stays locked until the store has tried a step
/// again, and returns what it opened.
async fn open_while_locked<T, E: Debug>(
    path: &Path,
    open: impl Future<Output = Result<T, E>>,
) -> T {
    let holder = exclusive_lock(path).await;
    let before = super::retried();
    let finished = Cell::new(false);
    let started = Instant::now();
    let opening = async {
        let opened = open.await;
        finished.set(true);
        opened
    };
    let releasing = async {
        while super::retried() == before && !finished.get() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        holder.close().await.unwrap();
    };
    let (opened, ()) = tokio::join!(opening, releasing);
    let elapsed = started.elapsed();
    let store = opened.unwrap_or_else(|error| panic!("open: {error:?} after {elapsed:?}"));
    assert_eq!(super::retried() - before, 1, "after {elapsed:?}");
    assert!(elapsed >= Duration::from_secs(5), "after {elapsed:?}");
    store
}

#[cfg(feature = "consent-sqlite")]
#[tokio::test]
#[cfg_attr(
    not(windows),
    ignore = "only Windows makes SQLite's lock wait outlast the pool wait"
)]
async fn consent_open_tries_again_when_establishing_its_connection_times_out() {
    use crate::consent::SqliteConsentStore;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("consent.sqlite");
    SqliteConsentStore::initialize(&path, 8)
        .await
        .unwrap()
        .close()
        .await;
    let store = open_while_locked(&path, SqliteConsentStore::open(&path, 8)).await;
    store.close().await;
}

#[cfg(feature = "sqlite")]
#[tokio::test]
#[cfg_attr(
    not(windows),
    ignore = "only Windows makes SQLite's lock wait outlast the pool wait"
)]
async fn replay_open_tries_again_when_establishing_its_connection_times_out() {
    use crate::age_assurance::SqliteReplayStore;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    SqliteReplayStore::open(&path, 8)
        .await
        .unwrap()
        .close()
        .await;
    let store = open_while_locked(&path, SqliteReplayStore::open(&path, 8)).await;
    store.close().await;
}
