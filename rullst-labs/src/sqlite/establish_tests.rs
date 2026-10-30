#![allow(clippy::unwrap_used, clippy::expect_used)]
//! A real bootstrap connection that outlives the pool wait is tried again.

use super::super::{ContentKey, SqliteLabs, StoreConfig, cause};
use crate::{ExecutionProfile, Reference, SystemClock};
use sqlx::{
    Connection, SqliteConnection,
    sqlite::{SqliteConnectOptions, SqliteLockingMode},
};
use std::{
    cell::Cell,
    path::Path,
    time::{Duration, Instant},
};

fn config() -> StoreConfig {
    StoreConfig::new(
        Reference::new("stalled").unwrap(),
        10,
        4,
        ExecutionProfile::Simulation,
    )
    .unwrap()
}

fn key() -> ContentKey {
    ContentKey::new([5; 32]).unwrap()
}

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

/// `open` establishes its first connection while another connection keeps
/// the file locked, so that establishment waits past the pool's 5-second
/// wait and times out. The lock is released only once the store has tried
/// the step again, so the first attempt never saw the file unlocked.
///
/// Establishing the connection waits for the lock in SQLite's busy handler,
/// whose 5-second timeout equals the pool wait and is spent in about 60
/// sleeps. Windows rounds each sleep up to the system timer tick, so the
/// handler gives up well after the pool wait has ended. On Linux the sleeps
/// overshoot by microseconds and the handler gives up only about 20 ms after
/// the pool wait, so a late runtime thread can see `SQLITE_BUSY` first, which
/// is not retried: the test runs on Windows only.
#[tokio::test]
#[cfg_attr(
    not(windows),
    ignore = "only Windows makes SQLite's lock wait outlast the pool wait"
)]
async fn open_tries_again_when_establishing_its_connection_times_out() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("labs.sqlite");
    SqliteLabs::initialize(&path, config(), key(), SystemClock)
        .await
        .unwrap()
        .close()
        .await;
    let holder = exclusive_lock(&path).await;
    let before = super::retried();
    let finished = Cell::new(false);
    let started = Instant::now();
    let opening = async {
        let opened = SqliteLabs::open(&path, config(), key(), SystemClock).await;
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
    let store = opened
        .unwrap_or_else(|error| panic!("open: {error:?} ({:?}) after {elapsed:?}", cause::last()));
    assert_eq!(super::retried() - before, 1, "after {elapsed:?}");
    assert!(elapsed >= Duration::from_secs(5), "after {elapsed:?}");
    store.close().await;
}
