//! A nested `Orm::transaction` that leaves a deeper savepoint open fails and
//! rolls its work back instead of releasing the leaked level.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::{Error, Orm};

#[derive(Debug, Clone, rullst_orm::Orm)]
#[orm(table = "balance_records")]
struct BalanceRecord {
    id: i32,
    name: String,
}

async fn insert(name: &str) -> Result<(), Error> {
    let mut record = BalanceRecord {
        id: 0,
        name: name.to_string(),
    };
    record.save().await
}

#[tokio::test]
async fn a_leaked_inner_savepoint_rolls_the_nested_work_back() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-savepoint-balance-{}-{}.db",
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
    sqlx::query("CREATE TABLE balance_records (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(Orm::pool().unwrap())
        .await
        .expect("create fixture");

    Orm::transaction(|_| {
        Box::pin(async move {
            insert("outer").await?;
            let nested = Orm::transaction(|inner| {
                Box::pin(async move {
                    insert("nested").await?;
                    let mut guard = inner.lock().await;
                    let tx = guard.as_mut().expect("managed transaction");
                    let deeper = sqlx::Acquire::begin(&mut **tx).await?;
                    // Leaves the deeper savepoint open on the connection.
                    std::mem::forget(deeper);
                    drop(guard);
                    Ok::<(), Error>(())
                })
            })
            .await;
            let message = nested
                .expect_err("an unbalanced savepoint fails")
                .to_string();
            assert!(
                message.contains("nested transaction savepoints were left unbalanced"),
                "{message}"
            );
            insert("outer-after").await?;
            Ok::<(), Error>(())
        })
    })
    .await
    .expect("the outer transaction commits");

    let names = BalanceRecord::query()
        .order_by("id")
        .pluck_string("name")
        .await
        .expect("read names");
    assert_eq!(names, ["outer", "outer-after"]);

    Orm::pool().unwrap().close().await;
    let _ = std::fs::remove_file(database_path);
}
