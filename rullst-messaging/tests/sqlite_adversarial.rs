#![cfg(feature = "sqlite")]

mod sqlite_adversarial_support;
mod sqlite_support;

use rullst_messaging::{
    DeadLetterQuery, FailureCode, MessageAdmin, MessageBroker, MessagingError, PublishRequest,
    SqliteBroker, StartPosition, SubscriptionRequest,
};
use sqlite_adversarial_support::{ManualClock, receive, subscribe_and_publish};
use sqlite_support::{cleanup, config, config_with_limits, fixture};
use std::time::Duration;

#[tokio::test]
async fn durable_boundaries_reject_invalid_time_unknown_groups_and_retry_delays() {
    let (path, url) = fixture("durable-boundaries");
    let clock = ManualClock::new(-1);
    let broker = SqliteBroker::connect_with_clock(url, config("durable-boundaries"), clock.clone())
        .await
        .expect("open broker");
    assert_eq!(broker.config().namespace().as_str(), "durable-boundaries");

    let request = PublishRequest::try_new("jobs", "job.ready", "job/1", b"work".to_vec())
        .expect("publication");
    assert_eq!(
        broker.publish(request).await,
        Err(MessagingError::ClockOutOfRange)
    );
    clock.advance(1_001);
    assert_eq!(
        broker.receive(receive("jobs", "missing")).await,
        Err(MessagingError::SubscriptionNotFound)
    );

    subscribe_and_publish(&broker, "jobs", "workers", "job/2").await;
    let delivery = broker
        .receive(receive("jobs", "workers"))
        .await
        .expect("claim")
        .pop()
        .expect("delivery");
    let failure = FailureCode::try_new("handler.retry").expect("failure code");
    assert!(
        matches!(
            broker
                .retry(delivery.ack_token(), Duration::MAX, failure.clone())
                .await,
            Err(MessagingError::Invalid {
                field: "retry delay",
                ..
            })
        ),
        "an unrepresentable retry delay must fail before consuming the lease"
    );
    assert!(matches!(
        broker
            .retry(
                delivery.ack_token(),
                Duration::from_secs(7 * 24 * 60 * 60 + 1),
                failure,
            )
            .await,
        Err(MessagingError::Invalid {
            field: "retry delay",
            ..
        })
    ));
    broker
        .dead_letter(
            delivery.ack_token(),
            FailureCode::try_new("handler.rejected").expect("failure code"),
        )
        .await
        .expect("dead letter with still-valid lease");
    assert_eq!(
        broker
            .dead_letter(
                delivery.ack_token(),
                FailureCode::try_new("handler.rejected").expect("failure code"),
            )
            .await,
        Err(MessagingError::LeaseNotFound)
    );
    drop(broker);
    cleanup(&path);

    let (overflow_path, overflow_url) = fixture("lease-overflow");
    let overflow = SqliteBroker::connect_with_clock(
        overflow_url,
        config("lease-overflow"),
        ManualClock::new(i64::MAX),
    )
    .await
    .expect("open overflow broker");
    subscribe_and_publish(&overflow, "jobs", "workers", "job/max").await;
    assert_eq!(
        overflow.receive(receive("jobs", "workers")).await,
        Err(MessagingError::ClockOutOfRange)
    );
    drop(overflow);
    cleanup(&overflow_path);
}

#[tokio::test]
async fn expired_operations_requeue_then_dead_letter_at_the_attempt_ceiling() {
    let (path, url) = fixture("expired-operations");
    let clock = ManualClock::new(100_000);
    let broker = SqliteBroker::connect_with_clock(url, config("expired-operations"), clock.clone())
        .await
        .expect("open broker");
    subscribe_and_publish(&broker, "jobs", "workers", "job/expired").await;

    let first = broker
        .receive(receive("jobs", "workers"))
        .await
        .expect("first claim")
        .pop()
        .expect("first delivery");
    clock.advance(1_000);
    assert_eq!(
        broker
            .retry(
                first.ack_token(),
                Duration::ZERO,
                FailureCode::try_new("handler.transient").expect("failure code"),
            )
            .await,
        Err(MessagingError::LeaseExpired)
    );

    let second = broker
        .receive(receive("jobs", "workers"))
        .await
        .expect("second claim")
        .pop()
        .expect("second delivery");
    assert_eq!(second.attempt(), 2);
    clock.advance(1_000);
    assert_eq!(
        broker
            .dead_letter(
                second.ack_token(),
                FailureCode::try_new("handler.rejected").expect("failure code"),
            )
            .await,
        Err(MessagingError::LeaseExpired)
    );

    let third = broker
        .receive(receive("jobs", "workers"))
        .await
        .expect("third claim")
        .pop()
        .expect("third delivery");
    assert_eq!(third.attempt(), 3);
    clock.advance(1_000);
    assert_eq!(
        broker.ack(third.ack_token()).await,
        Err(MessagingError::LeaseExpired)
    );
    let dead = broker
        .dead_letters(DeadLetterQuery::try_new("jobs", "workers", 10).expect("dead-letter query"))
        .await
        .expect("dead letters");
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].attempts(), 3);
    assert_eq!(dead[0].failure_code().as_str(), "delivery.max_attempts");
    drop(broker);
    cleanup(&path);
}

#[tokio::test]
async fn durable_capacity_and_exhausted_pending_state_fail_closed() {
    let (path, url) = fixture("durable-capacity");
    let broker = SqliteBroker::connect_with_clock(
        url.clone(),
        config_with_limits("durable-capacity", 1, 1, 3, 4 * 1024),
        ManualClock::new(200_000),
    )
    .await
    .expect("open broker");
    broker
        .subscribe(
            SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("first subscription");
    assert_eq!(
        broker
            .subscribe(
                SubscriptionRequest::try_new("audit", "readers", StartPosition::Earliest)
                    .expect("second subscription"),
            )
            .await,
        Err(MessagingError::CapacityExceeded {
            resource: "message subscriptions",
            limit: 1,
        })
    );
    broker
        .publish(
            PublishRequest::try_new("jobs", "job.ready", "job/1", b"one".to_vec())
                .expect("first publication"),
        )
        .await
        .expect("first publish");
    assert_eq!(
        broker
            .publish(
                PublishRequest::try_new("jobs", "job.ready", "job/2", b"two".to_vec())
                    .expect("second publication"),
            )
            .await,
        Err(MessagingError::CapacityExceeded {
            resource: "retained messages",
            limit: 1,
        })
    );

    let repair_pool = sqlx::SqlitePool::connect(&url)
        .await
        .expect("open repair pool");
    sqlx::query(
        "UPDATE rullst_messaging_deliveries SET attempt = 3 WHERE namespace = ? AND topic = ?",
    )
    .bind("durable-capacity")
    .bind("jobs")
    .execute(&repair_pool)
    .await
    .expect("inject exhausted pending state");
    assert!(
        broker
            .receive(receive("jobs", "workers"))
            .await
            .expect("repair exhausted delivery")
            .is_empty()
    );
    let dead = broker
        .dead_letters(DeadLetterQuery::try_new("jobs", "workers", 10).expect("dead-letter query"))
        .await
        .expect("dead letters");
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].failure_code().as_str(), "delivery.max_attempts");
    repair_pool.close().await;
    drop(broker);
    cleanup(&path);
}
