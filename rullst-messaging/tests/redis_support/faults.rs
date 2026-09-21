use super::redis_support::*;
use rullst_messaging::*;
use std::time::Duration;

#[tokio::test]
#[ignore = "requires the owned disposable Redis fixture"]
async fn missing_state_and_partial_mutations_fail_closed() {
    let namespace = unique("corruption");
    let config = configuration(&namespace);
    let broker = RedisBroker::provision(config.clone()).await.unwrap();
    broker
        .subscribe(subscription("orders", "one", StartPosition::Earliest))
        .await
        .unwrap();
    broker
        .subscribe(subscription("orders", "two", StartPosition::Earliest))
        .await
        .unwrap();
    let mut connection = raw().await;
    let base = prefix(&namespace);
    let topic_hash = digest(b"orders");
    let mut groups: Vec<String> = redis::cmd("SMEMBERS")
        .arg(format!("{base}groups:{topic_hash}"))
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(groups.len(), 2);
    groups.sort();
    // Publication sorts its bounded group list before updating the indexes.
    // Corrupt the second index to prove one real mutation preceded the failure.
    let _: () = redis::cmd("SET")
        .arg(format!("{base}ready:{}", groups[1]))
        .arg("wrong-type")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert!(matches!(
        broker.publish(publication("orders", "partial")).await,
        Err(MessagingError::StorageUnavailable { .. })
    ));
    let dirty: String = redis::cmd("HGET")
        .arg(format!("{base}meta"))
        .arg("dirty")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(dirty, "1");
    let partial: usize = redis::cmd("ZCARD")
        .arg(format!("{base}ready:{}", groups[0]))
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(partial, 1, "one write really preceded the failure");
    assert!(matches!(
        broker.publish(publication("orders", "partial")).await,
        Err(MessagingError::CorruptStorage { .. })
    ));
    assert!(matches!(
        RedisBroker::connect(config.clone()).await,
        Err(MessagingError::CorruptStorage { .. })
    ));
    assert!(matches!(
        RedisBroker::provision(config).await,
        Err(MessagingError::CorruptStorage { .. })
    ));

    let namespace = unique("lost");
    let config = configuration(&namespace);
    let broker = RedisBroker::provision(config.clone()).await.unwrap();
    let _: usize = redis::cmd("DEL")
        .arg(format!("{}meta", prefix(&namespace)))
        .query_async(&mut connection)
        .await
        .unwrap();
    assert!(matches!(
        broker.publish(publication("orders", "no-recreate")).await,
        Err(MessagingError::CorruptStorage { .. })
    ));
    assert!(matches!(
        RedisBroker::connect(config.clone()).await,
        Err(MessagingError::CorruptStorage { .. })
    ));
    assert!(matches!(
        RedisBroker::provision(config).await,
        Err(MessagingError::CorruptStorage { .. })
    ));
}

#[tokio::test]
#[ignore = "requires the owned disposable Redis fixture"]
async fn wrong_credentials_timeout_and_clock_regression_never_fall_back() {
    let namespace = unique("faults");
    let config = configuration(&namespace);
    let broker = RedisBroker::provision(config.clone()).await.unwrap();
    let wrong = RedisBrokerConfig::try_new(
        config.broker().clone(),
        "fixture-generation",
        endpoint(),
        "default",
        uuid::Uuid::new_v4().simple().to_string(),
    )
    .unwrap()
    .allow_loopback_for_tests()
    .unwrap();
    assert!(matches!(
        RedisBroker::connect(wrong).await,
        Err(MessagingError::StorageUnavailable { .. })
    ));
    let plain = RedisBrokerConfig::try_new(
        config.broker().clone(),
        "fixture-generation",
        endpoint(),
        "default",
        password(),
    )
    .unwrap();
    assert!(RedisBroker::connect(plain).await.is_err());
    let mut connection = raw().await;
    let _: () = redis::cmd("HSET")
        .arg(format!("{}meta", prefix(&namespace)))
        .arg("clock")
        .arg("8000000000000000")
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(
        broker.publish(publication("orders", "clock")).await,
        Err(MessagingError::ClockOutOfRange)
    );
    let _: () = redis::cmd("HSET")
        .arg(format!("{}meta", prefix(&namespace)))
        .arg("clock")
        .arg("0")
        .query_async(&mut connection)
        .await
        .unwrap();
    let bounded = RedisBroker::connect(config.with_timeout(Duration::from_millis(100)).unwrap())
        .await
        .unwrap();
    let _: () = redis::cmd("CLIENT")
        .arg("PAUSE")
        .arg(350)
        .arg("ALL")
        .query_async(&mut connection)
        .await
        .unwrap();
    let started = std::time::Instant::now();
    assert!(matches!(
        bounded.publish(publication("orders", "ambiguous")).await,
        Err(MessagingError::StorageUnavailable { .. })
    ));
    assert!(started.elapsed() < Duration::from_secs(1));
    tokio::time::sleep(Duration::from_millis(400)).await;
    // Either the first command committed or it did not. Replay converges to one
    // message; a timeout never promises rollback or activates the mock backend.
    let first = broker
        .publish(publication("orders", "ambiguous"))
        .await
        .unwrap();
    let second = broker
        .publish(publication("orders", "ambiguous"))
        .await
        .unwrap();
    assert_eq!(first.id(), second.id());
    assert!(second.is_duplicate());
    assert!(!bounded.is_mock());
}
