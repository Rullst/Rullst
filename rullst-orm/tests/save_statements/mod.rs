//! Shared by the SQLite `save_*statements_test` binaries: the recorder, a
//! database with the tables a test names, and the assertions.
#![allow(dead_code)]

#[path = "../driver_contract/save_recorder.rs"]
pub mod save_recorder;

use std::future::Future;

use rullst_orm::Orm;
pub use save_recorder::{Recorded, Recorder};

/// Installs the process-wide recorder of the `sp_` tables (and the audit
/// table), then opens an in-memory SQLite database and runs `schema`.
///
/// SQLite runs statements on the connection's worker thread, which a
/// thread-local subscriber does not see, so each binary holds one test.
pub async fn sqlite(schema: &[&str]) -> Recorder {
    let recorder = Recorder::install_global(&["sp_", "rullst_audits"]);
    Orm::init_with_options("sqlite::memory:", 1, 10)
        .await
        .expect("initialize SQLite");
    for sql in schema {
        execute(sql).await;
    }
    recorder
}

pub async fn execute(sql: &str) {
    Orm::raw(sql)
        .execute()
        .await
        .unwrap_or_else(|error| panic!("{sql}: {error}"));
}

pub async fn stored_text(sql: &str, id: i32) -> Option<String> {
    let query = rullst_orm::_sqlx::query_as(rullst_orm::_sqlx::AssertSqlSafe(sql));
    let row: Option<(String,)> = query
        .bind(id)
        .fetch_optional(Orm::pool().expect("pool"))
        .await
        .expect("read stored value");
    row.map(|(value,)| value)
}

pub async fn row_count(table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    let query = rullst_orm::_sqlx::query_as(rullst_orm::_sqlx::AssertSqlSafe(sql));
    let (count,): (i64,) = query
        .fetch_one(Orm::pool().expect("pool"))
        .await
        .expect("count rows");
    count
}

/// Runs `future` and returns its output with what it sent.
pub async fn record<T>(recorder: &Recorder, future: impl Future<Output = T>) -> (T, Recorded) {
    recorder.take();
    let output = future.await;
    (output, recorder.take())
}

pub fn assert_sent(seen: &Recorded, begins: usize, statements: &[&str], case: &str) {
    assert_eq!(seen.begins, begins, "{case}: transactions opened");
    assert_eq!(seen.statements, statements, "{case}: statements");
}
