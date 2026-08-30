#![cfg(all(
    feature = "redis",
    not(any(feature = "strict-postgres", feature = "strict-mysql"))
))]

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, rullst_orm::Orm)]
#[orm(table = "cache_transaction_records")]
struct CacheTransactionRecord {
    pub id: i32,
    pub name: String,
}

#[tokio::test]
async fn remembered_queries_bypass_redis_inside_every_transaction_api() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-query-cache-transaction-{}.db",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&database_path);
    let database_url = format!("sqlite:{}?mode=rwc", database_path.to_string_lossy());
    Orm::init(&database_url)
        .await
        .expect("initialize isolated SQLite ORM");
    Schema::create("cache_transaction_records", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
    })
    .await
    .expect("create cache transaction table");
    sqlx::query("INSERT INTO cache_transaction_records (name) VALUES (?)")
        .bind("visible in transaction")
        .execute(Orm::pool().expect("ORM should be initialized"))
        .await
        .expect("insert fixture");

    let mut explicit = Orm::begin_transaction()
        .await
        .expect("begin explicit transaction");
    let explicit_rows = CacheTransactionRecord::query()
        .remember(30)
        .get_with_tx(&mut explicit)
        .await
        .expect("explicit transaction must bypass uninitialized Redis");
    assert_eq!(explicit_rows.len(), 1);
    explicit.rollback().await.expect("roll back explicit read");

    let task_scoped_count = Orm::transaction(|_| {
        Box::pin(async move {
            let rows = CacheTransactionRecord::query().remember(30).get().await?;
            Ok::<usize, rullst_orm::Error>(rows.len())
        })
    })
    .await
    .expect("task-scoped transaction must bypass uninitialized Redis");
    assert_eq!(task_scoped_count, 1);

    let invalid_ttl = CacheTransactionRecord::query()
        .remember(0)
        .get()
        .await
        .expect_err("zero TTL must fail validation");
    assert!(matches!(invalid_ttl, rullst_orm::Error::Validation(_)));

    let unconfigured_cache = CacheTransactionRecord::query()
        .remember(30)
        .get()
        .await
        .expect_err("outside a transaction, an explicitly requested cache needs Redis config");
    assert!(matches!(unconfigured_cache, rullst_orm::Error::Internal(_)));

    let _ = std::fs::remove_file(database_path);
}
