#![cfg(feature = "sqlite")]

mod sqlite_adversarial_support;
mod sqlite_support;

use rullst_messaging::{
    MessageBroker, PublishRequest, SqliteBroker, StartPosition, SubscriptionRequest,
};
use sqlite_adversarial_support::{ManualClock, receive, subscribe_and_publish};
use sqlite_support::{cleanup, config, fixture};

#[tokio::test]
async fn epoch_zero_publication_retains_its_timestamp_and_identity_on_restart_replay() {
    let (path, url) = fixture("epoch-zero-replay");
    let clock = ManualClock::new(0);
    let broker = SqliteBroker::connect_with_clock(url.clone(), config("epoch-zero"), clock.clone())
        .await
        .unwrap();
    let request = PublishRequest::try_new("jobs", "job.ready", "one", b"payload".to_vec()).unwrap();
    let first = broker.publish(request.clone()).await.unwrap();
    assert!(!first.is_duplicate());
    assert_eq!(first.published_at_ms(), 0);
    broker.close().await;
    clock.advance(1_000);
    let reopened = SqliteBroker::connect_with_clock(url, config("epoch-zero"), clock)
        .await
        .unwrap();
    let replay = reopened.publish(request).await.unwrap();
    assert!(replay.is_duplicate());
    assert_eq!(replay.id(), first.id());
    assert_eq!(replay.published_at_ms(), 0);
    reopened.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn new_groups_backfill_retained_history_only_for_earliest_with_scoped_fanout() {
    let (path, url) = fixture("history-before-subscription");
    let clock = ManualClock::new(100_000);
    let broker = SqliteBroker::connect_with_clock(url.clone(), config("school-a"), clock.clone())
        .await
        .unwrap();
    let other = SqliteBroker::connect_with_clock(url, config("school-b"), clock)
        .await
        .unwrap();
    subscribe_and_publish(&other, "jobs", "earliest", "one").await;
    subscribe_and_publish(&broker, "other-topic", "earliest", "one").await;
    let mut ids = Vec::new();
    for key in ["one", "two"] {
        let receipt = broker
            .publish(
                PublishRequest::try_new("jobs", "job.ready", key, key.as_bytes().to_vec()).unwrap(),
            )
            .await
            .unwrap();
        ids.push(receipt.id().clone());
    }
    for (group, position, pending) in [
        ("earliest", StartPosition::Earliest, 2),
        ("latest", StartPosition::Latest, 0),
    ] {
        let subscription = SubscriptionRequest::try_new("jobs", group, position).unwrap();
        let created = broker.subscribe(subscription.clone()).await.unwrap();
        assert!(created.was_created());
        assert_eq!(created.pending_messages(), pending);
        let repeated = broker.subscribe(subscription).await.unwrap();
        assert!(!repeated.was_created());
        assert_eq!(repeated.pending_messages(), pending);
    }
    let backlog = broker.receive(receive("jobs", "earliest")).await.unwrap();
    assert_eq!(
        backlog
            .iter()
            .map(|d| d.envelope().id().clone())
            .collect::<Vec<_>>(),
        ids
    );
    assert!(
        broker
            .receive(receive("jobs", "latest"))
            .await
            .unwrap()
            .is_empty()
    );
    for delivery in &backlog {
        broker.ack(delivery.ack_token()).await.unwrap();
    }
    let later = broker
        .publish(PublishRequest::try_new("jobs", "job.ready", "three", b"later".to_vec()).unwrap())
        .await
        .unwrap();
    for group in ["earliest", "latest"] {
        let deliveries = broker.receive(receive("jobs", group)).await.unwrap();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].envelope().id(), later.id());
        assert_eq!(deliveries[0].attempt(), 1);
        broker.ack(deliveries[0].ack_token()).await.unwrap();
    }
    // Neither the other topic nor the same topic in another namespace was consumed.
    assert_eq!(
        broker
            .receive(receive("other-topic", "earliest"))
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        other
            .receive(receive("jobs", "earliest"))
            .await
            .unwrap()
            .len(),
        1
    );
    other.close().await;
    broker.close().await;
    cleanup(&path);
}
