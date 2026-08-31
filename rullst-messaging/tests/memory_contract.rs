mod support;

use rullst_messaging::{
    BrokerConfig, DeadLetterQuery, FailureCode, InMemoryBroker, MessageAdmin, MessageBroker,
    MessagingError, PublishRequest, PurgeRequest, ReceiveRequest, RetryDisposition, StartPosition,
    SubscriptionRequest,
};
use std::time::Duration;
use support::{ManualClock, run_core_contract};

fn broker_with_limits(
    clock: ManualClock,
    retained: usize,
    attempts: u32,
) -> InMemoryBroker<ManualClock> {
    let config = BrokerConfig::try_new("test-suite")
        .expect("valid config")
        .with_limits(retained, 16, attempts, 1024)
        .expect("valid limits");
    InMemoryBroker::with_clock(config, clock)
}

#[tokio::test]
async fn in_memory_broker_passes_the_shared_core_contract() {
    let broker = broker_with_limits(ManualClock::new(10_000), 16, 5);
    run_core_contract(&broker).await;
}

#[tokio::test]
async fn leases_retry_on_trusted_time_and_dead_letter_at_the_attempt_ceiling() {
    let clock = ManualClock::new(100_000);
    let broker = broker_with_limits(clock.clone(), 8, 3);
    broker
        .subscribe(
            SubscriptionRequest::try_new("events", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    broker
        .publish(
            PublishRequest::try_new("events", "job.ready", "job/1", b"payload".to_vec())
                .expect("publish request"),
        )
        .await
        .expect("publish");

    let receive = || {
        ReceiveRequest::try_new("events", "workers", "worker-1", 1, Duration::from_secs(1))
            .expect("receive request")
    };
    let first = broker.receive(receive()).await.expect("first receive");
    assert_eq!(first[0].attempt(), 1);
    clock.advance(1_000);
    assert_eq!(
        broker.ack(first[0].ack_token()).await,
        Err(MessagingError::LeaseExpired)
    );

    let second = broker.receive(receive()).await.expect("second receive");
    assert_eq!(second[0].attempt(), 2);
    let retry = broker
        .retry(
            second[0].ack_token(),
            Duration::from_secs(2),
            FailureCode::try_new("handler.transient").expect("failure code"),
        )
        .await
        .expect("retry");
    assert_eq!(
        retry,
        RetryDisposition::Scheduled {
            available_at_ms: 103_000
        }
    );
    assert!(broker.receive(receive()).await.expect("not due").is_empty());
    clock.advance(1_999);
    assert!(
        broker
            .receive(receive())
            .await
            .expect("still not due")
            .is_empty()
    );
    clock.advance(1);

    let third = broker.receive(receive()).await.expect("third receive");
    assert_eq!(third[0].attempt(), 3);
    assert_eq!(
        broker
            .retry(
                third[0].ack_token(),
                Duration::ZERO,
                FailureCode::try_new("handler.transient").expect("failure code"),
            )
            .await
            .expect("terminal retry"),
        RetryDisposition::DeadLettered
    );
    assert_eq!(
        broker.ack(third[0].ack_token()).await,
        Err(MessagingError::LeaseNotFound)
    );
    let dead = broker
        .dead_letters(DeadLetterQuery::try_new("events", "workers", 10).expect("query"))
        .await
        .expect("dead letters");
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].attempts(), 3);
    assert_eq!(dead[0].failure_code().as_str(), "handler.transient");
}

#[tokio::test]
async fn latest_groups_skip_history_and_capacity_recovers_only_after_explicit_purge() {
    let clock = ManualClock::new(500_000);
    let broker = broker_with_limits(clock, 1, 3);
    let first = PublishRequest::try_new("audit", "entry.created", "entry/1", b"one".to_vec())
        .expect("first");
    broker.publish(first).await.expect("first publish");
    broker
        .subscribe(
            SubscriptionRequest::try_new("audit", "latest", StartPosition::Latest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    let receive = ReceiveRequest::try_new("audit", "latest", "worker", 10, Duration::from_secs(10))
        .expect("receive");
    assert!(
        broker
            .receive(receive.clone())
            .await
            .expect("history")
            .is_empty()
    );

    let second = PublishRequest::try_new("audit", "entry.created", "entry/2", b"two".to_vec())
        .expect("second");
    assert_eq!(
        broker.publish(second.clone()).await,
        Err(MessagingError::CapacityExceeded {
            resource: "retained messages",
            limit: 1,
        })
    );
    let purge = broker
        .purge_terminal(PurgeRequest::try_new("audit", 1).expect("purge"))
        .await
        .expect("purge skipped history");
    assert_eq!(purge.removed(), 1);
    broker.publish(second).await.expect("publish after purge");
    let delivery = broker.receive(receive).await.expect("new delivery");
    assert_eq!(delivery.len(), 1);
    assert_eq!(delivery[0].envelope().payload(), b"two");
}

#[tokio::test]
async fn concurrent_exact_publication_and_competing_consumers_do_not_duplicate_state() {
    let broker = broker_with_limits(ManualClock::new(900_000), 32, 3);
    broker
        .subscribe(
            SubscriptionRequest::try_new("race", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    let request = PublishRequest::try_new("race", "event.ready", "event/1", b"value".to_vec())
        .expect("request");

    let mut tasks = Vec::new();
    for _ in 0..32 {
        let broker = broker.clone();
        let request = request.clone();
        tasks.push(tokio::spawn(async move { broker.publish(request).await }));
    }
    let mut ids = Vec::new();
    let mut originals = 0usize;
    for task in tasks {
        let receipt = task.await.expect("task join").expect("publish");
        if !receipt.is_duplicate() {
            originals += 1;
        }
        ids.push(receipt.id().as_str().to_string());
    }
    assert_eq!(originals, 1);
    assert!(ids.windows(2).all(|pair| pair[0] == pair[1]));

    let first_request =
        ReceiveRequest::try_new("race", "workers", "worker-a", 1, Duration::from_secs(30))
            .expect("receive a");
    let second_request =
        ReceiveRequest::try_new("race", "workers", "worker-b", 1, Duration::from_secs(30))
            .expect("receive b");
    let (first, second) = tokio::join!(
        broker.receive(first_request),
        broker.receive(second_request)
    );
    assert_eq!(
        first.expect("first claim").len() + second.expect("second claim").len(),
        1
    );
}

#[tokio::test]
async fn concurrent_acknowledgements_consume_the_lease_exactly_once() {
    let broker = broker_with_limits(ManualClock::new(950_000), 8, 3);
    broker
        .subscribe(
            SubscriptionRequest::try_new("acks", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    broker
        .publish(
            PublishRequest::try_new("acks", "event.ready", "event/ack", b"value".to_vec())
                .expect("request"),
        )
        .await
        .expect("publish");
    let delivery = broker
        .receive(
            ReceiveRequest::try_new("acks", "workers", "worker-a", 1, Duration::from_secs(30))
                .expect("receive"),
        )
        .await
        .expect("delivery")
        .pop()
        .expect("one delivery");

    let first_broker = broker.clone();
    let first_token = delivery.ack_token().clone();
    let first = tokio::spawn(async move { first_broker.ack(&first_token).await });
    let second_broker = broker.clone();
    let second_token = delivery.ack_token().clone();
    let second = tokio::spawn(async move { second_broker.ack(&second_token).await });
    let first = first.await.expect("first join");
    let second = second.await.expect("second join");

    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let failure = if let Err(error) = first {
        error
    } else {
        second.expect_err("the other acknowledgement must lose")
    };
    assert_eq!(failure, MessagingError::LeaseNotFound);
}
