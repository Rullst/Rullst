//! Each clock reading of the webhook checks is tested on its own: a scripted
//! clock returns chosen readings first and the steady time afterwards.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::{
    BrokerConfig, FailureCode, MessageBroker, PublishRequest, ReceiveRequest, StartPosition,
    SubscriptionRequest,
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const START: i64 = 1_800_000_000_000;
const TOPIC: &str = "webhook.events";
const GROUP: &str = "webhook.receiver";

#[derive(Clone, Default)]
struct ScriptedClock(Arc<Mutex<VecDeque<i64>>>);

impl ScriptedClock {
    fn script(&self, readings: impl IntoIterator<Item = i64>) {
        *self.0.lock().unwrap() = readings.into_iter().collect();
    }
}

impl Clock for ScriptedClock {
    fn now_millis(&self) -> Result<i64> {
        Ok(self.0.lock().unwrap().pop_front().unwrap_or(START))
    }
}

struct Fixture {
    path: PathBuf,
    broker: SqliteBroker<ScriptedClock>,
    clock: ScriptedClock,
    delivery: Delivery,
}

impl Fixture {
    /// Publishes one message at `START` and leases it for 30 seconds.
    async fn leased() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rullst-webhook-checks-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let url = format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"));
        let clock = ScriptedClock::default();
        let config = BrokerConfig::try_new("webhook-checks").unwrap();
        let broker = SqliteBroker::connect_with_clock(url, config, clock.clone())
            .await
            .unwrap();
        broker
            .subscribe(SubscriptionRequest::try_new(TOPIC, GROUP, StartPosition::Earliest).unwrap())
            .await
            .unwrap();
        broker
            .publish(PublishRequest::try_new(TOPIC, "ready", "event-1", b"{}".to_vec()).unwrap())
            .await
            .unwrap();
        let request =
            ReceiveRequest::try_new(TOPIC, GROUP, "worker", 1, Duration::from_secs(30)).unwrap();
        let delivery = broker.receive(request).await.unwrap().pop().unwrap();
        assert_eq!(delivery.lease_expires_at_ms(), START + 30_000);
        Self {
            path,
            broker,
            clock,
            delivery,
        }
    }

    async fn validate(&self, readings: impl IntoIterator<Item = i64>) -> Result<i64> {
        self.clock.script(readings);
        self.broker
            .validate_webhook_delivery(&self.delivery, TOPIC, GROUP, 60_000)
            .await
    }

    async fn retry(&self, readings: impl IntoIterator<Item = i64>) -> Result<()> {
        self.clock.script(readings);
        let id = self.delivery.envelope().id().as_str();
        self.broker
            .retry_webhook_failure(id, TOPIC, GROUP, 60_000)
            .await
    }

    async fn close(self) {
        self.broker.close().await;
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.path.display()));
        }
    }
}

#[tokio::test]
async fn dispatch_check_ends_at_the_lease_and_accepts_an_advancing_clock() {
    let fixture = Fixture::leased().await;
    assert_eq!(fixture.validate([]).await, Ok(START + 30_000));
    assert_eq!(
        fixture.validate([START + 1, START + 2, START + 3]).await,
        Ok(START + 30_000)
    );
    fixture.close().await;
}

#[tokio::test]
async fn dispatch_check_refuses_any_single_reading_outside_the_lease() {
    let fixture = Fixture::leased().await;
    let expired = Err(MessagingError::LeaseExpired);
    // Before creation at the first reading, at the deadline at the second or the third.
    assert_eq!(fixture.validate([START - 1]).await, expired);
    assert_eq!(fixture.validate([START, START + 30_000]).await, expired);
    assert_eq!(
        fixture.validate([START, START, START + 30_000]).await,
        expired
    );
    assert_eq!(fixture.validate([]).await, Ok(START + 30_000));
    fixture.close().await;
}

#[tokio::test]
async fn operator_retry_checks_both_readings_against_the_delivery_window() {
    let fixture = Fixture::leased().await;
    let code = FailureCode::try_new("webhook.receiver_rejected").unwrap();
    fixture
        .broker
        .dead_letter(fixture.delivery.ack_token(), code)
        .await
        .unwrap();
    let expired = Err(MessagingError::LeaseExpired);
    assert_eq!(fixture.retry([START - 1]).await, expired);
    assert_eq!(fixture.retry([START, START + 60_000]).await, expired);
    // Both refusals rolled back, so the failure is still there to retry.
    assert_eq!(fixture.retry([START + 1, START + 2]).await, Ok(()));
    assert_eq!(fixture.retry([]).await, Err(MessagingError::LeaseNotFound));
    fixture.close().await;
}
