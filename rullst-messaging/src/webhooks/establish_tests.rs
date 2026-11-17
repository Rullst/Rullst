#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Opening an outbox whose file stays locked past its deadline is tried
//! again, on every platform.

use super::{
    Result, WebhookConfig, WebhookDestination, WebhookError, WebhookOutbox, WebhookSigningKey,
};
use crate::{MessagingKeyring, MessagingStorageKey, SystemClock, establish::retries};
use sqlx::{
    Connection, SqliteConnection,
    sqlite::{SqliteConnectOptions, SqliteLockingMode},
};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    time::Duration,
};

/// A deadline that the held lock outlives quickly; SQLite waits 30 seconds
/// for the lock, so the deadline always ends the attempt.
const SHORT: Duration = Duration::from_millis(300);

fn fixture() -> (PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "rullst-webhook-establish-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let url = format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"));
    (path, url)
}

fn cleanup(path: &Path) {
    for path in [
        path.to_path_buf(),
        path.with_extension("sqlite-wal"),
        path.with_extension("sqlite-shm"),
    ] {
        let _ = std::fs::remove_file(path);
    }
}

async fn open(url: &str, deadline: Duration) -> Result<WebhookOutbox> {
    let destination =
        WebhookDestination::approved_https("https://receiver.example/events").unwrap();
    let config = WebhookConfig::new("stalled", destination, 4).unwrap();
    let key = WebhookSigningKey::new("fixture", "mock_establish").unwrap();
    let storage = MessagingKeyring::new(MessagingStorageKey::try_new("storage", [71; 32]).unwrap());
    WebhookOutbox::open_within(url.to_owned(), config, key, storage, SystemClock, deadline).await
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

/// `open` connects while another connection keeps the file under SQLite's
/// exclusive lock, so the first attempt outlives its deadline and ends with
/// `Timeout`. The lock is released only once `open` has tried again, so the
/// first attempt never saw the file unlocked.
#[tokio::test]
async fn open_tries_again_when_its_deadline_expires_while_connecting() {
    let (path, url) = fixture();
    open(&url, super::DEADLINE).await.unwrap().close().await;
    let holder = exclusive_lock(&path).await;
    let before = retries();
    let finished = Cell::new(false);
    let opening = async {
        // A second for each later attempt, in case a runner's disk stalls.
        let opened = open(&url, Duration::from_secs(1)).await;
        finished.set(true);
        opened
    };
    let releasing = async {
        while retries() == before && !finished.get() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        holder.close().await.unwrap();
    };
    let (opened, ()) = tokio::join!(opening, releasing);
    let outbox = opened.unwrap_or_else(|error| panic!("open: {error:?}"));
    assert!(retries() > before);
    outbox.close().await;
    cleanup(&path);
}

/// A lock held past every attempt ends `open` with `Timeout`.
#[tokio::test]
async fn open_times_out_when_the_lock_outlives_every_attempt() {
    let (path, url) = fixture();
    open(&url, super::DEADLINE).await.unwrap().close().await;
    let holder = exclusive_lock(&path).await;
    let before = retries();
    let opened = open(&url, SHORT).await;
    holder.close().await.unwrap();
    assert_eq!(opened.err(), Some(WebhookError::Timeout));
    assert_eq!(retries() - before, 2);
    cleanup(&path);
}
