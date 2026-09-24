#![cfg(feature = "sqlite")]

mod sqlite_adversarial_support;
mod sqlite_support;

use rullst_messaging::{
    DeadLetterQuery, FailureCode, MessageAdmin, MessageBroker, MessagingError, RetryDisposition,
    SqliteBroker, StartPosition, SubscriptionRequest,
};
use sqlite_adversarial_support::{ManualClock, receive, subscribe_and_publish};
use sqlite_support::{cleanup, config, fixture};
use sqlx::Connection;
use std::time::Duration;

#[tokio::test]
async fn receive_alone_reclaims_an_abandoned_lease_after_restart_in_its_own_scope() {
    let (path, url) = fixture("abandoned-receive");
    let clock = ManualClock::new(100_000);
    let broker = SqliteBroker::connect_with_clock(url.clone(), config("school-a"), clock.clone())
        .await
        .unwrap();
    let other = SqliteBroker::connect_with_clock(url.clone(), config("school-b"), clock.clone())
        .await
        .unwrap();
    subscribe_and_publish(&broker, "jobs", "active", "same-key").await;
    broker
        .subscribe(SubscriptionRequest::try_new("jobs", "idle", StartPosition::Earliest).unwrap())
        .await
        .unwrap();
    subscribe_and_publish(&broker, "other-topic", "active", "same-key").await;
    subscribe_and_publish(&other, "jobs", "active", "same-key").await;
    let first = broker
        .receive(receive("jobs", "active"))
        .await
        .unwrap()
        .remove(0);
    for (owner, topic, group) in [
        (&broker, "jobs", "idle"),
        (&broker, "other-topic", "active"),
        (&other, "jobs", "active"),
    ] {
        assert_eq!(owner.receive(receive(topic, group)).await.unwrap().len(), 1);
    }
    clock.advance(999);
    assert!(
        broker
            .receive(receive("jobs", "active"))
            .await
            .unwrap()
            .is_empty()
    );
    broker.close().await;
    other.close().await;
    clock.advance(1);

    let reopened = SqliteBroker::connect_with_clock(url.clone(), config("school-a"), clock)
        .await
        .unwrap();
    // No expired worker ACK/retry/dead-letter is allowed to perform the sweep.
    let reclaimed = reopened.receive(receive("jobs", "active")).await.unwrap();
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].envelope().id(), first.envelope().id());
    assert_eq!(reclaimed[0].attempt(), 2);
    assert_ne!(reclaimed[0].ack_token(), first.ack_token());

    let inspect = sqlx::SqlitePool::connect(&url).await.unwrap();
    for (namespace, topic, group) in [
        ("school-a", "jobs", "idle"),
        ("school-a", "other-topic", "active"),
        ("school-b", "jobs", "active"),
    ] {
        let row: (String, i64) = sqlx::query_as(
            "SELECT state, attempt FROM rullst_messaging_deliveries WHERE namespace = ? AND topic = ? AND group_name = ?",
        )
        .bind(namespace).bind(topic).bind(group)
        .fetch_one(&inspect).await.unwrap();
        assert_eq!(
            row,
            ("in_flight".into(), 1),
            "unrelated scope {namespace}/{topic}/{group}"
        );
    }
    assert_eq!(
        reopened.ack(first.ack_token()).await,
        Err(MessagingError::LeaseNotFound)
    );
    reopened.ack(reclaimed[0].ack_token()).await.unwrap();
    inspect.close().await;
    reopened.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn receive_alone_dead_letters_abandonment_at_the_attempt_ceiling() {
    let (path, url) = fixture("abandoned-ceiling");
    let clock = ManualClock::new(100_000);
    let broker = SqliteBroker::connect_with_clock(url, config("abandoned-ceiling"), clock.clone())
        .await
        .unwrap();
    subscribe_and_publish(&broker, "jobs", "workers", "one").await;
    let mut original_id = None;
    for attempt in 1..=3 {
        let deliveries = broker.receive(receive("jobs", "workers")).await.unwrap();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].attempt(), attempt);
        if let Some(id) = &original_id {
            assert_eq!(deliveries[0].envelope().id(), id);
        } else {
            original_id = Some(deliveries[0].envelope().id().clone());
        }
        clock.advance(1_000);
    }
    assert!(
        broker
            .receive(receive("jobs", "workers"))
            .await
            .unwrap()
            .is_empty()
    );
    let dead = broker
        .dead_letters(DeadLetterQuery::try_new("jobs", "workers", 10).unwrap())
        .await
        .unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].attempts(), 3);
    assert_eq!(dead[0].envelope().id(), original_id.as_ref().unwrap());
    assert_eq!(dead[0].failure_code().as_str(), "delivery.max_attempts");
    broker.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn retry_accepts_exactly_seven_days_and_rejects_one_more_millisecond() {
    let (path, url) = fixture("retry-inclusive-limit");
    let clock = ManualClock::new(100_000);
    let broker =
        SqliteBroker::connect_with_clock(url, config("retry-inclusive-limit"), clock.clone())
            .await
            .unwrap();
    subscribe_and_publish(&broker, "jobs", "workers", "one").await;
    let first = broker
        .receive(receive("jobs", "workers"))
        .await
        .unwrap()
        .remove(0);
    let limit_ms = 7 * 24 * 60 * 60 * 1_000;
    let failure = FailureCode::try_new("handler.transient").unwrap();
    assert!(matches!(
        broker
            .retry(
                first.ack_token(),
                Duration::from_millis(limit_ms + 1),
                failure.clone()
            )
            .await,
        Err(MessagingError::Invalid {
            field: "retry delay",
            ..
        })
    ));
    assert_eq!(
        broker
            .retry(first.ack_token(), Duration::from_millis(limit_ms), failure)
            .await
            .unwrap(),
        RetryDisposition::Scheduled {
            available_at_ms: 100_000 + limit_ms as i64
        }
    );
    clock.advance(limit_ms as i64 - 1);
    assert!(
        broker
            .receive(receive("jobs", "workers"))
            .await
            .unwrap()
            .is_empty()
    );
    clock.advance(1);
    let next = broker.receive(receive("jobs", "workers")).await.unwrap();
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].envelope().id(), first.envelope().id());
    assert_eq!(next[0].attempt(), 2);
    broker.ack(next[0].ack_token()).await.unwrap();
    broker.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn ignored_delivery_updates_fail_closed_and_roll_back_trigger_effects() {
    let (path, url) = fixture("ignored-transition");
    let broker = SqliteBroker::connect_with_clock(
        url.clone(),
        config("ignored-transition"),
        ManualClock::new(100_000),
    )
    .await
    .unwrap();
    // Keep schema changes on one connection: another pooled connection can
    // retain the old SQLite schema while preparing a repeated CREATE or ALTER.
    let mut inject = sqlx::SqliteConnection::connect(&url).await.unwrap();
    sqlx::query("CREATE TABLE transition_probe (value INTEGER NOT NULL)")
        .execute(&mut inject)
        .await
        .unwrap();
    for operation in ["ack", "retry", "dead-letter"] {
        subscribe_and_publish(&broker, "jobs", "workers", operation).await;
        let delivery = broker
            .receive(receive("jobs", "workers"))
            .await
            .unwrap()
            .remove(0);
        // Trusted test-only fault injection: a trigger suppresses the requested
        // update after a side effect. The broker must reject zero affected rows
        // and roll back that side effect rather than report successful delivery.
        sqlx::query("CREATE TRIGGER ignore_transition BEFORE UPDATE ON rullst_messaging_deliveries WHEN OLD.namespace = 'ignored-transition' BEGIN INSERT INTO transition_probe VALUES (1); SELECT RAISE(IGNORE); END")
            .execute(&mut inject).await.unwrap();
        let failure = FailureCode::try_new("handler.failure").unwrap();
        let result = match operation {
            "ack" => broker.ack(delivery.ack_token()).await,
            "retry" => broker
                .retry(delivery.ack_token(), Duration::ZERO, failure)
                .await
                .map(|_| ()),
            _ => broker.dead_letter(delivery.ack_token(), failure).await,
        };
        let context = if operation == "retry" {
            "retry transition"
        } else {
            "terminal delivery transition"
        };
        assert_eq!(result, Err(MessagingError::CorruptStorage { context }));
        let side_effects: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transition_probe")
            .fetch_one(&mut inject)
            .await
            .unwrap();
        assert_eq!(
            side_effects.0, 0,
            "failed {operation} must roll back the transaction"
        );
        let state: (String,) = sqlx::query_as(
            "SELECT d.state FROM rullst_messaging_deliveries d JOIN rullst_messaging_messages m ON d.namespace = m.namespace AND d.topic = m.topic AND d.sequence = m.sequence WHERE d.namespace = ? AND d.topic = ? AND d.group_name = ? AND m.message_id = ?",
        )
        .bind("ignored-transition")
        .bind("jobs")
        .bind("workers")
        .bind(delivery.envelope().id().as_str())
        .fetch_one(&mut inject)
        .await
        .unwrap();
        assert_eq!(state.0, "in_flight");
        sqlx::query("DROP TRIGGER ignore_transition")
            .execute(&mut inject)
            .await
            .unwrap();
        broker.ack(delivery.ack_token()).await.unwrap();
    }
    inject.close().await.unwrap();
    broker.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn duplicate_delivery_rows_are_rejected_without_committing_acknowledgement() {
    let (path, url) = fixture("duplicate-transition");
    let broker = SqliteBroker::connect_with_clock(
        url.clone(),
        config("duplicate-transition"),
        ManualClock::new(100_000),
    )
    .await
    .unwrap();
    subscribe_and_publish(&broker, "jobs", "workers", "one").await;
    let delivery = broker
        .receive(receive("jobs", "workers"))
        .await
        .unwrap()
        .remove(0);
    // One connection owns both the disposable schema damage and its repair.
    let mut inject = sqlx::SqliteConnection::connect(&url).await.unwrap();
    // The real schema prevents duplicates. Deliberately damage this disposable
    // table to exercise the defensive row-count guard and transaction rollback.
    for sql in [
        "ALTER TABLE rullst_messaging_deliveries RENAME TO original_deliveries",
        "CREATE TABLE rullst_messaging_deliveries AS SELECT * FROM original_deliveries",
        "INSERT INTO rullst_messaging_deliveries SELECT * FROM original_deliveries",
    ] {
        sqlx::query(sql).execute(&mut inject).await.unwrap();
    }
    assert_eq!(
        broker.ack(delivery.ack_token()).await,
        Err(MessagingError::CorruptStorage {
            context: "terminal delivery transition"
        })
    );
    let states: Vec<(String,)> = sqlx::query_as("SELECT state FROM rullst_messaging_deliveries")
        .fetch_all(&mut inject)
        .await
        .unwrap();
    assert_eq!(states, vec![("in_flight".into(),); 2]);
    sqlx::query("DROP TABLE rullst_messaging_deliveries")
        .execute(&mut inject)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE original_deliveries RENAME TO rullst_messaging_deliveries")
        .execute(&mut inject)
        .await
        .unwrap();
    broker.ack(delivery.ack_token()).await.unwrap();
    inject.close().await.unwrap();
    broker.close().await;
    cleanup(&path);
}
