#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::{BrokerConfig, MessageBroker, PublishRequest};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
struct FixedClock(i64);

impl Clock for FixedClock {
    fn now_millis(&self) -> Result<i64> {
        Ok(self.0)
    }
}

fn fixture(label: &str) -> (PathBuf, String) {
    let directory = PathBuf::from("target").join("rullst-messaging-tests");
    std::fs::create_dir_all(&directory).expect("create fixture directory");
    let path = directory.join(format!("{label}-{}.sqlite", uuid::Uuid::new_v4().simple()));
    let absolute = std::fs::canonicalize(&directory)
        .expect("canonical fixture directory")
        .join(path.file_name().expect("fixture name"));
    (absolute.clone(), format!("sqlite://{}", absolute.display()))
}

fn cleanup(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

fn request(topic: &str, key: &str, payload: impl Into<Vec<u8>>) -> PublishRequest {
    PublishRequest::try_new(topic, "event.ready", key, payload).expect("valid request")
}

#[tokio::test]
async fn publication_rejects_payload_bounds_corrupt_ids_and_exhausted_sequences() {
    let (path, url) = fixture("publish-guards");
    let config = BrokerConfig::try_new("publish-guards")
        .unwrap()
        .with_limits(16, 8, 3, 4)
        .unwrap();
    let broker = SqliteBroker::connect_with_clock(url, config, FixedClock(123_456))
        .await
        .unwrap();

    assert_eq!(
        broker
            .publish(request("events", "too-large", [0_u8; 5]))
            .await,
        Err(MessagingError::CapacityExceeded {
            resource: "message payload bytes",
            limit: 4,
        })
    );

    let duplicate = request("events", "event/1", b"one".to_vec());
    broker.publish(duplicate.clone()).await.unwrap();
    sqlx::query(
        "UPDATE rullst_messaging_messages SET message_id = 'bad' WHERE namespace = ? AND idempotency_key = ?",
    )
    .bind("publish-guards")
    .bind("event/1")
    .execute(&broker.pool)
    .await
    .unwrap();
    assert_eq!(
        broker.publish(duplicate).await,
        Err(MessagingError::CorruptStorage {
            context: "message identifier",
        })
    );

    sqlx::query(
        "INSERT INTO rullst_messaging_topics (namespace, topic, next_sequence) VALUES (?, ?, ?)",
    )
    .bind("publish-guards")
    .bind("exhausted")
    .bind(i64::MAX)
    .execute(&broker.pool)
    .await
    .unwrap();
    assert_eq!(
        broker
            .publish(request("exhausted", "event/max", b"two".to_vec()))
            .await,
        Err(MessagingError::CapacityExceeded {
            resource: "topic sequence",
            limit: usize::MAX,
        })
    );

    broker.pool.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn closed_storage_returns_a_redacted_publication_error() {
    let (path, url) = fixture("publish-closed");
    let broker = SqliteBroker::connect_with_clock(
        url,
        BrokerConfig::try_new("publish-closed").unwrap(),
        FixedClock(123_456),
    )
    .await
    .unwrap();
    broker.pool.close().await;

    assert_eq!(
        broker
            .publish(request("events", "event/closed", b"body".to_vec()))
            .await,
        Err(MessagingError::StorageUnavailable {
            operation: "begin publication",
        })
    );
    cleanup(&path);
}
