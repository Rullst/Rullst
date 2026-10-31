#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Opening an outbox is tried again when its deadline expires, and only then.

use super::super::{
    Result, WebhookConfig, WebhookDestination, WebhookError, WebhookOutbox, WebhookSigningKey,
};
use super::{retries, retry};
use crate::{MessagingKeyring, MessagingStorageKey, SystemClock};
use sqlx::{
    Connection, SqliteConnection,
    sqlite::{SqliteConnectOptions, SqliteLockingMode},
};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Times out until the tenth call, so an unbounded retry would succeed.
async fn timeouts(calls: &Cell<u32>) -> Result<u32> {
    calls.set(calls.get() + 1);
    if calls.get() < 10 {
        Err(WebhookError::Timeout)
    } else {
        Ok(calls.get())
    }
}

#[tokio::test]
async fn an_expired_opening_is_tried_three_times_in_total() {
    let calls = Cell::new(0);
    let before = retries();
    assert_eq!(retry(|| timeouts(&calls)).await, Err(WebhookError::Timeout));
    assert_eq!(calls.get(), 3);
    assert_eq!(retries() - before, 2);
}

#[tokio::test]
async fn the_third_attempt_may_succeed() {
    let calls = Cell::new(7);
    assert_eq!(retry(|| timeouts(&calls)).await, Ok(10));
    assert_eq!(calls.get(), 10);
}

#[tokio::test]
async fn other_failures_and_successes_are_not_retried() {
    let before = retries();
    for refusal in [WebhookError::Storage, WebhookError::Configuration] {
        let calls = Cell::new(0);
        let outcome = retry(|| {
            calls.set(calls.get() + 1);
            std::future::ready(Err::<(), _>(refusal.clone()))
        })
        .await;
        assert_eq!(outcome, Err(refusal));
        assert_eq!(calls.get(), 1);
    }
    let calls = Cell::new(0);
    let opened = retry(|| {
        calls.set(calls.get() + 1);
        std::future::ready(Ok(5))
    })
    .await;
    assert_eq!(opened, Ok(5));
    assert_eq!(calls.get(), 1);
    assert_eq!(retries(), before);
}

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

async fn open(url: &str) -> Result<WebhookOutbox> {
    let destination =
        WebhookDestination::approved_https("https://receiver.example/events").unwrap();
    let config = WebhookConfig::new("stalled", destination, 4).unwrap();
    let key = WebhookSigningKey::new("fixture", "mock_establish").unwrap();
    let storage = MessagingKeyring::new(MessagingStorageKey::try_new("storage", [71; 32]).unwrap());
    WebhookOutbox::open(url, config, key, storage, SystemClock).await
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
/// exclusive lock. The store waits up to 30 seconds for that lock, so the
/// first attempt outlives its 10-second deadline and ends with `Timeout`. The
/// lock is released only once `open` has tried again, so the first attempt
/// never saw the file unlocked. No race decides the outcome (10 seconds
/// against 30), but the test waits out a whole deadline, so it runs only on
/// Windows, whose hosted runners showed the stall; `--ignored` runs it
/// elsewhere. A later attempt may time out too when the runner's disk
/// stalls, so it asserts at least one retry.
#[tokio::test]
#[cfg_attr(
    not(windows),
    ignore = "waits out a 10-second deadline; the stall it covers is seen on Windows runners"
)]
async fn open_tries_again_when_its_deadline_expires_while_connecting() {
    let (path, url) = fixture();
    open(&url).await.unwrap().close().await;
    let holder = exclusive_lock(&path).await;
    let before = retries();
    let finished = Cell::new(false);
    let started = Instant::now();
    let opening = async {
        let opened = open(&url).await;
        finished.set(true);
        opened
    };
    let releasing = async {
        while retries() == before && !finished.get() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        holder.close().await.unwrap();
    };
    let (opened, ()) = tokio::join!(opening, releasing);
    let elapsed = started.elapsed();
    let outbox = opened.unwrap_or_else(|error| panic!("open: {error:?} after {elapsed:?}"));
    assert!(retries() > before, "after {elapsed:?}");
    assert!(elapsed >= Duration::from_secs(10), "after {elapsed:?}");
    outbox.close().await;
    cleanup(&path);
}
