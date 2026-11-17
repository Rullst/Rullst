#![allow(clippy::unwrap_used, clippy::expect_used)]
//! A lock held past a bootstrap wait is tried again, on every platform.
//!
//! `open` connects while another connection keeps the file under SQLite's
//! exclusive lock, with short waits so that each test takes about a second.
//! One wait is much shorter than the other, so which one runs out is decided
//! rather than raced: SQLite's busy handler (`SQLITE_BUSY`) or the pool's wait
//! (`PoolTimedOut`). The stores' waits are 5 seconds each, which made the
//! outcome depend on the platform's timers.

use super::{Waits, retried};
use sqlx::{
    Connection, SqliteConnection,
    sqlite::{SqliteConnectOptions, SqliteLockingMode},
};
use std::{cell::Cell, fmt::Debug, future::Future, path::Path, time::Duration};

/// SQLite's busy handler runs out first: `SQLITE_BUSY`.
const BUSY: Waits = Waits {
    pool: Duration::from_secs(10),
    lock: Duration::from_millis(200),
};
/// The pool's wait runs out first: `PoolTimedOut`. A second long enough for
/// the next attempt's connection while a stalled runner disk delays it.
const POOL: Waits = Waits {
    pool: Duration::from_secs(1),
    lock: Duration::from_secs(10),
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
/// again, so the first attempt never sees the file unlocked, and returns what
/// it opened.
async fn open_while_locked<T, E: Debug>(
    path: &Path,
    open: impl Future<Output = Result<T, E>>,
) -> T {
    let holder = exclusive_lock(path).await;
    let before = retried();
    let finished = Cell::new(false);
    let opening = async {
        let opened = open.await;
        finished.set(true);
        opened
    };
    let releasing = async {
        while retried() == before && !finished.get() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        holder.close().await.unwrap();
    };
    let (opened, ()) = tokio::join!(opening, releasing);
    let store = opened.unwrap_or_else(|error| panic!("open: {error:?}"));
    assert!(retried() > before);
    store
}

/// Runs `open` while `path` stays locked throughout, and returns its error
/// after checking that each of the three attempts ran out.
async fn open_locked_throughout<T, E>(path: &Path, open: impl Future<Output = Result<T, E>>) -> E {
    let holder = exclusive_lock(path).await;
    let before = retried();
    let opened = open.await;
    holder.close().await.unwrap();
    assert_eq!(retried() - before, 2);
    opened.err().expect("a locked file does not open")
}

#[cfg(feature = "consent-sqlite")]
mod consent {
    use super::*;
    use crate::consent::{ConsentError, SqliteConsentStore};

    async fn initialized(directory: &Path) -> std::path::PathBuf {
        let path = directory.join("consent.sqlite");
        SqliteConsentStore::initialize(&path, 8)
            .await
            .unwrap()
            .close()
            .await;
        path
    }

    #[tokio::test]
    async fn a_lock_wait_that_runs_out_is_tried_again() {
        let directory = tempfile::tempdir().unwrap();
        let path = initialized(directory.path()).await;
        let store = open_while_locked(&path, SqliteConsentStore::open_with(&path, 8, BUSY)).await;
        store.close().await;
    }

    #[tokio::test]
    async fn a_pool_wait_that_runs_out_is_tried_again() {
        let directory = tempfile::tempdir().unwrap();
        let path = initialized(directory.path()).await;
        let store = open_while_locked(&path, SqliteConsentStore::open_with(&path, 8, POOL)).await;
        store.close().await;
    }

    #[tokio::test]
    async fn a_lock_held_past_every_attempt_is_a_store_timeout() {
        let directory = tempfile::tempdir().unwrap();
        let path = initialized(directory.path()).await;
        let error =
            open_locked_throughout(&path, SqliteConsentStore::open_with(&path, 8, BUSY)).await;
        assert_eq!(error, ConsentError::StoreTimeout);
    }
}

#[cfg(feature = "sqlite")]
mod replay {
    use super::*;
    use crate::age_assurance::{AgeError, SqliteReplayStore};

    async fn created(directory: &Path) -> std::path::PathBuf {
        let path = directory.join("replay.sqlite");
        SqliteReplayStore::open(&path, 8)
            .await
            .unwrap()
            .close()
            .await;
        path
    }

    #[tokio::test]
    async fn a_lock_wait_that_runs_out_is_tried_again() {
        let directory = tempfile::tempdir().unwrap();
        let path = created(directory.path()).await;
        let store = open_while_locked(&path, SqliteReplayStore::open_with(&path, 8, BUSY)).await;
        store.close().await;
    }

    #[tokio::test]
    async fn a_pool_wait_that_runs_out_is_tried_again() {
        let directory = tempfile::tempdir().unwrap();
        let path = created(directory.path()).await;
        let store = open_while_locked(&path, SqliteReplayStore::open_with(&path, 8, POOL)).await;
        store.close().await;
    }

    #[tokio::test]
    async fn a_lock_held_past_every_attempt_is_a_store_timeout() {
        let directory = tempfile::tempdir().unwrap();
        let path = created(directory.path()).await;
        let error =
            open_locked_throughout(&path, SqliteReplayStore::open_with(&path, 8, BUSY)).await;
        assert_eq!(error, AgeError::StoreTimeout);
    }
}
