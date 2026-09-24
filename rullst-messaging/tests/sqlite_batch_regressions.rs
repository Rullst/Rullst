#![cfg(feature = "sqlite")]

mod sqlite_adversarial_support;
mod sqlite_support;

use rullst_messaging::{
    FailureCode, MessageAdmin, MessageBroker, MessagingError, PublishRequest, PurgeRequest,
    SqliteBroker, StartPosition, SubscriptionRequest,
};
use sqlite_adversarial_support::{ManualClock, receive, subscribe_and_publish};
use sqlite_support::{cleanup, config, fixture};
use std::time::Duration;

#[tokio::test]
async fn a_corrupt_second_message_rolls_back_the_entire_receive_batch() {
    let (path, url) = fixture("batch-rollback");
    let broker = SqliteBroker::connect_with_clock(
        url.clone(),
        config("batch-rollback"),
        ManualClock::new(100_000),
    )
    .await
    .unwrap();
    subscribe_and_publish(&broker, "jobs", "workers", "first").await;
    let second = broker
        .publish(
            PublishRequest::try_new("jobs", "event.ready", "second", b"payload".to_vec()).unwrap(),
        )
        .await
        .unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    // The ordered first delivery is claimed before decoding this second row.
    sqlx::query("UPDATE rullst_messaging_messages SET event_kind = 'bad event' WHERE namespace = ? AND message_id = ?")
        .bind("batch-rollback").bind(second.id().as_str()).execute(&pool).await.unwrap();
    assert_eq!(
        broker.receive(receive("jobs", "workers")).await,
        Err(MessagingError::CorruptStorage {
            context: "message event kind"
        })
    );
    let rows: Vec<(String, i64, Option<String>)> = sqlx::query_as(
        "SELECT state, attempt, ack_token FROM rullst_messaging_deliveries WHERE namespace = ? ORDER BY sequence",
    ).bind("batch-rollback").fetch_all(&pool).await.unwrap();
    assert_eq!(rows, vec![("pending".to_owned(), 0, None); 2]);
    sqlx::query("UPDATE rullst_messaging_messages SET event_kind = 'event.ready' WHERE namespace = ? AND message_id = ?")
        .bind("batch-rollback").bind(second.id().as_str()).execute(&pool).await.unwrap();
    let deliveries = broker.receive(receive("jobs", "workers")).await.unwrap();
    assert_eq!(deliveries.len(), 2);
    assert_eq!(deliveries[1].envelope().id(), second.id());
    for delivery in deliveries {
        assert_eq!(delivery.attempt(), 1);
        broker.ack(delivery.ack_token()).await.unwrap();
    }
    pool.close().await;
    broker.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn purge_requires_every_consumer_to_finish_and_preserves_other_namespaces() {
    let (path, url) = fixture("partial-purge");
    let clock = ManualClock::new(100_000);
    let broker =
        SqliteBroker::connect_with_clock(url.clone(), config("partial-purge"), clock.clone())
            .await
            .unwrap();
    let other = SqliteBroker::connect_with_clock(url, config("other-namespace"), clock)
        .await
        .unwrap();
    subscribe_and_publish(&broker, "jobs", "alpha", "same-key").await;
    subscribe_and_publish(&other, "jobs", "alpha", "same-key").await;
    broker
        .subscribe(SubscriptionRequest::try_new("jobs", "beta", StartPosition::Earliest).unwrap())
        .await
        .unwrap();
    let alpha = broker
        .receive(receive("jobs", "alpha"))
        .await
        .unwrap()
        .remove(0);
    broker.ack(alpha.ack_token()).await.unwrap();
    let purge = || PurgeRequest::try_new("jobs", 10).unwrap();
    assert_eq!(broker.purge_terminal(purge()).await.unwrap().removed(), 0);
    let beta = broker
        .receive(receive("jobs", "beta"))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(beta.envelope().id(), alpha.envelope().id());
    assert_eq!(broker.purge_terminal(purge()).await.unwrap().removed(), 0);
    broker
        .dead_letter(
            beta.ack_token(),
            FailureCode::try_new("handler.rejected").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(broker.purge_terminal(purge()).await.unwrap().removed(), 1);
    assert_eq!(broker.purge_terminal(purge()).await.unwrap().removed(), 0);
    let untouched = other.receive(receive("jobs", "alpha")).await.unwrap();
    assert_eq!(untouched.len(), 1);
    assert_eq!(untouched[0].attempt(), 1);
    assert_eq!(untouched[0].envelope().payload(), b"payload");
    other.ack(untouched[0].ack_token()).await.unwrap();
    broker.close().await;
    other.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn reclaimed_delivery_rejects_every_old_token_operation_without_losing_its_new_lease() {
    let (path, url) = fixture("fenced-operations");
    let clock = ManualClock::new(100_000);
    let broker = SqliteBroker::connect_with_clock(url, config("fenced-operations"), clock.clone())
        .await
        .unwrap();
    subscribe_and_publish(&broker, "jobs", "workers", "one").await;
    let old = broker
        .receive(receive("jobs", "workers"))
        .await
        .unwrap()
        .remove(0);
    clock.advance(1_000);
    let fresh = broker
        .receive(receive("jobs", "workers"))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(fresh.envelope().id(), old.envelope().id());
    assert_eq!(fresh.attempt(), 2);
    assert_eq!(
        broker.ack(old.ack_token()).await,
        Err(MessagingError::LeaseNotFound)
    );
    assert_eq!(
        broker
            .retry(
                old.ack_token(),
                Duration::from_secs(1),
                FailureCode::try_new("handler.retry").unwrap()
            )
            .await,
        Err(MessagingError::LeaseNotFound)
    );
    assert_eq!(
        broker
            .dead_letter(
                old.ack_token(),
                FailureCode::try_new("handler.rejected").unwrap()
            )
            .await,
        Err(MessagingError::LeaseNotFound)
    );
    assert!(
        broker
            .receive(receive("jobs", "workers"))
            .await
            .unwrap()
            .is_empty()
    );
    broker.ack(fresh.ack_token()).await.unwrap();
    clock.advance(1_000);
    assert!(
        broker
            .receive(receive("jobs", "workers"))
            .await
            .unwrap()
            .is_empty()
    );
    broker.close().await;
    cleanup(&path);
}
