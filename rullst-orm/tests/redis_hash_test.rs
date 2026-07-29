use rullst_orm::Orm;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Clone, Debug, Serialize, Deserialize, Orm, FromRow, PartialEq, Default)]
#[orm(table = "redis_test_docs")]
pub struct RedisDoc {
    pub id: i32,
    pub title: String,
    pub views: i64,
}

#[tokio::test]
#[cfg(feature = "redis")]
async fn test_redis_hashes() {
    // Note: This test requires a running Redis instance on localhost:6379
    // If Redis is not available, it should probably be skipped or handled gracefully.
    // We'll initialize ORM with a fake SQLite db for SQL, and a redis connection.
    let init_res = Orm::init_with_options("sqlite::memory:", 1, 1).await;
    assert!(init_res.is_ok());

    let redis_res = Orm::init_redis("redis://127.0.0.1:6379").await;
    if redis_res.is_err() {
        println!("Skipping redis_hash_test because Redis is not available on 127.0.0.1:6379");
        return;
    }

    let doc = RedisDoc {
        id: 99,
        title: "My First Redis Hash".to_string(),
        views: 10,
    };

    // 1. Save to Redis
    let save_res = doc.save_to_redis().await;
    assert!(
        save_res.is_ok(),
        "Failed to save to redis: {:?}",
        save_res.err()
    );

    // 2. Get from Redis
    let fetched = RedisDoc::get_from_redis(99).await.unwrap();
    assert!(fetched.is_some());

    let fetched_doc = fetched.unwrap();
    assert_eq!(fetched_doc.id, 99);
    assert_eq!(fetched_doc.title, "My First Redis Hash");
    assert_eq!(fetched_doc.views, 10);

    // 3. Increment a field
    let new_views = RedisDoc::increment_redis_field(99, RedisDocColumn::Views, 5)
        .await
        .unwrap();
    assert_eq!(new_views, 15);

    // 4. Fetch again to verify increment
    let fetched_again = RedisDoc::get_from_redis(99).await.unwrap().unwrap();
    assert_eq!(fetched_again.views, 15);
}
