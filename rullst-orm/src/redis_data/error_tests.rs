//! Redis failures read once: "Redis operation failed: <reason>".
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::{
    RedisDataConfig, RedisDataKey, RedisDataStore, RedisField, RedisStructuresRepository,
    RedisValue, redis_error,
};
use crate::polyglot::PolyglotError;

fn message(error: PolyglotError) -> String {
    let text = error.to_string();
    match error {
        PolyglotError::Driver { backend, message } => {
            assert_eq!(backend, "Redis");
            assert_eq!(text, format!("Redis operation failed: {message}"));
            assert_eq!(text.matches("operation failed").count(), 1, "{text}");
            message
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn driver_errors_keep_the_server_code_and_reason() {
    let server = redis::make_extension_error(
        "WRONGPASS".to_owned(),
        Some("invalid username-password pair or user is disabled.".to_owned()),
    );
    assert_eq!(
        message(redis_error(server)),
        "WRONGPASS invalid username-password pair or user is disabled."
    );
    // Without a reason, the driver's own text.
    let bare = redis::make_extension_error("NOAUTH".to_owned(), None);
    assert_eq!(message(redis_error(bare)), "\"NOAUTH\"");
    let io = redis::RedisError::from((redis::ErrorKind::Io, "connection refused"));
    assert_eq!(message(redis_error(io)), "connection refused - Io");
    let leaky = redis::RedisError::from((
        redis::ErrorKind::InvalidClientConfig,
        "cannot connect",
        "rediss://user:secret@cache.example:6380/0\nretrying".to_owned(),
    ));
    assert_eq!(
        message(redis_error(leaky)),
        "cannot connect - InvalidClientConfig: [redacted URL] retrying"
    );
}

#[tokio::test]
async fn the_offline_increment_fails_like_redis() {
    let store = RedisDataStore::connect_or_mock(RedisDataConfig::new("", "errors", "", ""))
        .await
        .unwrap();
    let key = RedisDataKey::new("account:1").unwrap();
    let name = RedisField::new("name").unwrap();
    let visits = RedisField::new("visits").unwrap();
    store
        .hash_set(&key, &name, &RedisValue::new("Ada").unwrap())
        .await
        .unwrap();
    assert_eq!(
        message(store.hash_increment(&key, &name, 1).await.unwrap_err()),
        "ERR hash value is not an integer"
    );
    store.hash_increment(&key, &visits, i64::MAX).await.unwrap();
    assert_eq!(
        message(store.hash_increment(&key, &visits, 1).await.unwrap_err()),
        "ERR increment or decrement would overflow"
    );
}

#[tokio::test]
async fn an_unreachable_server_is_reported_once() {
    // Nothing listens on port 1 of the loopback interface.
    let refused = RedisDataStore::connect_or_mock(RedisDataConfig::unauthenticated_local(
        "redis://127.0.0.1:1",
        "errors",
    ))
    .await;
    let Err(error) = refused else {
        panic!("a closed port must not connect");
    };
    let reason = message(error);
    assert!(!reason.is_empty());
    assert_ne!(reason, "Redis operation failed");
}

/// Runs against the server named by `RULLST_TEST_REDIS_URL`; skipped when
/// it is unset.
#[tokio::test]
async fn a_live_increment_of_text_keeps_the_server_reason() {
    let Ok(url) = std::env::var("RULLST_TEST_REDIS_URL") else {
        eprintln!("RULLST_TEST_REDIS_URL is unset; skipping the live Redis error contract");
        return;
    };
    let namespace = format!("live-errors-{}", rand::random::<u32>());
    let store =
        RedisDataStore::connect_or_mock(RedisDataConfig::unauthenticated_local(url, namespace))
            .await
            .unwrap();
    let key = RedisDataKey::new("account:1").unwrap();
    let name = RedisField::new("name").unwrap();
    store
        .hash_set(&key, &name, &RedisValue::new("Ada").unwrap())
        .await
        .unwrap();
    assert_eq!(
        message(store.hash_increment(&key, &name, 1).await.unwrap_err()),
        "ERR hash value is not an integer"
    );
    assert!(
        store
            .delete(&key, super::RedisStructure::Hash)
            .await
            .unwrap()
    );
}
