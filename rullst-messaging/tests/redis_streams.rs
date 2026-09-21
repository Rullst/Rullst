#![cfg(feature = "redis-streams")]

#[path = "redis_support/bounds.rs"]
mod bounds;
#[path = "redis_support/faults.rs"]
mod faults;
#[cfg(feature = "orm-outbox")]
#[path = "redis_support/outbox.rs"]
mod outbox;
mod redis_support;
#[allow(dead_code)]
mod support;
#[path = "redis_support/tls.rs"]
mod tls;

use redis_support::*;
use rullst_messaging::*;
use std::time::Duration;

#[tokio::test]
async fn explicit_mock_runs_the_shared_contract_without_network() {
    let config = RedisBrokerConfig::try_new(
        BrokerConfig::try_new("offline").unwrap(),
        "deployment-v1",
        "rediss://invalid.example",
        "",
        "",
    )
    .unwrap();
    assert!(config.clone().require_production().is_err());
    assert!(!format!("{config:?}").contains("invalid.example"));
    let broker = RedisBroker::connect(config).await.unwrap();
    assert!(broker.is_mock());
    support::run_core_contract(&broker).await;
}

#[test]
fn configuration_rejects_unsafe_endpoints_credentials_and_limits() {
    for endpoint in [
        "redis://remote.example",
        "redis://localhost",
        "rediss://user:secret@host",
        "rediss://host/1",
        "rediss://host#insecure",
        "rediss://host?insecure=true",
        "https://host",
        "rediss://host:0",
        "rediss://host/path",
    ] {
        assert!(
            RedisBrokerConfig::try_new(
                BrokerConfig::try_new("test").unwrap(),
                "v1",
                endpoint,
                "user",
                "fixture-only"
            )
            .is_err()
        );
    }
    for username in ["", "mock_user", "line\nbreak"] {
        assert!(
            RedisBrokerConfig::try_new(
                BrokerConfig::try_new("test").unwrap(),
                "v1",
                "rediss://host",
                username,
                "fixture-only"
            )
            .is_err()
        );
    }
    let config = RedisBrokerConfig::try_new(
        BrokerConfig::try_new("test").unwrap(),
        "v1",
        "rediss://host",
        "user",
        "fixture-only",
    )
    .unwrap();
    assert!(config.clone().with_timeout(Duration::ZERO).is_err());
    assert!(
        config
            .clone()
            .with_timeout(Duration::from_secs(31))
            .is_err()
    );
    assert!(
        config
            .clone()
            .require_production()
            .unwrap()
            .allow_loopback_for_tests()
            .is_err()
    );
    assert!(!format!("{config:?}").contains("fixture-only"));
    assert!(
        RedisBrokerConfig::try_new(
            BrokerConfig::try_new("test")
                .unwrap()
                .with_limits(10_001, 2, 2, 10)
                .unwrap(),
            "v1",
            "rediss://host",
            "user",
            "fixture-only"
        )
        .is_err()
    );
}

#[tokio::test]
#[ignore = "requires the owned disposable Redis fixture"]
async fn actual_redis_contract_concurrency_and_fenced_redelivery() {
    let config = configuration(&unique("contract"));
    assert!(matches!(
        RedisBroker::connect(config.clone()).await,
        Err(MessagingError::CorruptStorage { .. })
    ));
    let broker = RedisBroker::provision(config.clone()).await.unwrap();
    assert!(!broker.is_mock());
    support::run_core_contract(&broker).await;
    let other = RedisBroker::connect(config.clone()).await.unwrap();
    let request = publication("concurrent", "one");
    let (first, second) = tokio::join!(broker.publish(request.clone()), other.publish(request));
    let (first, second) = (first.unwrap(), second.unwrap());
    assert_eq!(first.id(), second.id());
    assert_ne!(first.is_duplicate(), second.is_duplicate());
    broker
        .subscribe(subscription(
            "concurrent",
            "shared",
            StartPosition::Earliest,
        ))
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        broker.receive(receive("concurrent", "shared", "a", 1)),
        other.receive(receive("concurrent", "shared", "b", 1))
    );
    let mut deliveries = a.unwrap();
    deliveries.extend(b.unwrap());
    assert_eq!(deliveries.len(), 1);
    let lease = deliveries.remove(0);
    let subscribed_again = broker
        .subscribe(subscription(
            "concurrent",
            "shared",
            StartPosition::Earliest,
        ))
        .await
        .unwrap();
    assert!(!subscribed_again.was_created());
    assert_eq!(
        subscribed_again.pending_messages(),
        0,
        "leased messages are not counted as pending"
    );
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert_eq!(
        other.ack(lease.ack_token()).await,
        Err(MessagingError::LeaseExpired)
    );
    assert_eq!(
        broker.ack(lease.ack_token()).await,
        Err(MessagingError::LeaseNotFound)
    );
    let fresh = other
        .receive(receive("concurrent", "shared", "c", 1))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(fresh.attempt(), 2);
    assert_eq!(fresh.envelope().id(), first.id());
    assert_eq!(fresh.envelope().published_at_ms(), first.published_at_ms());
    assert_ne!(fresh.ack_token(), lease.ack_token());
    assert_eq!(
        other
            .retry(
                fresh.ack_token(),
                Duration::ZERO,
                FailureCode::try_new("temporary").unwrap()
            )
            .await
            .unwrap(),
        RetryDisposition::DeadLettered
    );
    assert_eq!(
        other
            .dead_letters(DeadLetterQuery::try_new("concurrent", "shared", 10).unwrap())
            .await
            .unwrap()[0]
            .attempts(),
        2
    );
    let altered = RedisBrokerConfig::try_new(
        config.broker().clone(),
        "wrong-generation",
        endpoint(),
        "default",
        PASSWORD,
    )
    .unwrap()
    .allow_loopback_for_tests()
    .unwrap();
    assert!(matches!(
        RedisBroker::connect(altered).await,
        Err(MessagingError::ConfigurationConflict)
    ));
    let isolated = RedisBroker::provision(configuration(&unique("isolated")))
        .await
        .unwrap();
    assert_eq!(
        isolated.ack(fresh.ack_token()).await,
        Err(MessagingError::LeaseNotFound)
    );
}

#[tokio::test]
#[ignore = "requires the owned disposable Redis fixture"]
async fn actual_redis_latest_retry_purge_and_bounded_capacity() {
    let namespace = unique("retention");
    let limits = BrokerConfig::try_new(&namespace)
        .unwrap()
        .with_limits(3, 3, 2, 64)
        .unwrap();
    let broker = RedisBroker::provision(configuration_with(limits))
        .await
        .unwrap();
    broker.publish(publication("topic", "old")).await.unwrap();
    assert_eq!(
        broker
            .purge_terminal(PurgeRequest::try_new("topic", 100).unwrap())
            .await
            .unwrap()
            .removed(),
        0
    );
    let receipt = broker
        .subscribe(subscription("topic", "latest", StartPosition::Latest))
        .await
        .unwrap();
    assert_eq!(receipt.pending_messages(), 0);
    assert!(
        broker
            .receive(receive("topic", "latest", "worker", 10))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        broker
            .purge_terminal(PurgeRequest::try_new("topic", 100).unwrap())
            .await
            .unwrap()
            .removed(),
        1
    );
    let request = publication("topic", "new");
    broker.publish(request.clone()).await.unwrap();
    let earliest = broker
        .subscribe(subscription("topic", "early", StartPosition::Earliest))
        .await
        .unwrap();
    assert_eq!(earliest.pending_messages(), 1);
    let first = broker
        .receive(receive("topic", "latest", "a", 1))
        .await
        .unwrap()
        .remove(0);
    let retried = broker
        .retry(
            first.ack_token(),
            Duration::from_millis(100),
            FailureCode::try_new("transient").unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(retried, RetryDisposition::Scheduled { .. }));
    assert!(
        broker
            .receive(receive("topic", "latest", "a", 1))
            .await
            .unwrap()
            .is_empty()
    );
    tokio::time::sleep(Duration::from_millis(150)).await;
    let next = broker
        .receive(receive("topic", "latest", "a", 1))
        .await
        .unwrap()
        .remove(0);
    broker.ack(next.ack_token()).await.unwrap();
    assert_eq!(
        broker
            .purge_terminal(PurgeRequest::try_new("topic", 100).unwrap())
            .await
            .unwrap()
            .removed(),
        0
    );
    let early = broker
        .receive(receive("topic", "early", "b", 1))
        .await
        .unwrap()
        .remove(0);
    broker.ack(early.ack_token()).await.unwrap();
    assert_eq!(
        broker
            .purge_terminal(PurgeRequest::try_new("topic", 100).unwrap())
            .await
            .unwrap()
            .removed(),
        1
    );
    assert!(!broker.publish(request).await.unwrap().is_duplicate());
    broker.publish(publication("other", "two")).await.unwrap();
    broker.publish(publication("other", "three")).await.unwrap();
    assert!(matches!(
        broker.publish(publication("other", "four")).await,
        Err(MessagingError::CapacityExceeded {
            resource: "retained messages",
            ..
        })
    ));
    assert!(matches!(
        broker
            .publish(PublishRequest::try_new("other", "event", "big", vec![0; 65]).unwrap())
            .await,
        Err(MessagingError::CapacityExceeded { .. })
    ));
}

#[tokio::test]
#[ignore = "requires two phases around an actual Redis process restart"]
async fn actual_redis_restart_retains_receipts_groups_and_unacked_messages() {
    let file = std::path::PathBuf::from(
        std::env::var("RULLST_REDIS_RESTART_RECEIPT").expect("owned receipt path"),
    );
    let phase = std::env::var("RULLST_REDIS_RESTART_PHASE").expect("explicit phase");
    if phase == "exercise" {
        let namespace = unique("restart");
        let broker = RedisBroker::provision(configuration(&namespace))
            .await
            .unwrap();
        broker
            .subscribe(subscription("orders", "worker", StartPosition::Earliest))
            .await
            .unwrap();
        let receipt = broker
            .publish(publication("orders", "survive"))
            .await
            .unwrap();
        let pending = broker
            .receive(receive("orders", "worker", "before", 1))
            .await
            .unwrap()
            .remove(0);
        assert_eq!(pending.attempt(), 1);
        std::fs::write(file, format!("{namespace}\n{}", receipt.id().as_str())).unwrap();
    } else {
        assert_eq!(phase, "restart");
        let stored = std::fs::read_to_string(file).unwrap();
        let (namespace, id) = stored.split_once('\n').unwrap();
        let broker = RedisBroker::connect(configuration(namespace))
            .await
            .unwrap();
        let receipt = broker
            .publish(publication("orders", "survive"))
            .await
            .unwrap();
        assert!(receipt.is_duplicate());
        assert_eq!(receipt.id().as_str(), id);
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let redelivered = broker
            .receive(receive("orders", "worker", "after", 1))
            .await
            .unwrap()
            .remove(0);
        assert_eq!(redelivered.attempt(), 2);
        assert_eq!(redelivered.envelope().id().as_str(), id);
        broker.ack(redelivered.ack_token()).await.unwrap();
        assert!(
            broker
                .receive(receive("orders", "worker", "after", 1))
                .await
                .unwrap()
                .is_empty()
        );
    }
}
