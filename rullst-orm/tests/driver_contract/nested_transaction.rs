//! A nested `Orm::transaction` joins the outer transaction through a
//! savepoint on every driver. Without it, the same-row update below waits on
//! the outer transaction's row lock on PostgreSQL and MySQL/MariaDB.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{Error, FromRow, Orm, after_commit};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_nested_rows")]
struct ContractNestedRow {
    id: i32,
    name: String,
}

async fn insert(name: &'static str) -> Result<ContractNestedRow, Error> {
    let mut row = ContractNestedRow {
        id: 0,
        name: name.to_string(),
    };
    row.save().await?;
    Ok(row)
}

async fn names() -> Vec<String> {
    ContractNestedRow::query()
        .order_by("id")
        .pluck_string("name")
        .await
        .expect("read nested contract rows")
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_nested_rows", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
    })
    .await
    .expect("create nested transaction contract table");

    let bounded = tokio::time::timeout(
        Duration::from_secs(30),
        Orm::transaction(|_| {
            Box::pin(async {
                let mut outer = insert("outer").await?;
                let id = outer.id;
                let failed = Orm::transaction(|_| {
                    Box::pin(async {
                        insert("rolled-back").await?;
                        Err::<(), Error>(Error::Validation("inner failure".to_string()))
                    })
                })
                .await;
                assert!(failed.is_err());
                Orm::transaction(move |_| {
                    Box::pin(async move {
                        let mut same = ContractNestedRow::find(id)
                            .await?
                            .ok_or(Error::RecordNotFound)?;
                        same.name = "updated-inside".to_string();
                        same.save().await
                    })
                })
                .await?;
                outer.name = "updated-outside".to_string();
                outer.save().await?;
                Ok::<(), Error>(())
            })
        }),
    )
    .await
    .unwrap_or_else(|_| panic!("{driver} nested transaction must not wait on its own row lock"));
    bounded.unwrap_or_else(|error| panic!("{driver} nested transaction: {error}"));
    assert_eq!(names().await, vec!["updated-outside"], "{driver}");

    let rolled_back = Orm::transaction(|_| {
        Box::pin(async {
            Orm::transaction(|_| Box::pin(async { insert("phantom").await.map(|_| ()) })).await?;
            Err::<(), Error>(Error::Validation("outer failure".to_string()))
        })
    })
    .await;
    assert!(rolled_back.is_err());
    assert_eq!(
        names().await,
        vec!["updated-outside"],
        "{driver} nested success must roll back with the outer transaction"
    );

    exercise_concurrent_siblings(driver).await;

    Schema::drop_if_exists("contract_nested_rows")
        .await
        .expect("drop nested transaction contract table");
}

/// Sibling nested transactions joined on one task take turns on the shared
/// connection, so their savepoints never interleave: each releases or rolls
/// back only its own work, and the outer transaction still commits.
async fn exercise_concurrent_siblings(driver: &str) {
    let committed = Arc::new(AtomicUsize::new(0));
    let callback = committed.clone();
    let siblings = tokio::time::timeout(
        Duration::from_secs(30),
        Orm::transaction(move |_| {
            Box::pin(async move {
                let (first, second, failed) = tokio::join!(
                    Orm::transaction(|_| Box::pin(async {
                        insert("sibling-a1").await?;
                        tokio::task::yield_now().await;
                        insert("sibling-a2").await?;
                        Ok::<(), Error>(())
                    })),
                    Orm::transaction(move |_| Box::pin(async move {
                        insert("sibling-b1").await?;
                        for _ in 0..3 {
                            tokio::task::yield_now().await;
                        }
                        insert("sibling-b2").await?;
                        after_commit(move || async move {
                            callback.fetch_add(1, Ordering::SeqCst);
                            Ok(())
                        })
                        .await
                    })),
                    Orm::transaction(|_| Box::pin(async {
                        insert("sibling-c-rolled-back").await?;
                        tokio::task::yield_now().await;
                        Err::<(), Error>(Error::Validation("sibling failure".to_string()))
                    })),
                );
                first?;
                second?;
                assert!(failed.is_err(), "the failing sibling reports its error");
                Ok::<(), Error>(())
            })
        }),
    )
    .await
    .unwrap_or_else(|_| panic!("{driver} sibling nested transactions must not deadlock"));
    siblings.unwrap_or_else(|error| panic!("{driver} sibling nested transactions: {error}"));

    let rows = names().await;
    for expected in ["sibling-a1", "sibling-a2", "sibling-b1", "sibling-b2"] {
        assert!(
            rows.iter().any(|name| name == expected),
            "{driver} {expected}"
        );
    }
    assert!(
        !rows.iter().any(|name| name == "sibling-c-rolled-back"),
        "{driver} the failed sibling rolls back only its own savepoint"
    );
    assert_eq!(committed.load(Ordering::SeqCst), 1, "{driver} callbacks");
}
