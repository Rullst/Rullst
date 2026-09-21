use rullst::messaging::{
    BrokerConfig, MessageBroker, PublishRequest, RedisBroker, RedisBrokerConfig,
    ReceiveRequest, StartPosition, SubscriptionRequest,
};
use std::time::Duration;

#[test]
fn facade_exposes_optional_redis_contract_without_an_orm_dependency() {
    let runtime = rullst::async_runtime::tokio::runtime::Builder::new_current_thread()
        .enable_all().build().unwrap();
    runtime.block_on(async {
        let config = RedisBrokerConfig::try_new(BrokerConfig::try_new("facade").unwrap(),
            "deployment-v1", "rediss://redis.invalid", "", "mock_fixture").unwrap();
        let broker = RedisBroker::connect(config).await.unwrap();
        assert!(broker.is_mock());
        broker.subscribe(SubscriptionRequest::try_new("events", "workers", StartPosition::Earliest).unwrap()).await.unwrap();
        let request = PublishRequest::try_new("events", "lesson.ready", "lesson:42", b"ready".to_vec()).unwrap();
        let first = broker.publish(request.clone()).await.unwrap();
        let second = broker.publish(request).await.unwrap();
        assert!(second.is_duplicate());
        assert_eq!(first.id(), second.id());
        let messages = broker.receive(ReceiveRequest::try_new("events", "workers", "a", 2, Duration::from_secs(30)).unwrap()).await.unwrap();
        assert_eq!(messages.len(), 1);
        broker.ack(messages[0].ack_token()).await.unwrap();
    });
}
