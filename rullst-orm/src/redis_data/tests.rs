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

#[test]
fn bracketed_ipv6_loopback_is_a_local_endpoint() {
    assert!(
        RedisDataConfig::unauthenticated_local("redis://[::1]:6379", "application")
            .validate()
            .is_ok()
    );
    assert!(
        RedisDataConfig::unauthenticated_local("redis://[2001:db8::1]:6379", "application")
            .validate()
            .is_err()
    );
}

/// Like Redis `ZREVRANGE`, members with equal scores come in descending
/// lexicographic order, so a limit that cuts through a tie keeps the same
/// members offline and live.
#[tokio::test]
async fn mock_breaks_sorted_set_ties_like_zrevrange() {
    let store = RedisDataStore::connect_or_mock(RedisDataConfig::new("", "test-app", "", ""))
        .await
        .expect("empty credentials should select the mock");
    let key = RedisDataKey::new("leaderboard:ties").expect("valid key");
    for (member, score) in [("alice", 10.0), ("bob", 10.0), ("carol", 5.0)] {
        store
            .sorted_set_add(
                &key,
                &RedisMember::new(member).expect("valid member"),
                score,
            )
            .await
            .expect("sorted-set insert should succeed");
    }
    let top = |limit| {
        let store = &store;
        let key = &key;
        async move {
            store
                .sorted_set_top(key, RedisScanLimit::new(limit).expect("valid scan limit"))
                .await
                .expect("ranking should succeed")
                .into_iter()
                .map(|row| row.member().as_str().to_owned())
                .collect::<Vec<_>>()
        }
    };
    assert_eq!(top(1).await, ["bob"]);
    assert_eq!(top(3).await, ["bob", "alice", "carol"]);
}

fn validated(config: RedisDataConfig) -> Result<(), &'static str> {
    config.validate().map(|_| ()).map_err(|error| match error {
        PolyglotError::InvalidConfiguration { reason, .. } => reason,
        other => panic!("unexpected error: {other:?}"),
    })
}

fn acl(endpoint: &str, username: &str, password: &str) -> RedisDataConfig {
    RedisDataConfig::new(endpoint, "application", username, password)
}

#[test]
fn any_mock_endpoint_or_credential_selects_the_offline_backend() {
    assert!(acl("", "user", "secret").requests_mock());
    assert!(acl("mock://redis", "user", "secret").requests_mock());
    assert!(acl("rediss://cache.example.com", "mock_user", "secret").requests_mock());
    assert!(acl("rediss://cache.example.com", "user", "mock_secret").requests_mock());
    assert!(!acl("rediss://cache.example.com", "user", "secret").requests_mock());
    assert!(
        !RedisDataConfig::unauthenticated_local("redis://127.0.0.1", "application").requests_mock()
    );
    assert_eq!(
        format!("{:?}", acl("rediss://cache.example.com", "user", "secret")),
        "RedisDataConfig { endpoint: \"[CONFIGURED]\", credentials: \"[REDACTED]\", namespace: \"application\" }"
    );
}

#[tokio::test]
async fn the_offline_backend_still_validates_its_namespace() {
    let invalid = RedisDataStore::connect_or_mock(RedisDataConfig::new("", "bad/ns", "", "")).await;
    assert!(matches!(
        invalid,
        Err(PolyglotError::InvalidConfiguration {
            backend: "Redis",
            ..
        })
    ));
}

#[test]
fn endpoints_are_bare_tls_urls_unless_local() {
    let shape =
        "endpoint must not contain credentials, a database path, query parameters, or a fragment";
    for endpoint in [
        "rediss://user@cache.example.com",
        "rediss://:pw@cache.example.com",
        "rediss://cache.example.com?db=1",
        "rediss://cache.example.com#x",
        "rediss://cache.example.com/2",
    ] {
        assert_eq!(
            validated(acl(endpoint, "user", "secret")),
            Err(shape),
            "{endpoint}"
        );
    }
    assert_eq!(
        validated(acl("rediss://cache.example.com/", "user", "secret")),
        Ok(())
    );
    assert_eq!(
        validated(acl("redis://127.0.0.1:6379", "user", "secret")),
        Ok(())
    );
    assert_eq!(
        validated(acl("redis://cache.example.com", "user", "secret")),
        Err("remote endpoints must use rediss:// TLS")
    );
    assert_eq!(
        validated(acl("http://cache.example.com", "user", "secret")),
        Err("only redis:// and rediss:// endpoints are supported")
    );
}

#[test]
fn credentials_hold_one_to_2048_visible_bytes() {
    let message = "credentials must contain 1-2,048 visible ASCII bytes";
    let endpoint = "rediss://cache.example.com";
    assert_eq!(validated(acl(endpoint, "user", "pass word")), Ok(()));
    assert_eq!(validated(acl(endpoint, "user", &"p".repeat(2_048))), Ok(()));
    assert_eq!(
        validated(acl(endpoint, &"u".repeat(2_048), "secret")),
        Ok(())
    );
    for (username, password) in [
        ("user", "p".repeat(2_049)),
        ("user", "pass\tword".to_string()),
        ("us\ner", "secret".to_string()),
        ("usér", "secret".to_string()),
    ] {
        assert_eq!(
            validated(acl(endpoint, username, &password)),
            Err(message),
            "{username:?}"
        );
    }
}

#[tokio::test]
async fn bounded_wrappers_and_mock_membership_are_exact() {
    assert!(RedisMember::new("m".repeat(4 * 1024)).is_ok());
    assert!(RedisMember::new("m".repeat(4 * 1024 + 1)).is_err());
    assert!(RedisValue::new("v".repeat(1024 * 1024)).is_ok());
    assert_eq!(
        RedisDataKey::new("account:7").unwrap().as_str(),
        "account:7"
    );
    assert_eq!(RedisValue::new("value").unwrap().into_inner(), "value");
    assert_eq!(RedisMember::new("member").unwrap().into_inner(), "member");

    let store = RedisDataStore::connect_or_mock(RedisDataConfig::new("", "test-app", "", ""))
        .await
        .expect("mock");
    let key = RedisDataKey::new("members").unwrap();
    let present = RedisMember::new("present").unwrap();
    store.set_add(&key, &present).await.unwrap();
    assert!(store.set_contains(&key, &present).await.unwrap());
    assert!(
        !store
            .set_contains(&key, &RedisMember::new("absent").unwrap())
            .await
            .unwrap()
    );
}

/// Runs against the server named by `RULLST_TEST_REDIS_URL` (a loopback,
/// unauthenticated test instance); skipped when it is unset.
#[tokio::test]
async fn live_structures_round_trip_in_their_namespace() {
    let Ok(url) = std::env::var("RULLST_TEST_REDIS_URL") else {
        eprintln!("RULLST_TEST_REDIS_URL is unset; skipping the live Redis data contract");
        return;
    };
    let namespace = format!("live-data-{}", rand::random::<u32>());
    let connect = |namespace: String| {
        RedisDataStore::connect_or_mock(RedisDataConfig::unauthenticated_local(
            url.clone(),
            namespace,
        ))
    };
    let store = connect(namespace.clone()).await.expect("live store");
    let other = connect(format!("{namespace}-other"))
        .await
        .expect("second namespace");
    assert!(!store.is_mock());
    let key = RedisDataKey::new("account:1").unwrap();
    let field = RedisField::new("name").unwrap();

    assert!(
        store
            .hash_set(&key, &field, &RedisValue::new("Ada").unwrap())
            .await
            .unwrap()
    );
    assert!(
        !store
            .hash_set(&key, &field, &RedisValue::new("Grace").unwrap())
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .hash_get(&key, &field)
            .await
            .unwrap()
            .map(RedisValue::into_inner),
        Some("Grace".to_string())
    );
    assert_eq!(other.hash_get(&key, &field).await.unwrap(), None);

    let members = ["alpha", "beta", "gamma"].map(|member| RedisMember::new(member).unwrap());
    for member in &members {
        assert!(store.set_add(&key, member).await.unwrap());
    }
    assert!(!store.set_add(&key, &members[0]).await.unwrap());
    assert!(store.set_contains(&key, &members[1]).await.unwrap());
    assert!(
        !store
            .set_contains(&key, &RedisMember::new("delta").unwrap())
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .set_scan(&key, RedisScanLimit::new(1).unwrap())
            .await
            .unwrap()
            .len(),
        1
    );
    let mut scanned: Vec<String> = store
        .set_scan(&key, RedisScanLimit::new(10).unwrap())
        .await
        .unwrap()
        .into_iter()
        .map(RedisMember::into_inner)
        .collect();
    scanned.sort();
    assert_eq!(scanned, ["alpha", "beta", "gamma"]);

    for (member, score) in members.iter().zip([1.0, 2.0, 3.0]) {
        assert!(store.sorted_set_add(&key, member, score).await.unwrap());
    }
    assert!(!store.sorted_set_add(&key, &members[0], 4.0).await.unwrap());
    let top: Vec<(String, f64)> = store
        .sorted_set_top(&key, RedisScanLimit::new(2).unwrap())
        .await
        .unwrap()
        .into_iter()
        .map(|scored| (scored.member().as_str().to_owned(), scored.score()))
        .collect();
    assert_eq!(
        top,
        [("alpha".to_string(), 4.0), ("gamma".to_string(), 3.0)]
    );

    for structure in [
        RedisStructure::Hash,
        RedisStructure::Set,
        RedisStructure::SortedSet,
    ] {
        assert!(store.delete(&key, structure).await.unwrap());
        assert!(!store.delete(&key, structure).await.unwrap());
    }
}
