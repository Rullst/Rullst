#![cfg(feature = "sqlite")]
#[allow(dead_code)]
mod sqlite_support;
#[allow(dead_code)]
mod support;
use rullst_messaging::*;
use sqlite_support::{cleanup, config, fixture};
use std::time::Duration;
use support::ManualClock;

#[tokio::test]
async fn lock_wait_cannot_authorize_an_expired_ack_retry_or_dead_letter() {
    for operation in ["ack", "retry", "dead"] {
        let (path, url) = fixture("lease-lock-deadline");
        let clock = ManualClock::new(50_000);
        let broker = SqliteBroker::connect_with_clock(&url, config("leases"), clock.clone())
            .await
            .unwrap();
        broker
            .subscribe(
                SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest).unwrap(),
            )
            .await
            .unwrap();
        broker
            .publish(PublishRequest::try_new("jobs", "ready", "one", []).unwrap())
            .await
            .unwrap();
        let receive = || {
            ReceiveRequest::try_new("jobs", "workers", "worker", 1, Duration::from_secs(1)).unwrap()
        };
        let claimed = broker.receive(receive()).await.unwrap().remove(0);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let lock = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
        let pending = async {
            match operation {
                "ack" => broker.ack(claimed.ack_token()).await,
                "retry" => broker
                    .retry(
                        claimed.ack_token(),
                        Duration::from_secs(1),
                        FailureCode::try_new("fixture").unwrap(),
                    )
                    .await
                    .map(|_| ()),
                _ => {
                    broker
                        .dead_letter(
                            claimed.ack_token(),
                            FailureCode::try_new("fixture").unwrap(),
                        )
                        .await
                }
            }
        };
        tokio::pin!(pending);
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut pending)
                .await
                .is_err()
        );
        clock.advance(1001);
        lock.rollback().await.unwrap();
        assert_eq!(
            pending.await,
            Err(MessagingError::LeaseExpired),
            "{operation}"
        );
        assert_eq!(broker.receive(receive()).await.unwrap()[0].attempt(), 2);
        broker.close().await;
        pool.close().await;
        cleanup(&path);
    }
}

#[tokio::test]
async fn claim_lease_starts_after_lock_acquisition() {
    let (path, url) = fixture("claim-lock-deadline");
    let clock = ManualClock::new(50_000);
    let broker = SqliteBroker::connect_with_clock(&url, config("claims"), clock.clone())
        .await
        .unwrap();
    broker
        .subscribe(
            SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest).unwrap(),
        )
        .await
        .unwrap();
    broker
        .publish(PublishRequest::try_new("jobs", "ready", "one", []).unwrap())
        .await
        .unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let lock = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let pending = broker.receive(
        ReceiveRequest::try_new("jobs", "workers", "worker", 1, Duration::from_secs(1)).unwrap(),
    );
    tokio::pin!(pending);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut pending)
            .await
            .is_err()
    );
    clock.advance(5000);
    lock.rollback().await.unwrap();
    let claimed = pending.await.unwrap().remove(0);
    assert_eq!(claimed.lease_expires_at_ms(), 56_000);
    broker.ack(claimed.ack_token()).await.unwrap();
    broker.close().await;
    pool.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn publication_and_retry_timestamps_use_admitted_time() {
    let (path, url) = fixture("timestamp-lock-deadline");
    let clock = ManualClock::new(50_000);
    let broker = SqliteBroker::connect_with_clock(&url, config("timestamps"), clock.clone())
        .await
        .unwrap();
    broker
        .subscribe(
            SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest).unwrap(),
        )
        .await
        .unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let lock = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let pending = broker.publish(PublishRequest::try_new("jobs", "ready", "one", []).unwrap());
    tokio::pin!(pending);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut pending)
            .await
            .is_err()
    );
    clock.advance(5000);
    lock.rollback().await.unwrap();
    assert_eq!(pending.await.unwrap().published_at_ms(), 55_000);
    let claimed = broker
        .receive(
            ReceiveRequest::try_new("jobs", "workers", "worker", 1, Duration::from_secs(10))
                .unwrap(),
        )
        .await
        .unwrap()
        .remove(0);
    let lock = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let pending = broker.retry(
        claimed.ack_token(),
        Duration::from_secs(2),
        FailureCode::try_new("fixture").unwrap(),
    );
    tokio::pin!(pending);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut pending)
            .await
            .is_err()
    );
    clock.advance(5000);
    lock.rollback().await.unwrap();
    assert_eq!(
        pending.await.unwrap(),
        RetryDisposition::Scheduled {
            available_at_ms: 62_000
        }
    );
    broker.close().await;
    pool.close().await;
    cleanup(&path);
}
