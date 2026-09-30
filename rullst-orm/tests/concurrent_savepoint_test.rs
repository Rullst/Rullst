//! Nested savepoints never leave the pooled connection inside a transaction.
//!
//! The pool has a single connection, so a connection returned while still in
//! `BEGIN` would silently swallow every later write. Durability is checked
//! through an independent SQLite connection.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;
use std::time::Duration;

use rullst_orm::{Error, FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "savepoint_records")]
struct SavepointRecord {
    id: i32,
    name: String,
}

async fn insert(name: &'static str) -> Result<(), Error> {
    let mut record = SavepointRecord {
        id: 0,
        name: name.to_string(),
    };
    record.save().await
}

/// Names committed to the database file, read outside the ORM pool.
async fn durable_names(url: &str) -> Vec<String> {
    let mut connection = <sqlx::sqlite::SqliteConnection as sqlx::Connection>::connect(url)
        .await
        .expect("open an independent SQLite connection");
    let names = sqlx::query_scalar::<_, String>("SELECT name FROM savepoint_records ORDER BY id")
        .fetch_all(&mut connection)
        .await
        .expect("read committed names");
    sqlx::Connection::close(connection)
        .await
        .expect("close independent connection");
    names
}

/// A standalone write after the scenario must be durable, which fails when
/// the scenario returned its connection to the pool inside a transaction.
async fn assert_pool_is_clean(url: &str, marker: &'static str) {
    insert(marker).await.expect("standalone insert");
    assert!(
        durable_names(url).await.iter().any(|name| name == marker),
        "a later standalone write must commit ({marker})"
    );
}

/// Siblings on a task-scoped transaction that `Orm::transaction` did not open
/// (like the `#[rullst_orm::test]` sandbox) also take turns.
async fn sandbox_siblings_take_turns() {
    let sandbox = Orm::begin_transaction().await.expect("sandbox transaction");
    let sandbox = Arc::new(tokio::sync::Mutex::new(Some(sandbox)));
    rullst_orm::CURRENT_TX
        .scope(sandbox.clone(), async {
            let (first, second) = tokio::join!(
                Orm::transaction(|_| Box::pin(async {
                    insert("sandbox-a1").await?;
                    tokio::task::yield_now().await;
                    insert("sandbox-a2").await
                })),
                Orm::transaction(|_| Box::pin(async {
                    insert("sandbox-b1").await?;
                    for _ in 0..3 {
                        tokio::task::yield_now().await;
                    }
                    insert("sandbox-b2").await
                })),
            );
            first.expect("first sandbox sibling");
            second.expect("second sandbox sibling");
        })
        .await;
    let sandbox = sandbox.lock().await.take().expect("sandbox still owned");
    sandbox.rollback().await.expect("roll back sandbox");
}

/// A savepoint left open inside a managed transaction makes the commit fail
/// closed instead of releasing it and pooling a connection still in `BEGIN`.
async fn leaked_savepoint_rolls_the_transaction_back() {
    let leaked = Orm::transaction(|shared| {
        Box::pin(async move {
            insert("leaked-outer").await?;
            {
                let mut guard = shared.lock().await;
                let tx = guard.as_mut().expect("managed transaction");
                std::mem::forget(sqlx::Acquire::begin(&mut **tx).await?);
            }
            insert("inside-leaked-savepoint").await?;
            Ok::<(), Error>(())
        })
    })
    .await;
    assert!(matches!(leaked, Err(Error::Internal(_))), "{leaked:?}");
}

/// A nested future cancelled while another future holds the connection
/// cannot roll its savepoint back at once; the enclosing commit fails closed.
async fn busy_cancellation_fails_closed() {
    let cancelled = Orm::transaction(|shared| {
        Box::pin(async move {
            insert("before-busy-cancel").await?;
            let _ = tokio::time::timeout(Duration::from_millis(200), async {
                tokio::join!(
                    Orm::transaction(|_| Box::pin(async {
                        insert("abandoned").await?;
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        Ok::<(), Error>(())
                    })),
                    async {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        let _busy = shared.lock().await;
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    },
                )
            })
            .await;
            insert("after-busy-cancel").await?;
            Ok::<(), Error>(())
        })
    })
    .await;
    assert!(
        cancelled.is_err(),
        "the enclosing transaction must fail closed"
    );
}

/// A caller-owned task-scoped transaction committed with a savepoint still
/// open only releases that savepoint; the pool then closes the connection
/// instead of reusing it inside the unfinished transaction.
async fn pool_closes_a_connection_returned_inside_a_transaction() {
    let owned = Orm::begin_transaction().await.expect("owned transaction");
    let owned = Arc::new(tokio::sync::Mutex::new(Some(owned)));
    rullst_orm::CURRENT_TX
        .scope(owned.clone(), async {
            insert("owned-before-leak").await.expect("owned insert");
            let mut guard = owned.lock().await;
            let tx = guard.as_mut().expect("owned transaction");
            std::mem::forget(sqlx::Acquire::begin(&mut **tx).await.expect("savepoint"));
        })
        .await;
    let owned = owned.lock().await.take().expect("owned transaction");
    owned
        .commit()
        .await
        .expect("SQLx releases the innermost level");
}

#[tokio::test]
async fn nested_savepoints_never_pool_an_open_transaction() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-concurrent-savepoint-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    let url = format!("sqlite:{}?mode=rwc", database_path.display());
    Orm::init_with_options(&url, 1, 5)
        .await
        .expect("initialize single-connection SQLite pool");
    sqlx::query("CREATE TABLE savepoint_records (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(Orm::pool().unwrap())
        .await
        .expect("create savepoint fixture");

    sandbox_siblings_take_turns().await;
    assert!(durable_names(&url).await.is_empty(), "sandbox rolled back");
    assert_pool_is_clean(&url, "after-sandbox").await;

    leaked_savepoint_rolls_the_transaction_back().await;
    assert_pool_is_clean(&url, "after-leak").await;

    busy_cancellation_fails_closed().await;
    assert_pool_is_clean(&url, "after-busy-cancel-standalone").await;

    pool_closes_a_connection_returned_inside_a_transaction().await;
    assert_pool_is_clean(&url, "after-owned-leak").await;

    let durable = durable_names(&url).await;
    assert_eq!(
        durable,
        vec![
            "after-sandbox",
            "after-leak",
            "after-busy-cancel-standalone",
            "after-owned-leak"
        ],
        "only standalone writes commit; every leaked level was rolled back"
    );

    Orm::pool().unwrap().close().await;
    let _ = std::fs::remove_file(database_path);
}
