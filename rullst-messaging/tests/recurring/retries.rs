use super::support::*;
use std::time::Instant;

/// Rejects every publication, or accepts it only after `delay` of real time.
struct FixtureBroker {
    inner: InMemoryBroker<ManualClock>,
    reject: bool,
    delay: Duration,
}

impl FixtureBroker {
    fn new(namespace: &str, clock: &ManualClock, reject: bool, delay: Duration) -> Self {
        Self {
            inner: InMemoryBroker::with_clock(
                BrokerConfig::try_new(namespace).unwrap(),
                clock.clone(),
            ),
            reject,
            delay,
        }
    }
}

impl MessageBroker for FixtureBroker {
    async fn publish(&self, request: PublishRequest) -> rullst_messaging::Result<PublishReceipt> {
        if self.reject {
            return Err(MessagingError::StorageUnavailable {
                operation: "fixture rejection",
            });
        }
        tokio::time::sleep(self.delay).await;
        self.inner.publish(request).await
    }
    async fn subscribe(
        &self,
        request: SubscriptionRequest,
    ) -> rullst_messaging::Result<SubscriptionReceipt> {
        self.inner.subscribe(request).await
    }
    async fn receive(&self, request: ReceiveRequest) -> rullst_messaging::Result<Vec<Delivery>> {
        self.inner.receive(request).await
    }
    async fn ack(&self, token: &AckToken) -> rullst_messaging::Result<()> {
        self.inner.ack(token).await
    }
    async fn retry(
        &self,
        token: &AckToken,
        delay: Duration,
        code: FailureCode,
    ) -> rullst_messaging::Result<RetryDisposition> {
        self.inner.retry(token, delay, code).await
    }
    async fn dead_letter(
        &self,
        token: &AckToken,
        code: FailureCode,
    ) -> rullst_messaging::Result<()> {
        self.inner.dead_letter(token, code).await
    }
}

/// A fresh namespace with one materialized occurrence.
async fn ticked(url: &str, clock: &ManualClock) -> (String, PostgresRecurringStore<ManualClock>) {
    let namespace = unique();
    let store = open(url, &namespace, clock).await;
    store
        .create(definition("one", clock, MissedRunPolicy::CatchUp))
        .await
        .unwrap();
    assert_eq!(store.tick(1).await.unwrap().len(), 1);
    (namespace, store)
}

pub async fn run(url: &str) {
    // The first failed attempt waits exactly two seconds before a new claim.
    let clock = ManualClock::new();
    let (namespace, store) = ticked(url, &clock).await;
    let lease = store.claim(1).await.unwrap().remove(0);
    let failing = FixtureBroker::new(&namespace, &clock, true, Duration::ZERO);
    assert!(matches!(
        store.relay(&lease, &failing).await,
        Err(RecurringRelayError::Publication(_))
    ));
    assert!(store.claim(1).await.unwrap().is_empty());
    clock.advance(1_999);
    assert!(store.claim(1).await.unwrap().is_empty());
    clock.advance(1);
    assert_eq!(store.claim(1).await.unwrap().len(), 1);
    store.close().await;

    // A failure whose backoff would end after the delivery window is terminal at once.
    let clock = ManualClock::new();
    let (namespace, store) = ticked(url, &clock).await;
    clock.advance(86_400_000 - 1_500);
    let lease = store.claim(1).await.unwrap().remove(0);
    let failing = FixtureBroker::new(&namespace, &clock, true, Duration::ZERO);
    assert!(matches!(
        store.relay(&lease, &failing).await,
        Err(RecurringRelayError::Publication(_))
    ));
    let rows = store.occurrences(None, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state(), OccurrenceState::DeadLetter);
    store.close().await;

    // Publication is abandoned when the two-second lease ends, not after ten seconds.
    let clock = ManualClock::new();
    let (namespace, store) = ticked(url, &clock).await;
    let lease = store.claim(1).await.unwrap().remove(0);
    let slow = FixtureBroker::new(&namespace, &clock, false, Duration::from_secs(5));
    let started = Instant::now();
    assert_eq!(
        store.relay(&lease, &slow).await.err(),
        Some(RecurringRelayError::Publication(
            MessagingError::StorageUnavailable {
                operation: "bounded recurring publication",
            }
        ))
    );
    assert!(started.elapsed() < Duration::from_secs(5));

    // Closing one handle closes the pool its clones share.
    let clone = store.clone();
    store.close().await;
    assert_eq!(
        clone.schedules(None, 1).await.err(),
        Some(RecurringError::Storage)
    );
}
