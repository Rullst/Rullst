//! Time must be sampled after waiting for the state lock, not before.

use super::InMemoryBroker;
use crate::{
    BrokerConfig, Clock, MessageBroker, MessagingError, PublishRequest, ReceiveRequest, Result,
    StartPosition, SubscriptionRequest,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

#[derive(Clone)]
struct ManualClock(Arc<AtomicI64>);

impl Clock for ManualClock {
    fn now_millis(&self) -> Result<i64> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

async fn broker_with_lease(clock: &ManualClock) -> (InMemoryBroker<ManualClock>, crate::Delivery) {
    let broker =
        InMemoryBroker::with_clock(BrokerConfig::try_new("lock-time").unwrap(), clock.clone());
    broker
        .subscribe(
            SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest).unwrap(),
        )
        .await
        .unwrap();
    broker
        .publish(PublishRequest::try_new("jobs", "job.ready", "job-1", b"{}".to_vec()).unwrap())
        .await
        .unwrap();
    let mut deliveries = broker
        .receive(
            ReceiveRequest::try_new("jobs", "workers", "worker", 1, Duration::from_secs(1))
                .unwrap(),
        )
        .await
        .unwrap();
    (broker, deliveries.remove(0))
}

/// Lets a spawned task run until it blocks on the held state lock.
async fn let_task_reach_the_lock() {
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn an_ack_waiting_on_the_lock_is_judged_at_admission_time() {
    let clock = ManualClock(Arc::new(AtomicI64::new(1_000_000)));
    let (broker, delivery) = broker_with_lease(&clock).await;
    assert_eq!(delivery.lease_expires_at_ms(), 1_001_000);

    let held = broker.state.lock().await;
    let waiting = broker.clone();
    let token = delivery.ack_token().clone();
    let ack = tokio::spawn(async move { waiting.ack(&token).await });
    let_task_reach_the_lock().await;
    // The lease expires while the ACK waits behind another operation.
    clock.0.store(1_002_000, Ordering::SeqCst);
    drop(held);

    assert_eq!(ack.await.unwrap(), Err(MessagingError::LeaseExpired));
}

#[tokio::test(flavor = "current_thread")]
async fn a_receive_waiting_on_the_lock_grants_a_full_lease() {
    let clock = ManualClock(Arc::new(AtomicI64::new(1_000_000)));
    let broker =
        InMemoryBroker::with_clock(BrokerConfig::try_new("lock-time").unwrap(), clock.clone());
    broker
        .subscribe(
            SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest).unwrap(),
        )
        .await
        .unwrap();
    broker
        .publish(PublishRequest::try_new("jobs", "job.ready", "job-1", b"{}".to_vec()).unwrap())
        .await
        .unwrap();

    let held = broker.state.lock().await;
    let waiting = broker.clone();
    let receive = tokio::spawn(async move {
        waiting
            .receive(
                ReceiveRequest::try_new("jobs", "workers", "worker", 1, Duration::from_secs(1))
                    .unwrap(),
            )
            .await
    });
    let_task_reach_the_lock().await;
    clock.0.store(1_000_900, Ordering::SeqCst);
    drop(held);

    let deliveries = receive.await.unwrap().unwrap();
    assert_eq!(deliveries[0].lease_expires_at_ms(), 1_001_900);
}
