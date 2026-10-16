//! Broker-level bounds: closing, unsafe targets, persisted limits, legacy rows
//! and the edges of stored timestamps.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::{
    DeadLetterQuery, MessagingError, MessagingKeyring, MessagingStorageKey, StartPosition,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::path::PathBuf;

#[derive(Clone)]
struct FixedClock(i64);

impl Clock for FixedClock {
    fn now_millis(&self) -> Result<i64> {
        Ok(self.0)
    }
}

struct Fixture {
    path: PathBuf,
    url: String,
}

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rullst-sqlite-broker-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let url = format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"));
        Self { path, url }
    }

    async fn connect(&self, config: BrokerConfig) -> Result<SqliteBroker<FixedClock>> {
        SqliteBroker::connect_with_clock(&self.url, config, FixedClock(1_800_000_000_000)).await
    }

    async fn raw(&self, statements: &[&'static str]) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&self.url)
            .await
            .unwrap();
        for statement in statements {
            sqlx::query(*statement).execute(&pool).await.unwrap();
        }
        pool.close().await;
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.path.display()));
        }
    }
}

fn config(namespace: &str) -> BrokerConfig {
    BrokerConfig::try_new(namespace).unwrap()
}

async fn publish_one<C: Clock>(broker: &SqliteBroker<C>) {
    broker
        .subscribe(
            SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest).unwrap(),
        )
        .await
        .unwrap();
    broker
        .publish(PublishRequest::try_new("jobs", "job.ready", "job-1", b"{}".to_vec()).unwrap())
        .await
        .unwrap();
}

fn receive_request() -> ReceiveRequest {
    ReceiveRequest::try_new("jobs", "workers", "worker", 1, Duration::from_secs(30)).unwrap()
}

#[tokio::test]
async fn closing_one_handle_closes_the_shared_pool() {
    let fixture = Fixture::new();
    let broker = fixture.connect(config("closing")).await.unwrap();
    let clone = broker.clone();
    broker.close().await;
    let request = PublishRequest::try_new("jobs", "job.ready", "after", b"{}".to_vec()).unwrap();
    assert!(matches!(
        clone.publish(request).await,
        Err(MessagingError::StorageUnavailable { .. })
    ));
}

#[tokio::test]
async fn a_directory_target_is_refused_before_connecting() {
    let directory = std::env::temp_dir().join(format!(
        "rullst-sqlite-directory-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir(&directory).unwrap();
    let url = format!(
        "sqlite://{}",
        directory.to_string_lossy().replace('\\', "/")
    );
    let result = SqliteBroker::connect(url, config("directory")).await;
    std::fs::remove_dir(&directory).unwrap();
    assert_eq!(
        result.err(),
        Some(MessagingError::Invalid {
            field: "durable SQLite database URL",
            reason: "existing target must be a regular file",
        })
    );
}

#[cfg(unix)]
#[tokio::test]
async fn an_uninspectable_target_is_a_storage_error_before_connecting() {
    // A regular file used as a directory makes the inspection fail with ENOTDIR.
    let fixture = Fixture::new();
    std::fs::write(&fixture.path, b"").unwrap();
    let url = format!("sqlite://{}/child.sqlite", fixture.path.display());
    assert_eq!(
        SqliteBroker::connect(url, config("not-directory"))
            .await
            .err(),
        Some(MessagingError::StorageUnavailable {
            operation: "inspect database target",
        })
    );
}

#[tokio::test]
async fn every_persisted_limit_must_match_on_reopen() {
    let fixture = Fixture::new();
    let broker = fixture.connect(config("limits")).await.unwrap();
    let defaults = || config("limits");
    for changed in [
        defaults().with_limits(10_000, 64, 5, 1_048_576).unwrap(),
        defaults().with_limits(10_000, 128, 6, 1_048_576).unwrap(),
        defaults().with_limits(10_000, 128, 5, 1_048_575).unwrap(),
    ] {
        assert_eq!(
            fixture.connect(changed).await.err(),
            Some(MessagingError::ConfigurationConflict)
        );
    }
    assert!(fixture.connect(defaults()).await.is_ok());
    broker.close().await;
}

#[tokio::test]
async fn rows_written_before_storage_profiles_stay_plaintext() {
    let fixture = Fixture::new();
    let broker = fixture.connect(config("legacy")).await.unwrap();
    publish_one(&broker).await;
    broker.close().await;
    // A database from before storage profiles has rows but no profile record.
    fixture
        .raw(&["DELETE FROM rullst_messaging_storage_profiles WHERE namespace = 'legacy'"])
        .await;
    let keyring = MessagingKeyring::new(MessagingStorageKey::try_new("storage", [71; 32]).unwrap());
    assert_eq!(
        SqliteBroker::connect_encrypted_with_clock(
            &fixture.url,
            config("legacy"),
            keyring,
            FixedClock(1_800_000_000_000),
        )
        .await
        .err(),
        Some(MessagingError::ConfigurationConflict)
    );
    let reopened = fixture.connect(config("legacy")).await.unwrap();
    assert_eq!(reopened.receive(receive_request()).await.unwrap().len(), 1);
    reopened.close().await;
}

#[tokio::test]
async fn messages_and_dead_letters_at_time_zero_are_valid() {
    let fixture = Fixture::new();
    let broker = SqliteBroker::connect_with_clock(&fixture.url, config("zero"), FixedClock(0))
        .await
        .unwrap();
    publish_one(&broker).await;
    let delivery = broker
        .receive(receive_request())
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(delivery.envelope().published_at_ms(), 0);
    let code = FailureCode::try_new("handler.failed").unwrap();
    broker
        .dead_letter(delivery.ack_token(), code)
        .await
        .unwrap();
    let dead = broker
        .dead_letters(DeadLetterQuery::try_new("jobs", "workers", 1).unwrap())
        .await
        .unwrap();
    assert_eq!(dead[0].dead_lettered_at_ms(), 0);
    assert_eq!(dead[0].envelope().published_at_ms(), 0);
    broker.close().await;
}

#[tokio::test]
async fn a_stored_negative_publication_time_is_corruption() {
    let fixture = Fixture::new();
    let broker = fixture.connect(config("negative")).await.unwrap();
    publish_one(&broker).await;
    fixture
        .raw(&[
            "PRAGMA ignore_check_constraints = ON",
            "UPDATE rullst_messaging_messages SET published_at_ms = -1",
        ])
        .await;
    assert_eq!(
        broker.receive(receive_request()).await.err(),
        Some(MessagingError::CorruptStorage {
            context: "message bounds",
        })
    );
    broker.close().await;
}
