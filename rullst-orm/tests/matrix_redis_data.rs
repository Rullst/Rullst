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

#[derive(Clone, Debug, rullst_orm::Orm, FromRow, PartialEq)]
#[orm(table = "redis_matrix_tenant_docs", tenant_column = "organization_id")]
struct RedisMatrixTenantDoc {
    id: i32,
    organization_id: String,
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
    exercise_legacy_model_hashes().await;
}

/// Hashes stored by 12.1 under `orm:<table>:<id>` stay readable for global
/// models and move to the namespaced key on their next write. Tenant models
/// never read or move that key, which every tenant shared.
async fn exercise_legacy_model_hashes() {
    use rullst_orm::_redis::AsyncCommands;

    let mut redis = Orm::redis_manager().expect("generated Redis connection");
    let legacy = |id: i32| format!("orm:redis_matrix_docs:{id}");
    let namespaced = |id: i32| {
        rullst_orm::query_cache::model_hash_key("redis_matrix_docs", None, &id.to_string())
            .expect("namespaced hash key")
    };
    let doc = |id: i32, title: &str, views: i64| RedisMatrixDoc {
        id,
        title: title.to_owned(),
        views,
    };
    let fields = |id: i32, title: &str, views: i64| {
        vec![
            ("id", id.to_string()),
            ("title", format!("{title:?}")),
            ("views", views.to_string()),
        ]
    };
    for (id, entries) in [
        (11, fields(11, "legacy", 4)),
        (11, vec![("note", "\"kept\"".to_owned())]),
        (12, fields(12, "legacy", 9)),
    ] {
        let _: () = redis
            .hset_multiple(legacy(id), &entries)
            .await
            .expect("write a 12.1 hash");
    }

    // A read falls back to the 12.1 hash without moving it.
    assert_eq!(
        RedisMatrixDoc::get_from_redis(11)
            .await
            .expect("legacy hash fallback read"),
        Some(doc(11, "legacy", 4))
    );
    assert!(exists(&legacy(11)).await && !exists(&namespaced(11)).await);

    // An increment moves the whole hash, extra fields included, then applies.
    assert_eq!(
        RedisMatrixDoc::increment_redis_field(11, RedisMatrixDocColumn::Views, 1)
            .await
            .expect("increment migrates the legacy hash"),
        5
    );
    assert!(!exists(&legacy(11)).await && exists(&namespaced(11)).await);
    let note: Option<String> = redis
        .hget(namespaced(11), "note")
        .await
        .expect("HGET migrated field");
    assert_eq!(note.as_deref(), Some("\"kept\""));
    assert_eq!(
        RedisMatrixDoc::get_from_redis(11)
            .await
            .expect("migrated hash read"),
        Some(doc(11, "legacy", 5))
    );

    // A save migrates as well and then overwrites the model fields.
    doc(12, "saved", 1)
        .save_to_redis()
        .await
        .expect("save migrates the legacy hash");
    assert!(!exists(&legacy(12)).await);
    assert_eq!(
        RedisMatrixDoc::get_from_redis(12)
            .await
            .expect("saved hash"),
        Some(doc(12, "saved", 1))
    );

    // An existing namespaced hash wins over a stale 12.1 hash, and the next
    // write removes the stale one.
    doc(13, "current", 1)
        .save_to_redis()
        .await
        .expect("namespaced hash");
    let _: () = redis
        .hset_multiple(legacy(13), &fields(13, "stale", 100))
        .await
        .expect("write a stale 12.1 hash");
    assert_eq!(
        RedisMatrixDoc::get_from_redis(13)
            .await
            .expect("namespaced read"),
        Some(doc(13, "current", 1))
    );
    assert_eq!(
        RedisMatrixDoc::increment_redis_field(13, RedisMatrixDocColumn::Views, 1)
            .await
            .expect("increment keeps the namespaced hash"),
        2
    );
    assert!(!exists(&legacy(13)).await);
    assert_eq!(
        RedisMatrixDoc::get_from_redis(14)
            .await
            .expect("missing hash"),
        None
    );

    // A tenant model never reads, moves or deletes the shared 12.1 key.
    let tenant_legacy = "orm:redis_matrix_tenant_docs:21";
    let _: () = redis
        .hset_multiple(
            tenant_legacy,
            &[
                ("id", "21"),
                ("organization_id", "\"acme\""),
                ("views", "7"),
            ],
        )
        .await
        .expect("write a shared 12.1 tenant hash");
    assert_eq!(
        rullst_orm::with_tenant("acme", RedisMatrixTenantDoc::get_from_redis(21))
            .await
            .expect("tenant read"),
        None
    );
    assert_eq!(
        rullst_orm::with_tenant(
            "acme",
            RedisMatrixTenantDoc::increment_redis_field(21, RedisMatrixTenantDocColumn::Views, 1),
        )
        .await
        .expect("tenant increment"),
        1
    );
    let tenant_doc = RedisMatrixTenantDoc {
        id: 21,
        organization_id: "acme".to_owned(),
        views: 2,
    };
    rullst_orm::with_tenant("acme", tenant_doc.save_to_redis())
        .await
        .expect("tenant save");
    let legacy_views: Option<String> = redis
        .hget(tenant_legacy, "views")
        .await
        .expect("HGET shared legacy hash");
    assert_eq!(legacy_views.as_deref(), Some("7"));
    let _: () = redis.del(tenant_legacy).await.expect("clean up");
}

async fn exists(key: &str) -> bool {
    use rullst_orm::_redis::AsyncCommands;

    let mut redis = Orm::redis_manager().expect("generated Redis connection");
    redis.exists(key).await.expect("EXISTS")
}
