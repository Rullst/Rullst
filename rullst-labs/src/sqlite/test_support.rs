#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Shared fixtures for the job-plane unit tests.

use super::{ContentKey, SqliteLabs, StoreConfig, transaction::Operation};
use crate::{
    Action, Authorization, Clock, ExecutionLimits, ExecutionProfile, Exercise, ExerciseRef,
    GraderCase, LabError, Permission, Reference, RustSource, Scope, Submission,
};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};

pub(super) const NOW: i64 = 1_800_000_000;

#[derive(Clone)]
pub(super) struct Fixed(pub Arc<AtomicI64>);
impl Clock for Fixed {
    fn now(&self) -> Result<i64, LabError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

pub(super) struct Allow;
impl Authorization for Allow {
    async fn check(&self, _: &Reference, _: &Scope, _: Action) -> Result<Permission, LabError> {
        Permission::until(NOW + 100_000)
    }
}

pub(super) fn reference(value: &str) -> Reference {
    Reference::new(value).unwrap()
}

pub(super) fn scope() -> Scope {
    Scope::new("school", "rust").unwrap()
}

pub(super) fn exercise(id: &str, revision: &str, expected: i64) -> Exercise {
    Exercise::new(
        scope(),
        reference(id),
        reference(revision),
        vec![GraderCase {
            id: reference("private-case"),
            input: [123, 456],
            expected,
        }],
        ExecutionLimits::new(10, 100_000, 64).unwrap(),
    )
    .unwrap()
}

pub(super) fn submission(name: &str) -> Submission {
    Submission::new(
        reference(name),
        ExerciseRef::new("sum", "v1").unwrap(),
        RustSource::new("pub fn solve(a: i64, b: i64) -> i64 { a + b }").unwrap(),
        300,
    )
    .unwrap()
}

/// A fresh store in `directory` under `namespace`, with `sum`/`v1` registered.
pub(super) async fn store(
    directory: &tempfile::TempDir,
    namespace: &str,
) -> (SqliteLabs<Fixed>, PathBuf) {
    let path = directory.path().join(format!("{namespace}.sqlite"));
    let store = SqliteLabs::initialize(
        &path,
        StoreConfig::new(reference(namespace), 10, 4, ExecutionProfile::Simulation).unwrap(),
        ContentKey::new([9; 32]).unwrap(),
        Fixed(Arc::new(AtomicI64::new(NOW))),
    )
    .await
    .unwrap();
    store
        .register_exercise(&Allow, &reference("teacher"), &exercise("sum", "v1", 579))
        .await
        .unwrap();
    (store, path)
}

pub(super) async fn submit(store: &SqliteLabs<Fixed>, name: &str) {
    store
        .submit(&Allow, &reference("alice"), &scope(), submission(name))
        .await
        .unwrap();
}

async fn connect(path: &Path) -> SqliteConnection {
    SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(path))
        .await
        .unwrap()
}

/// Runs `sql`, binding `blob` when given, as an external writer could.
pub(super) async fn write(path: &Path, sql: &str, blob: Option<&[u8]>) {
    let mut connection = connect(path).await;
    let query = sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()));
    let query = match blob {
        Some(blob) => query.bind(blob.to_vec()),
        None => query,
    };
    query.execute(&mut connection).await.unwrap();
    connection.close().await.unwrap();
}

pub(super) async fn read_blob(path: &Path, sql: &str) -> Vec<u8> {
    let mut connection = connect(path).await;
    let blob: Vec<u8> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_owned()))
        .fetch_one(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();
    blob
}

/// Loads `id` in its own transaction, as every job-plane operation does.
pub(super) async fn load(
    store: &SqliteLabs<Fixed>,
    id: &str,
) -> Result<(super::record::Record, Option<Vec<u8>>), LabError> {
    let mut tx = Operation::begin(store, None).await?;
    let loaded = store.load_job(&mut tx, &scope(), &reference(id)).await?;
    tx.commit().await?;
    Ok(loaded)
}
