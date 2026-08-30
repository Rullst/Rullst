use super::{
    RedisDataConfig, RedisDataKey, RedisDataStore, RedisField, RedisMember, RedisScanLimit,
    RedisStructure, RedisStructuresRepository, RedisValue,
};
use crate::polyglot::{Backend, Capability, PolyglotError};

#[tokio::test]
async fn mock_exercises_hash_set_and_sorted_set_contracts() {
    let store = RedisDataStore::connect_or_mock(RedisDataConfig::new("", "test-app", "", ""))
        .await
        .expect("empty credentials should select the mock");
    assert!(store.is_mock());
    assert_eq!(RedisDataStore::capabilities().backend(), Backend::Redis);
    assert!(
        RedisDataStore::capabilities().supports(Capability::KeyValueStructures),
        "Redis should declare the bounded native-structures capability"
    );
    let key = RedisDataKey::new("account:42").expect("valid key");
    let field = RedisField::new("display_name").expect("valid field");
    assert!(
        store
            .hash_set(&key, &field, &RedisValue::new("Ada").expect("valid value"))
            .await
            .expect("hash set should succeed")
    );
    assert_eq!(
        store
            .hash_get(&key, &field)
            .await
            .expect("hash get should succeed")
            .expect("hash value should exist")
            .as_str(),
        "Ada"
    );
    let counter = RedisField::new("visits").expect("valid field");
    assert_eq!(
        store
            .hash_increment(&key, &counter, 3)
            .await
            .expect("atomic increment should succeed"),
        3
    );

    let alpha = RedisMember::new("alpha").expect("valid member");
    let beta = RedisMember::new("beta").expect("valid member");
    assert!(
        store
            .set_add(&key, &alpha)
            .await
            .expect("set add should succeed")
    );
    assert!(
        store
            .set_contains(&key, &alpha)
            .await
            .expect("membership check should succeed")
    );
    let members = store
        .set_scan(&key, RedisScanLimit::new(1).expect("valid scan limit"))
        .await
        .expect("bounded scan should succeed");
    assert_eq!(members, vec![alpha.clone()]);

    store
        .sorted_set_add(&key, &alpha, 1.0)
        .await
        .expect("sorted-set insert should succeed");
    store
        .sorted_set_add(&key, &beta, 9.0)
        .await
        .expect("second sorted-set insert should succeed");
    let ranking = store
        .sorted_set_top(&key, RedisScanLimit::new(2).expect("valid scan limit"))
        .await
        .expect("ranking should succeed");
    assert_eq!(ranking.len(), 2);
    assert_eq!(ranking[0].member(), &beta);
    assert_eq!(ranking[0].score(), 9.0);

    assert!(
        store
            .delete(&key, RedisStructure::Hash)
            .await
            .expect("hash deletion should succeed")
    );
    assert!(
        store
            .hash_get(&key, &field)
            .await
            .expect("hash get after delete should succeed")
            .is_none()
    );
    assert!(
        store
            .set_contains(&key, &alpha)
            .await
            .expect("deleting the hash must not delete the set")
    );
}

#[test]
fn validates_inputs_endpoint_policy_and_secret_redaction() {
    assert!(RedisDataKey::new("../escape").is_err());
    assert!(RedisField::new("line\nbreak").is_err());
    assert!(RedisMember::new("").is_err());
    assert!(RedisMember::new("line\nbreak").is_err());
    assert!(RedisScanLimit::new(0).is_err());
    assert!(RedisScanLimit::new(1_001).is_err());
    assert!(RedisValue::new("x".repeat(1024 * 1024 + 1)).is_err());
    assert!(matches!(
        super::validate_score(f64::NAN),
        Err(PolyglotError::InvalidConfiguration {
            backend: "Redis",
            ..
        })
    ));
    let debug = format!(
        "{:?}",
        RedisDataConfig::new(
            "rediss://private.example.com",
            "application",
            "private-user",
            "private-password"
        )
    );
    assert!(!debug.contains("private.example.com"));
    assert!(!debug.contains("private-user"));
    assert!(!debug.contains("private-password"));
}

#[tokio::test]
async fn rejects_cleartext_remote_and_unauthenticated_remote_endpoints() {
    assert!(
        RedisDataStore::connect_or_mock(RedisDataConfig::new(
            "redis://cache.example.com",
            "application",
            "default",
            "secret"
        ))
        .await
        .is_err()
    );
    assert!(
        RedisDataStore::connect_or_mock(RedisDataConfig::unauthenticated_local(
            "rediss://cache.example.com",
            "application"
        ))
        .await
        .is_err()
    );
}
