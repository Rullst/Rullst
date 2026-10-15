//! Exact bounds of the in-memory broker: query and purge limits, retry delays
//! and the clock's lower edge.

use super::InMemoryBroker;
use crate::{
    BrokerConfig, Clock, DeadLetterQuery, Delivery, FailureCode, MessageAdmin, MessageBroker,
    MessagingError, PublishRequest, PurgeRequest, ReceiveRequest, Result, RetryDisposition,
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

async fn broker_with(clock: &ManualClock, messages: usize) -> InMemoryBroker<ManualClock> {
    let broker =
        InMemoryBroker::with_clock(BrokerConfig::try_new("bounds").unwrap(), clock.clone());
    broker
        .subscribe(
            SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest).unwrap(),
        )
        .await
        .unwrap();
    for index in 0..messages {
        let request =
            PublishRequest::try_new("jobs", "job.ready", format!("job-{index}"), b"{}".to_vec())
                .unwrap();
        broker.publish(request).await.unwrap();
    }
    broker
}

async fn receive(broker: &InMemoryBroker<ManualClock>, count: usize) -> Vec<Delivery> {
    let request =
        ReceiveRequest::try_new("jobs", "workers", "worker", count, Duration::from_secs(30))
            .unwrap();
    broker.receive(request).await.unwrap()
}

fn clock_at(millis: i64) -> ManualClock {
    ManualClock(Arc::new(AtomicI64::new(millis)))
}

#[tokio::test]
async fn dead_letter_queries_return_exactly_their_limit() {
    let broker = broker_with(&clock_at(1_000), 3).await;
    for delivery in receive(&broker, 3).await {
        let code = FailureCode::try_new("handler.failed").unwrap();
        broker
            .dead_letter(delivery.ack_token(), code)
            .await
            .unwrap();
    }
    for limit in [1, 2, 3] {
        let query = DeadLetterQuery::try_new("jobs", "workers", limit).unwrap();
        assert_eq!(broker.dead_letters(query).await.unwrap().len(), limit);
    }
}

#[tokio::test]
async fn purges_remove_exactly_their_limit_and_never_pending_messages() {
    let broker = broker_with(&clock_at(1_000), 3).await;
    for delivery in receive(&broker, 2).await {
        broker.ack(delivery.ack_token()).await.unwrap();
    }
    // The third message is still pending for the group that starts at the first one.
    let purge = PurgeRequest::try_new("jobs", 3).unwrap();
    assert_eq!(broker.purge_terminal(purge).await.unwrap().removed(), 2);

    let broker = broker_with(&clock_at(1_000), 3).await;
    for delivery in receive(&broker, 3).await {
        broker.ack(delivery.ack_token()).await.unwrap();
    }
    let purge = PurgeRequest::try_new("jobs", 2).unwrap();
    assert_eq!(broker.purge_terminal(purge).await.unwrap().removed(), 2);

    let broker = broker_with(&clock_at(1_000), 1).await;
    let purge = PurgeRequest::try_new("jobs", 10).unwrap();
    assert_eq!(broker.purge_terminal(purge).await.unwrap().removed(), 0);
}

#[tokio::test]
async fn retries_may_be_delayed_by_exactly_seven_days() {
    let broker = broker_with(&clock_at(1_000), 2).await;
    let deliveries = receive(&broker, 2).await;
    let code = || FailureCode::try_new("handler.transient").unwrap();
    assert_eq!(
        broker
            .retry(
                deliveries[0].ack_token(),
                Duration::from_millis(604_800_000),
                code()
            )
            .await,
        Ok(RetryDisposition::Scheduled {
            available_at_ms: 604_801_000
        })
    );
    assert_eq!(
        broker
            .retry(
                deliveries[1].ack_token(),
                Duration::from_millis(604_800_001),
                code()
            )
            .await,
        Err(MessagingError::Invalid {
            field: "retry delay",
            reason: "must not exceed seven days",
        })
    );
}

#[tokio::test]
async fn the_clock_may_read_zero_but_not_less() {
    let clock = clock_at(0);
    let broker = broker_with(&clock, 1).await;
    assert_eq!(receive(&broker, 1).await[0].envelope().published_at_ms(), 0);
    clock.0.store(-1, Ordering::SeqCst);
    let request = PublishRequest::try_new("jobs", "job.ready", "late", b"{}".to_vec()).unwrap();
    assert_eq!(
        broker.publish(request).await,
        Err(MessagingError::ClockOutOfRange)
    );
}
