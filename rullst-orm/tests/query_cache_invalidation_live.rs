//! Table invalidation counts and model-hash fallback against
//! `RULLST_TEST_REDIS_URL` (skipped when unset).
#![cfg(all(
    feature = "redis",
    not(any(feature = "strict-postgres", feature = "strict-mysql"))
))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use redis::AsyncCommands;
use rullst_orm::Orm;
use rullst_orm::query_cache::{
    invalidate_table, query_key_for_model, read_model_hash, store_entry, write_model_hash,
};

async fn cache_entries(table: &str, count: usize) {
    for index in 0..count {
        let key = query_key_for_model(table, false, &format!("SELECT {index}"), &[]).unwrap();
        store_entry(&key, "[]", 300).await.unwrap();
    }
}

#[tokio::test]
async fn invalidation_counts_every_entry_and_hashes_fall_back_to_legacy_keys() {
    let Ok(redis_url) = std::env::var("RULLST_TEST_REDIS_URL") else {
        eprintln!("RULLST_TEST_REDIS_URL is unset; skipping the opt-in live Redis contract");
        return;
    };
    let namespace = format!("cache-invalidation-{}", rand::random::<u64>());
    Orm::init("sqlite::memory:").await.unwrap();
    Orm::init_redis_with_namespace(&redis_url, &namespace)
        .await
        .unwrap();

    assert_eq!(invalidate_table("live_entries").await.unwrap(), 0);
    cache_entries("live_entries", 2).await;
    assert_eq!(invalidate_table("live_entries").await.unwrap(), 2);
    // More than one 500-key batch is drained in a single call.
    cache_entries("live_entries", 501).await;
    assert_eq!(invalidate_table("live_entries").await.unwrap(), 501);
    assert_eq!(invalidate_table("live_entries").await.unwrap(), 0);

    let mut connection = redis::Client::open(redis_url.as_str())
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    let table = format!("legacy_{}", rand::random::<u32>());
    let legacy = format!("orm:{table}:7");
    let _: () = connection.hset(&legacy, "name", "legacy").await.unwrap();
    let only_legacy = read_model_hash(&table, None, "7").await.unwrap();
    assert_eq!(only_legacy.get("name").map(String::as_str), Some("legacy"));

    write_model_hash(&table, None, "8", &[("name", "current".to_string())])
        .await
        .unwrap();
    let _: () = connection
        .hset(format!("orm:{table}:8"), "name", "stale")
        .await
        .unwrap();
    let current = read_model_hash(&table, None, "8").await.unwrap();
    assert_eq!(current.get("name").map(String::as_str), Some("current"));
    let _: () = connection
        .del(&[legacy, format!("orm:{table}:8")])
        .await
        .unwrap();
}
