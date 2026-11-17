#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Corrupt or tampered storage and clock movement around one transaction.

use super::*;
use crate::{LibraryId, ProviderBinding, ProviderMode, sqlite::StoreConfig};
use std::sync::{Arc, Mutex};

const T: i64 = 1_800_000_000;

/// Returns the scripted readings in order, then repeats the last one.
#[derive(Clone)]
struct Script(Arc<Mutex<Vec<i64>>>);
impl Script {
    fn set(&self, readings: &[i64]) {
        *self.0.lock().unwrap() = readings.to_vec();
    }
}
impl Clock for Script {
    fn now(&self) -> Result<i64, Error> {
        let mut readings = self.0.lock().unwrap();
        let reading = readings[0];
        if readings.len() > 1 {
            readings.remove(0);
        }
        Ok(reading)
    }
}

fn config(max_assets: u32) -> StoreConfig {
    let binding = ProviderBinding::new(
        LibraryId::new(7).unwrap(),
        ProviderMode::Offline,
        Reference::new("transaction").unwrap(),
    );
    StoreConfig::testing(binding, max_assets).unwrap()
}

async fn store(dir: &tempfile::TempDir, max_assets: u32) -> (SqliteMedia<Script>, Script) {
    let clock = Script(Arc::new(Mutex::new(vec![T])));
    let store = SqliteMedia::initialize(
        dir.path().join("media.sqlite"),
        config(max_assets),
        clock.clone(),
    )
    .await
    .unwrap();
    (store, clock)
}

async fn execute(store: &SqliteMedia<Script>, sql: &'static str) {
    sqlx::query(sql).execute(&store.pool).await.unwrap();
}

/// Stores `record()` with matching key, revision and video columns.
async fn insert(store: &SqliteMedia<Script>) -> Record {
    let record = super::super::record::tests::record();
    sqlx::query(
        "INSERT INTO media_assets (id,tenant,course,video,revision,body) VALUES (?,?,?,?,?,?)",
    )
    .bind(record.asset.id.as_str())
    .bind(record.asset.scope.tenant.as_str())
    .bind(record.asset.scope.course.as_str())
    .bind(record.asset.video.as_ref().map(|v| v.as_str()))
    .bind(record.asset.revision)
    .bind(serde_json::to_string(&record).unwrap())
    .execute(&store.pool)
    .await
    .unwrap();
    record
}

#[tokio::test]
async fn commit_checks_the_clock_before_and_after_it_commits() {
    let dir = tempfile::tempdir().unwrap();
    let (store, clock) = store(&dir, 4).await;
    // begin reads the clock twice and commit twice: before and after COMMIT.
    clock.set(&[T + 1, T + 1, T + 2, T + 2]);
    Operation::begin(&store)
        .await
        .unwrap()
        .commit()
        .await
        .unwrap();
    clock.set(&[T + 3, T + 3, T + 2]);
    let tx = Operation::begin(&store).await.unwrap();
    assert_eq!(tx.commit().await.err(), Some(Error::Clock));
    clock.set(&[T + 4, T + 4, T + 5, T + 4]);
    let tx = Operation::begin(&store).await.unwrap();
    assert_eq!(tx.commit().await.err(), Some(Error::Uncertain));
    clock.set(&[T + 6, T + 6, T + 6, T + 7]);
    Operation::begin(&store)
        .await
        .unwrap()
        .commit()
        .await
        .unwrap();
    // A commit that ends at the deadline is not known to have been in time.
    clock.set(&[T + 8, T + 8, T + 9, T + 10]);
    let mut tx = Operation::begin(&store).await.unwrap();
    tx.until(Some(T + 10)).unwrap();
    assert_eq!(tx.commit().await.err(), Some(Error::Uncertain));
}

#[tokio::test]
async fn begin_refuses_tampered_metadata_and_asset_counts() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _clock) = store(&dir, 1).await;
    let mut conn = store.pool.acquire().await.unwrap();
    for sql in [
        "PRAGMA ignore_check_constraints=ON",
        "UPDATE media_meta SET last_now=0",
        "PRAGMA ignore_check_constraints=OFF",
    ] {
        sqlx::query(sql).execute(&mut *conn).await.unwrap();
    }
    drop(conn);
    assert_eq!(
        Operation::begin(&store).await.err(),
        Some(Error::Configuration)
    );
    execute(&store, "UPDATE media_meta SET last_now=1").await;
    Operation::begin(&store)
        .await
        .unwrap()
        .commit()
        .await
        .unwrap();
    // One row more than the store holds, whatever the rows contain.
    execute(
        &store,
        "INSERT INTO media_assets (id,tenant,course,video,revision,body) \
         VALUES ('a','t','c',NULL,1,'{}'),('b','t','c',NULL,1,'{}')",
    )
    .await;
    assert_eq!(
        Operation::begin(&store).await.err(),
        Some(Error::Configuration)
    );
}

#[tokio::test]
async fn get_refuses_a_video_column_that_disagrees_with_the_body() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _clock) = store(&dir, 4).await;
    let record = insert(&store).await;
    let (scope, id) = (&record.asset.scope, &record.asset.id);
    let mut tx = Operation::begin(&store).await.unwrap();
    assert_eq!(tx.get(scope, id).await.unwrap().asset, record.asset);
    tx.commit().await.unwrap();
    execute(&store, "UPDATE media_assets SET video=NULL").await;
    let mut tx = Operation::begin(&store).await.unwrap();
    assert_eq!(tx.get(scope, id).await.err(), Some(Error::Configuration));
}

#[test]
fn decode_accepts_a_stored_body_of_exactly_32_kib() {
    let body = serde_json::to_string(&super::super::record::tests::record()).unwrap();
    let padded = format!("{body}{}", " ".repeat(32768 - body.len()));
    assert_eq!(padded.len(), 32768);
    let decoded = Operation::<'_, Script>::decode(&padded).unwrap();
    assert_eq!(decoded.asset.revision, 5);
    assert_eq!(
        Operation::<'_, Script>::decode(&format!("{padded} ")).err(),
        Some(Error::Configuration)
    );
}

#[tokio::test]
async fn open_refuses_a_schema_object_with_the_right_name_and_other_sql() {
    let dir = tempfile::tempdir().unwrap();
    let (store, clock) = store(&dir, 4).await;
    let mut conn = store.pool.acquire().await.unwrap();
    for sql in [
        "DROP INDEX media_scope",
        "CREATE INDEX media_scope ON media_assets(tenant,course)",
    ] {
        sqlx::query(sql).execute(&mut *conn).await.unwrap();
    }
    drop(conn);
    store.close().await;
    let opened = SqliteMedia::open(dir.path().join("media.sqlite"), config(4), clock).await;
    assert_eq!(opened.err(), Some(Error::Configuration));
}

#[tokio::test]
async fn a_closed_store_refuses_new_transactions() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _clock) = store(&dir, 4).await;
    Operation::begin(&store)
        .await
        .unwrap()
        .commit()
        .await
        .unwrap();
    store.close().await;
    assert_eq!(Operation::begin(&store).await.err(), Some(Error::Storage));
}

#[test]
fn pool_timeouts_are_contention() {
    assert_eq!(storage(sqlx::Error::PoolTimedOut), Error::Busy);
    assert_eq!(storage(sqlx::Error::PoolClosed), Error::Storage);
}
