#![cfg(all(
    feature = "redis",
    not(any(feature = "strict-postgres", feature = "strict-mysql"))
))]

use redis::AsyncCommands;
use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "query_cache_live_records")]
struct QueryCacheLiveRecord {
    pub id: i32,
    pub name: String,
}

#[tokio::test]
async fn redis_cache_is_live_bounded_and_never_replaces_transaction_state() {
    let Ok(redis_url) = std::env::var("RULLST_TEST_REDIS_URL") else {
        eprintln!("RULLST_TEST_REDIS_URL is unset; skipping the opt-in live Redis contract");
        return;
    };
    let namespace = format!("query-cache-live-{}", rand::random::<u64>());
    let database_path =
        std::env::temp_dir().join(format!("rullst-query-cache-live-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&database_path);
    let database_url = format!("sqlite:{}?mode=rwc", database_path.to_string_lossy());

    Orm::init(&database_url)
        .await
        .expect("initialize isolated SQLite ORM");
    Orm::init_redis_with_namespace(&redis_url, &namespace)
        .await
        .expect("initialize live Redis query cache");
    Schema::create("query_cache_live_records", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
    })
    .await
    .expect("create live cache table");
    sqlx::query("INSERT INTO query_cache_live_records (name) VALUES (?)")
        .bind("first")
        .execute(Orm::pool().expect("ORM should be initialized"))
        .await
        .expect("insert live cache fixture");

    let query = QueryCacheLiveRecord::query().where_id(1).limit(1);
    let cache_key = rullst_orm::query_cache::query_key(
        "query_cache_live_records",
        &query.to_sql(),
        &query.bindings,
    )
    .expect("derive namespaced cache key");
    assert!(cache_key.contains(&namespace));

    let first = query
        .clone()
        .remember(30)
        .first()
        .await
        .expect("populate live Redis cache")
        .expect("fixture should exist");
    assert_eq!(first.name, "first");

    let mut redis = Orm::redis_manager().expect("Redis manager should be initialized");
    let exists: bool = redis.exists(&cache_key).await.expect("inspect cache key");
    let ttl: i64 = redis.ttl(&cache_key).await.expect("inspect cache TTL");
    assert!(exists);
    assert!((1..=30).contains(&ttl));

    sqlx::query("UPDATE query_cache_live_records SET name = ? WHERE id = ?")
        .bind("second")
        .bind(1_i32)
        .execute(Orm::pool().expect("ORM should be initialized"))
        .await
        .expect("update authoritative row");

    let cached = query
        .clone()
        .remember(30)
        .first()
        .await
        .expect("read populated cache")
        .expect("cached fixture should exist");
    assert_eq!(cached.name, "first");

    let mut explicit = Orm::begin_transaction()
        .await
        .expect("begin explicit transaction");
    let explicit_row = query
        .clone()
        .remember(30)
        .first_with_tx(&mut explicit)
        .await
        .expect("transactional query must bypass cache")
        .expect("authoritative fixture should exist");
    assert_eq!(explicit_row.name, "second");
    explicit.rollback().await.expect("roll back explicit read");

    let task_scoped_name = Orm::transaction(|_| {
        Box::pin(async move {
            let row = QueryCacheLiveRecord::query()
                .where_id(1)
                .remember(30)
                .first()
                .await?
                .ok_or_else(|| {
                    rullst_orm::Error::DatabaseError("live cache fixture disappeared".to_string())
                })?;
            Ok::<String, rullst_orm::Error>(row.name)
        })
    })
    .await
    .expect("task-scoped query must bypass cache");
    assert_eq!(task_scoped_name, "second");

    let _: () = redis
        .set(&cache_key, "{corrupt-json")
        .await
        .expect("install corrupt cache fixture");
    let recovered = query
        .remember(30)
        .first()
        .await
        .expect("corrupt cache must fall back to database")
        .expect("authoritative fixture should exist");
    assert_eq!(recovered.name, "second");
    let repaired: String = redis
        .get(&cache_key)
        .await
        .expect("read repaired cache entry");
    assert!(repaired.contains("second"));

    let mut updated = recovered;
    updated.name = "third".to_string();
    updated
        .save()
        .await
        .expect("model save should commit before cache invalidation");
    let exists_after_commit: bool = redis
        .exists(&cache_key)
        .await
        .expect("inspect invalidated cache key");
    assert!(!exists_after_commit);

    let repopulated = QueryCacheLiveRecord::query()
        .where_id(1)
        .limit(1)
        .remember(30)
        .first()
        .await
        .expect("repopulate cache after committed update")
        .expect("updated fixture should exist");
    assert_eq!(repopulated.name, "third");

    let rollback = Orm::transaction(|_| {
        Box::pin(async move {
            let mut model = QueryCacheLiveRecord::query()
                .where_id(1)
                .first()
                .await?
                .ok_or_else(|| {
                    rullst_orm::Error::DatabaseError("live cache fixture disappeared".to_string())
                })?;
            model.name = "rolled back fourth".to_string();
            model.save().await?;
            Err::<(), rullst_orm::Error>(rullst_orm::Error::Validation(
                "force cache invalidation rollback".to_string(),
            ))
        })
    })
    .await;
    assert!(rollback.is_err());
    let exists_after_rollback: bool = redis
        .exists(&cache_key)
        .await
        .expect("cache should survive rolled-back update");
    assert!(exists_after_rollback);
    let still_cached = QueryCacheLiveRecord::query()
        .where_id(1)
        .limit(1)
        .remember(30)
        .first()
        .await
        .expect("read cache after rollback")
        .expect("cached fixture should exist");
    assert_eq!(still_cached.name, "third");

    let _: usize = redis
        .del(&cache_key)
        .await
        .expect("remove isolated live cache key");
    let _ = std::fs::remove_file(database_path);
}
