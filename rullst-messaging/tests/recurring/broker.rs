use super::support::*;
use std::sync::atomic::AtomicBool;

struct ObservedBroker {
    inner: SqliteBroker<ManualClock>,
    clock: ManualClock,
    lose_ack: AtomicBool,
    reject: AtomicBool,
}
impl MessageBroker for ObservedBroker {
    async fn publish(&self, request: PublishRequest) -> rullst_messaging::Result<PublishReceipt> {
        if self.reject.load(Ordering::SeqCst) {
            return Err(MessagingError::StorageUnavailable {
                operation: "owned broker fault",
            });
        }
        let receipt = self.inner.publish(request).await?;
        if self.lose_ack.swap(false, Ordering::SeqCst) {
            self.clock.advance(2001);
        }
        Ok(receipt)
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
pub async fn run(url: &str) {
    let namespace = unique();
    let clock = ManualClock::new();
    let store = open(url, &namespace, &clock).await;
    let path = std::env::temp_dir().join(format!("{}.sqlite", unique()));
    let broker_url = format!("sqlite://{}", path.to_string_lossy());
    let broker = ObservedBroker {
        inner: SqliteBroker::connect_encrypted_with_clock(
            &broker_url,
            BrokerConfig::try_new(&namespace).unwrap(),
            keys(),
            clock.clone(),
        )
        .await
        .unwrap(),
        clock: clock.clone(),
        lose_ack: AtomicBool::new(true),
        reject: AtomicBool::new(false),
    };
    broker
        .subscribe(
            SubscriptionRequest::try_new("scheduled", "workers", StartPosition::Earliest).unwrap(),
        )
        .await
        .unwrap();
    store
        .create(definition("one", &clock, MissedRunPolicy::CatchUp))
        .await
        .unwrap();
    store.tick(1).await.unwrap();
    let first = store.claim(1).await.unwrap().remove(0);
    let uncertain = store.relay(&first, &broker).await.unwrap_err();
    let original = match uncertain {
        RecurringRelayError::Acknowledgement {
            publication,
            source: RecurringError::InvalidLease,
        } => publication,
        other => panic!("{other:?}"),
    };
    drop(broker);
    let broker = SqliteBroker::connect_encrypted_with_clock(
        &broker_url,
        BrokerConfig::try_new(&namespace).unwrap(),
        keys(),
        clock.clone(),
    )
    .await
    .unwrap();
    let second = store.claim(1).await.unwrap().remove(0);
    let replay = store.relay(&second, &broker).await.unwrap();
    assert!(replay.publication().is_duplicate());
    assert_eq!(replay.publication().id(), original.id());
    let deliveries = broker
        .receive(
            ReceiveRequest::try_new(
                "scheduled",
                "workers",
                "consumer",
                10,
                Duration::from_secs(30),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(
        deliveries[0].envelope().payload(),
        b"PRIVATE-SCHEDULE-CONTENT"
    );
    broker.ack(deliveries[0].ack_token()).await.unwrap();
    let broker = ObservedBroker {
        inner: broker,
        clock: clock.clone(),
        lose_ack: AtomicBool::new(false),
        reject: AtomicBool::new(true),
    };
    store.cancel("one").await.unwrap();
    store
        .create(definition("fail", &clock, MissedRunPolicy::Coalesce))
        .await
        .unwrap();
    store.tick(1).await.unwrap();
    let mut failed_id = String::new();
    for attempt in 1..=10 {
        let lease = store.claim(1).await.unwrap().remove(0);
        failed_id = lease.metadata().id().to_owned();
        assert_eq!(lease.metadata().attempts(), attempt);
        assert!(matches!(
            store.relay(&lease, &broker).await,
            Err(RecurringRelayError::Publication(_))
        ));
        clock.advance(600_000);
    }
    assert!(store.claim(1).await.unwrap().is_empty());
    assert!(
        store
            .occurrences(None, 100)
            .await
            .unwrap()
            .iter()
            .any(|r| r.id() == failed_id && r.state() == OccurrenceState::DeadLetter)
    );
    store.retry_failed(&failed_id).await.unwrap();
    broker.reject.store(false, Ordering::SeqCst);
    let lease = store.claim(1).await.unwrap().remove(0);
    store.relay(&lease, &broker).await.unwrap();
    assert!(store.retry_failed(&failed_id).await.is_err());
    drop(broker);
    store.close().await;
    for candidate in [
        path.clone(),
        path.with_extension("sqlite-wal"),
        path.with_extension("sqlite-shm"),
    ] {
        let _ = std::fs::remove_file(candidate);
    }
}
