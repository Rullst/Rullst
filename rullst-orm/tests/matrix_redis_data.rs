#![cfg(feature = "redis")]

mod support;

use rullst_orm::{
    FromRow, Orm, RedisDataConfig, RedisDataKey, RedisDataStore, RedisField, RedisMember,
    RedisScanLimit, RedisStructure, RedisStructuresRepository, RedisValue,
};
use serde::{Deserialize, Serialize};
use testcontainers::GenericImage;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, FromRow, PartialEq, Default)]
#[orm(table = "redis_matrix_docs")]
struct RedisMatrixDoc {
    id: i32,
    title: String,
    views: i64,
}

#[tokio::test]
async fn redis_native_structures_pass_a_live_namespaced_lifecycle() {
    let container = match GenericImage::new(
        "redis",
        "7.4-alpine@sha256:ff02b58f971e7d7d156a1267e283fcbbeee91773b6aa36c49dac28ecfe28eadf",
    )
    .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
    .with_exposed_port(6379.tcp())
    .start()
    .await
    {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("Redis", error);
            return;
        }
    };
    let host = container
        .get_host()
        .await
        .expect("Redis host should be available");
    let port = container
        .get_host_port_ipv4(6379)
        .await
        .expect("Redis port should be available");
    let endpoint = format!("redis://{host}:{port}");
    let store = RedisDataStore::connect_or_mock(RedisDataConfig::unauthenticated_local(
        &endpoint,
        "matrix-primary",
    ))
    .await
    .expect("live Redis data adapter should initialize");
    assert!(!store.is_mock());

    let key = RedisDataKey::new("account:42").expect("valid data key");
    let name = RedisField::new("name").expect("valid hash field");
    let visits = RedisField::new("visits").expect("valid hash field");
    assert!(
        store
            .hash_set(&key, &name, &RedisValue::new("Ada").expect("valid value"))
            .await
            .expect("hash field should be inserted")
    );
    assert_eq!(
        store
            .hash_get(&key, &name)
            .await
            .expect("hash read should succeed")
            .expect("hash value should exist")
            .as_str(),
        "Ada"
    );
    assert_eq!(
        store
            .hash_increment(&key, &visits, 5)
            .await
            .expect("hash increment should be atomic"),
        5
    );

    let reader = RedisMember::new("reader").expect("valid member");
    let admin = RedisMember::new("admin").expect("valid member");
    assert!(
        store
            .set_add(&key, &reader)
            .await
            .expect("set member should be inserted")
    );
    assert!(
        store
            .set_contains(&key, &reader)
            .await
            .expect("membership should be checked")
    );
    let scanned = store
        .set_scan(&key, RedisScanLimit::new(1).expect("valid scan limit"))
        .await
        .expect("bounded SSCAN should succeed");
    assert_eq!(scanned.len(), 1);

    store
        .sorted_set_add(&key, &reader, 10.0)
        .await
        .expect("first score should be stored");
    store
        .sorted_set_add(&key, &admin, 50.0)
        .await
        .expect("second score should be stored");
    let ranking = store
        .sorted_set_top(&key, RedisScanLimit::new(2).expect("valid scan limit"))
        .await
        .expect("bounded ZREVRANGE should succeed");
    assert_eq!(ranking.len(), 2);
    assert_eq!(ranking[0].member(), &admin);
    assert_eq!(ranking[0].score(), 50.0);

    let isolated = RedisDataStore::connect_or_mock(RedisDataConfig::unauthenticated_local(
        &endpoint,
        "matrix-isolated",
    ))
    .await
    .expect("second namespace should initialize");
    assert!(
        isolated
            .hash_get(&key, &name)
            .await
            .expect("isolated hash read should succeed")
            .is_none()
    );
    assert!(
        store
            .delete(&key, RedisStructure::Hash)
            .await
            .expect("exact structure deletion should succeed")
    );
    assert!(
        store
            .set_contains(&key, &reader)
            .await
            .expect("hash deletion must preserve the set")
    );

    Orm::init_redis_with_namespace(&endpoint, "matrix-generated")
        .await
        .expect("generated Redis hash connection should initialize");
    let document = RedisMatrixDoc {
        id: 7,
        title: "live hash".to_owned(),
        views: 3,
    };
    document
        .save_to_redis()
        .await
        .expect("generated model hash should save");
    assert_eq!(
        RedisMatrixDoc::get_from_redis(7)
            .await
            .expect("generated model hash should load"),
        Some(document)
    );
    assert_eq!(
        RedisMatrixDoc::increment_redis_field(7, RedisMatrixDocColumn::Views, 2)
            .await
            .expect("generated numeric hash field should increment"),
        5
    );
}
