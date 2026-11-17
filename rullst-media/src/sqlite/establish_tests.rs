#![allow(clippy::unwrap_used, clippy::expect_used)]
//! A lock held past a bootstrap wait is tried again, on every platform.
//!
//! `open` connects while another connection keeps the file under SQLite's
//! exclusive lock, with short waits so that each test takes about a second.
//! One wait is much shorter than the other, so which one runs out is decided
//! rather than raced: SQLite's busy handler (`SQLITE_BUSY`) or the pool's wait
//! (`PoolTimedOut`). The store's waits are 5 seconds each, which made the
//! outcome depend on the platform's timers.

use super::super::{SqliteMedia, StoreConfig};
use super::{Waits, retried};
use crate::{LibraryId, MediaError, ProviderBinding, ProviderMode, Reference, SystemClock};
use sqlx::{
    Connection, SqliteConnection,
    sqlite::{SqliteConnectOptions, SqliteLockingMode},
};
use std::{cell::Cell, future::Future, path::Path, time::Duration};

/// The wait that is meant to run out.
const SHORT: Duration = Duration::from_millis(200);
/// A pool wait that tries the second attempt's connection while a stalled
/// runner disk delays it.
const POOL: Duration = Duration::from_secs(1);
/// The wait that is not meant to run out.
const LONG: Duration = Duration::from_secs(10);

fn config() -> StoreConfig {
    let binding = ProviderBinding {
        library: LibraryId::new(1).unwrap(),
        mode: ProviderMode::Offline,
        environment: Reference::new("locked").unwrap(),
    };
    StoreConfig::testing(binding, 4).unwrap()
}

async fn initialized(directory: &Path) -> std::path::PathBuf {
    let path = directory.join("media.sqlite");
    SqliteMedia::initialize(&path, config(), SystemClock)
        .await
        .unwrap()
        .close()
        .await;
    path
}

async fn open(path: &Path, waits: Waits) -> Result<SqliteMedia, MediaError> {
    SqliteMedia::open_with(path, config(), SystemClock, waits).await
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

/// Runs `opening` while `path` stays locked until the store has tried a step
/// again, so the first attempt never sees the file unlocked.
async fn locked_until_retried<T>(path: &Path, opening: impl Future<Output = T>) -> T {
    let holder = exclusive_lock(path).await;
    let before = retried();
    let finished = Cell::new(false);
    let opening = async {
        let opened = opening.await;
        finished.set(true);
        opened
    };
    let releasing = async {
        while retried() == before && !finished.get() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        holder.close().await.unwrap();
    };
    tokio::join!(opening, releasing).0
}

#[tokio::test]
async fn a_lock_wait_that_runs_out_is_tried_again() {
    let directory = tempfile::tempdir().unwrap();
    let path = initialized(directory.path()).await;
    let before = retried();
    let waits = Waits {
        pool: LONG,
        lock: SHORT,
    };
    let store = locked_until_retried(&path, open(&path, waits))
        .await
        .unwrap_or_else(|error| panic!("open: {error:?}"));
    assert!(retried() > before);
    store.close().await;
}

#[tokio::test]
async fn a_pool_wait_that_runs_out_is_tried_again() {
    let directory = tempfile::tempdir().unwrap();
    let path = initialized(directory.path()).await;
    let before = retried();
    let waits = Waits {
        pool: POOL,
        lock: LONG,
    };
    let store = locked_until_retried(&path, open(&path, waits))
        .await
        .unwrap_or_else(|error| panic!("open: {error:?}"));
    assert!(retried() > before);
    store.close().await;
}

#[tokio::test]
async fn a_lock_held_past_every_attempt_is_a_storage_timeout() {
    let directory = tempfile::tempdir().unwrap();
    let path = initialized(directory.path()).await;
    let holder = exclusive_lock(&path).await;
    let before = retried();
    let waits = Waits {
        pool: LONG,
        lock: SHORT,
    };
    let opened = open(&path, waits).await;
    holder.close().await.unwrap();
    // Not `Busy`, which a contended operation of an open store returns.
    assert_eq!(opened.err(), Some(MediaError::StorageTimeout));
    assert_eq!(retried() - before, 2);
}
