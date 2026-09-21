use super::redis_support::*;
use rullst_messaging::*;
use std::time::Duration;

#[tokio::test]
#[ignore = "requires the owned disposable Redis fixture"]
async fn byte_retention_reply_batches_and_expired_attempts_are_bounded() {
    let broker = RedisBroker::provision(configuration_with(
        BrokerConfig::try_new(unique("bytes"))
            .unwrap()
            .with_limits(100, 1, 2, 1024 * 1024)
            .unwrap(),
    ))
    .await
    .unwrap();
    broker
        .subscribe(subscription("files", "one", StartPosition::Earliest))
        .await
        .unwrap();
    assert!(matches!(
        broker
            .subscribe(subscription("files", "two", StartPosition::Earliest))
            .await,
        Err(MessagingError::CapacityExceeded {
            resource: "message subscriptions",
            limit: 1
        })
    ));
    let mut accepted = 0;
    for index in 0..65 {
        let request = PublishRequest::try_new(
            "files",
            "blob.created",
            format!("file:{index}"),
            vec![7; 1024 * 1024],
        )
        .unwrap();
        match broker.publish(request).await {
            Ok(_) => accepted += 1,
            Err(MessagingError::CapacityExceeded {
                resource: "Redis retained envelope bytes",
                limit,
            }) => {
                assert_eq!(limit, 64 * 1024 * 1024);
                break;
            }
            error => panic!("unexpected quota result: {error:?}"),
        }
    }
    assert_eq!(
        accepted, 63,
        "wire overhead also consumes the retained byte budget"
    );
    let batch = broker
        .receive(
            ReceiveRequest::try_new("files", "one", "worker", 100, Duration::from_secs(30))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        batch.len(),
        3,
        "four envelopes would exceed the four-MiB reply budget"
    );
    for delivery in batch {
        broker
            .dead_letter(
                delivery.ack_token(),
                FailureCode::try_new("large.rejected").unwrap(),
            )
            .await
            .unwrap();
    }
    assert_eq!(
        broker
            .dead_letters(DeadLetterQuery::try_new("files", "one", 100).unwrap())
            .await
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        broker
            .purge_terminal(PurgeRequest::try_new("files", 1000).unwrap())
            .await
            .unwrap()
            .removed(),
        3
    );
    broker
        .publish(
            PublishRequest::try_new("files", "blob.created", "after-purge", vec![7; 1024 * 1024])
                .unwrap(),
        )
        .await
        .unwrap();

    let broker = RedisBroker::provision(configuration(&unique("expiry")))
        .await
        .unwrap();
    broker
        .subscribe(subscription("events", "one", StartPosition::Earliest))
        .await
        .unwrap();
    broker
        .publish(publication("events", "expires"))
        .await
        .unwrap();
    let first = broker
        .receive(receive("events", "one", "lost", 1))
        .await
        .unwrap()
        .remove(0);
    assert!(
        broker
            .retry(
                first.ack_token(),
                Duration::from_secs(7 * 86400 + 1),
                FailureCode::try_new("late").unwrap()
            )
            .await
            .is_err()
    );
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let second = broker
        .receive(receive("events", "one", "replacement", 1))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(second.attempt(), 2);
    assert_eq!(
        broker.ack(first.ack_token()).await,
        Err(MessagingError::LeaseNotFound)
    );
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert!(
        broker
            .receive(receive("events", "one", "third", 1))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        broker.ack(second.ack_token()).await,
        Err(MessagingError::LeaseNotFound)
    );
    let dead = broker
        .dead_letters(DeadLetterQuery::try_new("events", "one", 10).unwrap())
        .await
        .unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].failure_code().as_str(), "delivery.max_attempts");
}
