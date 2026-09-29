//! A nested `Orm::transaction` joins the active transaction through a
//! savepoint instead of committing independently on another connection.
//!
//! The pool has a single connection, so a nested call that checked out a
//! second pooled transaction would time out instead of succeeding.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rullst_orm::{Error, FromRow, Orm, Outbox, after_commit};
use serde_json::json;

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "nested_records")]
struct NestedRecord {
    id: i32,
    name: String,
}

async fn insert(name: &'static str) -> Result<NestedRecord, Error> {
    let mut record = NestedRecord {
        id: 0,
        name: name.to_string(),
    };
    record.save().await?;
    Ok(record)
}

async fn names() -> Vec<String> {
    NestedRecord::query()
        .order_by("id")
        .pluck_string("name")
        .await
        .expect("read committed names")
}

fn fail<T>(reason: &str) -> Result<T, Error> {
    Err(Error::Validation(reason.to_string()))
}

fn assert_send<T: Send>(_: &T) {}

#[test]
fn transaction_future_is_send() {
    // Required to nest the call inside another transaction closure, whose
    // future must be `Send`, and to run it in a spawned task.
    let future = Orm::transaction(|_| Box::pin(async { Ok::<(), Error>(()) }));
    assert_send(&future);
}

/// A task-scoped transaction that is not an `Orm::transaction`, as created by
/// `#[rullst_orm::test]`: a nested call must join it rather than check out
/// the pool's only connection again.
async fn nested_call_joins_a_task_scoped_sandbox() {
    let sandbox = Orm::begin_transaction().await.expect("sandbox transaction");
    let sandbox = Arc::new(tokio::sync::Mutex::new(Some(sandbox)));
    rullst_orm::CURRENT_TX
        .scope(sandbox.clone(), async {
            Orm::transaction(|_| Box::pin(async { insert("sandboxed").await.map(|_| ()) }))
                .await
                .expect("nested call joins the sandbox");
            assert_eq!(
                names().await,
                vec!["sandboxed"],
                "visible inside the sandbox"
            );
        })
        .await;
    let sandbox = sandbox.lock().await.take().expect("sandbox still owned");
    sandbox.rollback().await.expect("roll back sandbox");
    assert!(
        names().await.is_empty(),
        "the nested work rolls back with the sandbox"
    );
}

#[tokio::test]
async fn nested_transactions_use_savepoints_of_the_outer_transaction() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-nested-transaction-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    Orm::init_with_options(
        &format!("sqlite:{}?mode=rwc", database_path.display()),
        1,
        2,
    )
    .await
    .expect("initialize single-connection SQLite pool");
    sqlx::query("CREATE TABLE nested_records (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(Orm::pool().unwrap())
        .await
        .expect("create nested fixture");
    Outbox::install().await.expect("install outbox");
    nested_call_joins_a_task_scoped_sandbox().await;

    // An inner failure rolls back only the inner work; the outer commits.
    let callbacks = Arc::new(AtomicUsize::new(0));
    let inner_callbacks = callbacks.clone();
    Orm::transaction(move |outer| {
        Box::pin(async move {
            insert("outer-before").await?;
            let nested = Orm::transaction(move |inner| {
                Box::pin(async move {
                    assert!(Arc::ptr_eq(&outer, &inner), "nested call shares the handle");
                    insert("inner-rolled-back").await?;
                    after_commit(move || async move {
                        inner_callbacks.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .await?;
                    fail::<()>("inner step failed")
                })
            })
            .await;
            assert!(matches!(nested, Err(Error::DatabaseError(_))), "{nested:?}");
            insert("outer-after").await?;
            Ok::<(), Error>(())
        })
    })
    .await
    .expect("outer commits after a caught inner failure");
    assert_eq!(names().await, vec!["outer-before", "outer-after"]);
    assert_eq!(
        callbacks.load(Ordering::SeqCst),
        0,
        "failed savepoint discards callbacks"
    );

    // An inner success commits only with the outer transaction.
    let committed_callbacks = callbacks.clone();
    let rolled_back = Orm::transaction(move |_| {
        Box::pin(async move {
            Orm::transaction(move |_| {
                Box::pin(async move {
                    insert("phantom").await?;
                    Outbox::enqueue(
                        "orders",
                        "order:1:created",
                        "order.created",
                        &json!({"order": 1}),
                    )
                    .await?;
                    after_commit(move || async move {
                        committed_callbacks.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .await?;
                    Ok::<(), Error>(())
                })
            })
            .await?;
            fail::<()>("later outer step failed")
        })
    })
    .await;
    assert!(rolled_back.is_err());
    assert_eq!(names().await, vec!["outer-before", "outer-after"]);
    assert!(
        Outbox::claim_next("orders", "worker", 30, 3)
            .await
            .expect("claim outbox")
            .is_none(),
        "the nested outbox row must roll back with the outer transaction"
    );
    assert_eq!(
        callbacks.load(Ordering::SeqCst),
        0,
        "outer rollback discards promoted callbacks"
    );

    // Promoted callbacks run once, after the outer commit; the same row can
    // be updated by both levels without a second connection or lock wait.
    let promoted = callbacks.clone();
    Orm::transaction(move |_| {
        Box::pin(async move {
            let mut record = insert("shared-row").await?;
            let id = record.id;
            Orm::transaction(move |_| {
                Box::pin(async move {
                    let mut same = NestedRecord::find(id).await?.ok_or(Error::RecordNotFound)?;
                    same.name = "updated-inside".to_string();
                    same.save().await?;
                    after_commit(move || async move {
                        promoted.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .await?;
                    Ok::<(), Error>(())
                })
            })
            .await?;
            record.name = "updated-outside".to_string();
            record.save().await?;
            Ok::<(), Error>(())
        })
    })
    .await
    .expect("shared-row transaction commits");
    assert_eq!(callbacks.load(Ordering::SeqCst), 1);
    assert_eq!(
        names().await,
        vec!["outer-before", "outer-after", "updated-outside"]
    );

    // Two levels of nesting keep SQLx's savepoint depth balanced.
    Orm::transaction(|_| {
        Box::pin(async {
            insert("level-1").await?;
            Orm::transaction(|_| {
                Box::pin(async {
                    insert("level-2").await?;
                    let deepest = Orm::transaction(|_| {
                        Box::pin(async {
                            insert("level-3").await?;
                            fail::<()>("deepest failed")
                        })
                    })
                    .await;
                    assert!(deepest.is_err());
                    Ok::<(), Error>(())
                })
            })
            .await?;
            insert("level-1-after").await?;
            Ok::<(), Error>(())
        })
    })
    .await
    .expect("multi-level transaction commits");
    let all = names().await;
    assert!(all.ends_with(&[
        "level-1".to_string(),
        "level-2".to_string(),
        "level-1-after".to_string()
    ]));
    assert!(!all.contains(&"level-3".to_string()));

    // A cancelled nested transaction rolls its savepoint back; the outer
    // transaction stays usable and commits only its own work.
    Orm::transaction(|_| {
        Box::pin(async {
            let cancelled = tokio::time::timeout(
                Duration::from_millis(50),
                Orm::transaction(|_| {
                    Box::pin(async {
                        insert("cancelled").await?;
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        Ok::<(), Error>(())
                    })
                }),
            )
            .await;
            assert!(cancelled.is_err(), "nested future must time out");
            insert("after-cancel").await?;
            Ok::<(), Error>(())
        })
    })
    .await
    .expect("outer commits after a cancelled nested transaction");
    let all = names().await;
    assert!(all.contains(&"after-cancel".to_string()));
    assert!(!all.contains(&"cancelled".to_string()), "{all:?}");

    // Without an active transaction the call still owns its own commit.
    Orm::transaction(|_| Box::pin(async { insert("standalone").await.map(|_| ()) }))
        .await
        .expect("standalone transaction");
    assert!(names().await.contains(&"standalone".to_string()));

    Orm::pool().unwrap().close().await;
    let _ = std::fs::remove_file(database_path);
}
