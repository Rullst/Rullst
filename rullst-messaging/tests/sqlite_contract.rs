#![cfg(feature = "sqlite")]

mod sqlite_support;
mod support;

use rullst_messaging::{
    BrokerConfig, DeadLetterQuery, FailureCode, MessageAdmin, MessageBroker, MessagingError,
    PublishRequest, PurgeRequest, ReceiveRequest, RetryDisposition, SqliteBroker, StartPosition,
    SubscriptionRequest,
};
use sqlite_support::{cleanup, config, fixture};
use std::time::Duration;
use support::{ManualClock, run_core_contract};

#[tokio::test]
async fn sqlite_broker_passes_the_shared_contract() {
    let (path, url) = fixture("shared-contract");
    let broker =
        SqliteBroker::connect_with_clock(url, config("sqlite-contract"), ManualClock::new(10_000))
            .await
            .expect("open SQLite broker");
    run_core_contract(&broker).await;
    drop(broker);
    cleanup(&path);
}

#[tokio::test]
async fn restart_preserves_idempotency_leases_attempts_and_acknowledgement() {
    let (path, url) = fixture("restart");
    let clock = ManualClock::new(100_000);
    let broker =
        SqliteBroker::connect_with_clock(url.clone(), config("restart-contract"), clock.clone())
            .await
            .expect("open first broker");
    broker
        .subscribe(
            SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    let request = PublishRequest::try_new("jobs", "job.ready", "job/7", b"work".to_vec())
        .expect("publication");
    let first_receipt = broker
        .publish(request.clone())
        .await
        .expect("publish before restart");
    let receive = || {
        ReceiveRequest::try_new("jobs", "workers", "worker-a", 1, Duration::from_secs(1))
            .expect("receive request")
    };
    let first = broker.receive(receive()).await.expect("first claim");
    assert_eq!(first[0].attempt(), 1);
    drop(broker);

    clock.advance(1_000);
    let reopened =
        SqliteBroker::connect_with_clock(url.clone(), config("restart-contract"), clock.clone())
            .await
            .expect("reopen broker");
    assert_eq!(
        reopened.ack(first[0].ack_token()).await,
        Err(MessagingError::LeaseExpired)
    );
    let second = reopened.receive(receive()).await.expect("second claim");
    assert_eq!(second[0].attempt(), 2);
    reopened
        .ack(second[0].ack_token())
        .await
        .expect("ack after restart");
    let replay = reopened.publish(request).await.expect("idempotent replay");
    assert!(replay.is_duplicate());
    assert_eq!(replay.id(), first_receipt.id());
    drop(reopened);

    let final_broker = SqliteBroker::connect_with_clock(url, config("restart-contract"), clock)
        .await
        .expect("final reopen");
    assert!(
        final_broker
            .receive(receive())
            .await
            .expect("acked message remains terminal")
            .is_empty()
    );
    drop(final_broker);
    cleanup(&path);
}

#[tokio::test]
async fn multiple_instances_serialize_publication_and_competing_claims() {
    let (path, url) = fixture("concurrency");
    let clock = ManualClock::new(200_000);
    let first = SqliteBroker::connect_with_clock(
        url.clone(),
        config("concurrency-contract"),
        clock.clone(),
    )
    .await
    .expect("open first broker");
    let second = SqliteBroker::connect_with_clock(url, config("concurrency-contract"), clock)
        .await
        .expect("open second broker");
    first
        .subscribe(
            SubscriptionRequest::try_new("events", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    let request =
        PublishRequest::try_new("events", "event.ready", "event/one", b"payload".to_vec())
            .expect("publication");
    let mut tasks = Vec::new();
    for index in 0..16 {
        let broker = if index % 2 == 0 {
            first.clone()
        } else {
            second.clone()
        };
        let request = request.clone();
        tasks.push(tokio::spawn(async move { broker.publish(request).await }));
    }
    let mut originals = 0usize;
    let mut ids = Vec::new();
    for task in tasks {
        let receipt = task.await.expect("join publisher").expect("publish");
        originals += usize::from(!receipt.is_duplicate());
        ids.push(receipt.id().as_str().to_string());
    }
    assert_eq!(originals, 1);
    assert!(ids.windows(2).all(|pair| pair[0] == pair[1]));

    let receive_a =
        ReceiveRequest::try_new("events", "workers", "worker-a", 1, Duration::from_secs(30))
            .expect("receive a");
    let receive_b =
        ReceiveRequest::try_new("events", "workers", "worker-b", 1, Duration::from_secs(30))
            .expect("receive b");
    let (claimed_a, claimed_b) = tokio::join!(first.receive(receive_a), second.receive(receive_b));
    assert_eq!(
        claimed_a.expect("claim a").len() + claimed_b.expect("claim b").len(),
        1
    );
    drop(first);
    drop(second);
    cleanup(&path);
}

#[tokio::test]
async fn corrupt_rows_fail_closed_and_can_be_repaired_without_losing_the_message() {
    let (path, url) = fixture("corruption");
    let broker = SqliteBroker::connect_with_clock(
        url.clone(),
        config("corruption-contract"),
        ManualClock::new(300_000),
    )
    .await
    .expect("open broker");
    broker
        .subscribe(
            SubscriptionRequest::try_new("audit", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    broker
        .publish(
            PublishRequest::try_new("audit", "entry.ready", "entry/1", b"value".to_vec())
                .expect("publication"),
        )
        .await
        .expect("publish");

    let repair_pool = sqlx::SqlitePool::connect(&url)
        .await
        .expect("open repair connection");
    sqlx::query(
        "UPDATE rullst_messaging_messages SET headers_json = 'not-json' WHERE namespace = ?",
    )
    .bind("corruption-contract")
    .execute(&repair_pool)
    .await
    .expect("inject row corruption");
    let receive = || {
        ReceiveRequest::try_new("audit", "workers", "worker", 1, Duration::from_secs(30))
            .expect("receive")
    };
    assert_eq!(
        broker.receive(receive()).await,
        Err(MessagingError::CorruptStorage {
            context: "message header encoding"
        })
    );
    sqlx::query("UPDATE rullst_messaging_messages SET headers_json = '{}' WHERE namespace = ?")
        .bind("corruption-contract")
        .execute(&repair_pool)
        .await
        .expect("repair row");
    assert_eq!(
        broker
            .receive(receive())
            .await
            .expect("claim repaired message")
            .len(),
        1
    );
    repair_pool.close().await;
    drop(broker);
    cleanup(&path);
}

#[tokio::test]
async fn reopening_a_namespace_with_different_limits_fails_closed() {
    let (path, url) = fixture("configuration");
    let first = SqliteBroker::connect(url.clone(), config("config-contract"))
        .await
        .expect("open first configuration");
    drop(first);
    let changed = BrokerConfig::try_new("config-contract")
        .expect("valid config")
        .with_limits(63, 16, 3, 4 * 1024)
        .expect("changed limits");
    let result = SqliteBroker::connect(url, changed).await;
    assert!(matches!(result, Err(MessagingError::ConfigurationConflict)));
    cleanup(&path);
}

#[tokio::test]
async fn durable_adapter_rejects_in_memory_databases() {
    for database_url in [
        "sqlite::memory:",
        "sqlite://:memory:",
        "sqlite:",
        "sqlite:file::memory:?cache=shared",
        "sqlite://named?mode=memory",
        "sqlite://named?mode=mem%6fry",
        "postgres://localhost/messages",
        "sqlite://messages?mode=unknown",
    ] {
        let result = SqliteBroker::connect(database_url, config("memory-contract")).await;
        assert!(
            matches!(
                result,
                Err(MessagingError::Invalid {
                    field: "durable SQLite database URL",
                    ..
                })
            ),
            "unexpected result for {database_url}"
        );
    }
}

#[tokio::test]
async fn latest_subscription_skips_history_and_allows_explicit_purge() {
    let (path, url) = fixture("latest-purge");
    let broker =
        SqliteBroker::connect_with_clock(url, config("latest-contract"), ManualClock::new(400_000))
            .await
            .expect("open broker");
    broker
        .publish(
            PublishRequest::try_new("audit", "entry.ready", "entry/1", b"old".to_vec())
                .expect("old publication"),
        )
        .await
        .expect("publish history");
    broker
        .subscribe(
            SubscriptionRequest::try_new("audit", "latest", StartPosition::Latest)
                .expect("latest subscription"),
        )
        .await
        .expect("subscribe latest");
    let receive = || {
        ReceiveRequest::try_new("audit", "latest", "worker", 10, Duration::from_secs(30))
            .expect("receive")
    };
    assert!(
        broker
            .receive(receive())
            .await
            .expect("skip history")
            .is_empty()
    );
    assert_eq!(
        broker
            .purge_terminal(PurgeRequest::try_new("audit", 10).expect("purge"))
            .await
            .expect("purge skipped history")
            .removed(),
        1
    );
    broker
        .publish(
            PublishRequest::try_new("audit", "entry.ready", "entry/2", b"new".to_vec())
                .expect("new publication"),
        )
        .await
        .expect("publish new message");
    let delivery = broker
        .receive(receive())
        .await
        .expect("receive new message");
    assert_eq!(delivery.len(), 1);
    assert_eq!(delivery[0].envelope().payload(), b"new");
    drop(broker);
    cleanup(&path);
}

#[tokio::test]
async fn durable_retry_respects_due_time_and_attempt_ceiling() {
    let (path, url) = fixture("retry");
    let clock = ManualClock::new(500_000);
    let broker = SqliteBroker::connect_with_clock(url, config("retry-contract"), clock.clone())
        .await
        .expect("open broker");
    broker
        .subscribe(
            SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    broker
        .publish(
            PublishRequest::try_new("jobs", "job.ready", "job/1", b"work".to_vec())
                .expect("publication"),
        )
        .await
        .expect("publish");
    let receive = || {
        ReceiveRequest::try_new("jobs", "workers", "worker", 1, Duration::from_secs(30))
            .expect("receive")
    };
    let first = broker.receive(receive()).await.expect("first attempt");
    assert_eq!(first[0].attempt(), 1);
    assert_eq!(
        broker
            .retry(
                first[0].ack_token(),
                Duration::from_secs(2),
                FailureCode::try_new("handler.transient").expect("failure code"),
            )
            .await
            .expect("schedule retry"),
        RetryDisposition::Scheduled {
            available_at_ms: 502_000
        }
    );
    assert!(broker.receive(receive()).await.expect("not due").is_empty());
    clock.advance(2_000);
    let second = broker.receive(receive()).await.expect("second attempt");
    assert_eq!(second[0].attempt(), 2);
    broker
        .retry(
            second[0].ack_token(),
            Duration::ZERO,
            FailureCode::try_new("handler.transient").expect("failure code"),
        )
        .await
        .expect("schedule immediate retry");
    let third = broker.receive(receive()).await.expect("third attempt");
    assert_eq!(third[0].attempt(), 3);
    assert_eq!(
        broker
            .retry(
                third[0].ack_token(),
                Duration::ZERO,
                FailureCode::try_new("handler.exhausted").expect("failure code"),
            )
            .await
            .expect("dead-letter exhausted retry"),
        RetryDisposition::DeadLettered
    );
    let dead = broker
        .dead_letters(DeadLetterQuery::try_new("jobs", "workers", 10).expect("dead query"))
        .await
        .expect("list dead letters");
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].attempts(), 3);
    assert_eq!(dead[0].failure_code().as_str(), "handler.exhausted");
    drop(broker);
    cleanup(&path);
}
