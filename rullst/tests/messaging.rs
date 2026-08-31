#![cfg(feature = "messaging")]

use rullst::messaging::{
    BrokerConfig, InMemoryBroker, MessageBroker, PublishRequest, StartPosition, SubscriptionRequest,
};

#[tokio::test]
async fn umbrella_exports_the_bounded_messaging_boundary() {
    let broker = InMemoryBroker::new(BrokerConfig::try_new("umbrella").expect("configuration"));
    broker
        .subscribe(
            SubscriptionRequest::try_new("events", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    let receipt = broker
        .publish(
            PublishRequest::try_new("events", "event.ready", "event/1", b"payload".to_vec())
                .expect("publication"),
        )
        .await
        .expect("publish");
    assert!(!receipt.is_duplicate());
}
