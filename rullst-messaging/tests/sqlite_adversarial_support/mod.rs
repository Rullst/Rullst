use rullst_messaging::{
    Clock, MessageBroker, PublishRequest, ReceiveRequest, Result, SqliteBroker, StartPosition,
    SubscriptionRequest,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ManualClock(Arc<AtomicI64>);

impl ManualClock {
    pub fn new(now_millis: i64) -> Self {
        Self(Arc::new(AtomicI64::new(now_millis)))
    }

    #[allow(dead_code)]
    pub fn advance(&self, millis: i64) {
        self.0.fetch_add(millis, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_millis(&self) -> Result<i64> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

pub fn receive(topic: &str, group: &str) -> ReceiveRequest {
    ReceiveRequest::try_new(topic, group, "worker", 10, Duration::from_secs(1))
        .expect("valid receive request")
}

pub async fn subscribe_and_publish(
    broker: &SqliteBroker<ManualClock>,
    topic: &str,
    group: &str,
    key: &str,
) -> PublishRequest {
    broker
        .subscribe(
            SubscriptionRequest::try_new(topic, group, StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    let request = PublishRequest::try_new(topic, "event.ready", key, b"payload".to_vec())
        .expect("publication");
    broker.publish(request.clone()).await.expect("publish");
    request
}
