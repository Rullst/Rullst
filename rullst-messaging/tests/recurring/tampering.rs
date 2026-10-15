use super::support::*;

const TOPIC: &str = "scheduled";

/// Ticks and claims one occurrence in a fresh namespace.
async fn leased(
    url: &str,
    clock: &ManualClock,
) -> (String, PostgresRecurringStore<ManualClock>, OccurrenceLease) {
    let namespace = unique();
    let store = open(url, &namespace, clock).await;
    store
        .create(definition("one", clock, MissedRunPolicy::CatchUp))
        .await
        .unwrap();
    store.tick(1).await.unwrap();
    let lease = store.claim(1).await.unwrap().remove(0);
    (namespace, store, lease)
}

/// Each field of a leased row is checked on its own: a row that disagrees with
/// the lease in any one of them is refused before anything is published, even
/// when the others, including the lease hash, still match.
pub async fn run(url: &str) {
    let clock = ManualClock::new();
    let admin = admin(url).await;
    for tamper in [
        "UPDATE rullst_recurring_occurrences SET state='pending' WHERE namespace=$1",
        "UPDATE rullst_recurring_occurrences SET version=version+1 WHERE namespace=$1",
        "UPDATE rullst_recurring_occurrences SET lease_until=lease_until+1 WHERE namespace=$1",
        "UPDATE rullst_recurring_occurrences SET expires=0 WHERE namespace=$1",
        "UPDATE rullst_recurring_occurrences SET lease_hash=decode('00','hex') WHERE namespace=$1",
    ] {
        let (namespace, store, lease) = leased(url, &clock).await;
        sqlx::query(tamper)
            .bind(&namespace)
            .execute(&admin)
            .await
            .unwrap();
        let broker =
            InMemoryBroker::with_clock(BrokerConfig::try_new(&namespace).unwrap(), clock.clone());
        broker
            .subscribe(
                SubscriptionRequest::try_new(TOPIC, "workers", StartPosition::Earliest).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            store.relay(&lease, &broker).await.err(),
            Some(RecurringRelayError::BeforePublication(
                RecurringError::InvalidLease
            )),
            "{tamper}"
        );
        let receive =
            ReceiveRequest::try_new(TOPIC, "workers", "worker", 10, Duration::from_secs(30))
                .unwrap();
        assert!(
            broker.receive(receive).await.unwrap().is_empty(),
            "{tamper}"
        );
        store.close().await;
    }

    // A stored due time that is not after the definition's first-after time
    // is refused instead of materialized.
    let namespace = unique();
    let store = open(url, &namespace, &clock).await;
    let schedule = definition("one", &clock, MissedRunPolicy::CatchUp);
    let first_after = schedule.first_after_ms();
    store.create(schedule).await.unwrap();
    sqlx::query("UPDATE rullst_recurring_definitions SET next_due=$2 WHERE namespace=$1")
        .bind(&namespace)
        .bind(first_after)
        .execute(&admin)
        .await
        .unwrap();
    assert_eq!(store.tick(1).await, Err(RecurringError::Configuration));
    assert!(store.occurrences(None, 10).await.unwrap().is_empty());
    store.close().await;
    admin.close().await;
}
