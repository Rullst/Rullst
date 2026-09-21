use rullst_messaging::*;
use std::time::Duration;

pub fn password() -> String {
    std::env::var("RULLST_MESSAGING_TEST_REDIS_PASSWORD")
        .expect("run .github/check-messaging-redis.py with its generated fixture credential")
}

pub fn endpoint() -> String {
    std::env::var("RULLST_MESSAGING_TEST_REDIS_URL").expect("run .github/check-messaging-redis.py")
}

pub fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}

pub fn configuration(namespace: &str) -> RedisBrokerConfig {
    configuration_with(
        BrokerConfig::try_new(namespace)
            .unwrap()
            .with_limits(100, 8, 2, 4096)
            .unwrap(),
    )
}

pub fn configuration_with(broker: BrokerConfig) -> RedisBrokerConfig {
    RedisBrokerConfig::try_new(
        broker,
        "fixture-generation",
        endpoint(),
        "default",
        password(),
    )
    .unwrap()
    .allow_loopback_for_tests()
    .unwrap()
}

pub fn publication(topic: &str, key: &str) -> PublishRequest {
    PublishRequest::try_new(topic, "event.created", key, b"fixture payload".to_vec()).unwrap()
}

pub fn subscription(topic: &str, group: &str, start: StartPosition) -> SubscriptionRequest {
    SubscriptionRequest::try_new(topic, group, start).unwrap()
}

pub fn receive(topic: &str, group: &str, consumer: &str, limit: usize) -> ReceiveRequest {
    ReceiveRequest::try_new(topic, group, consumer, limit, Duration::from_secs(1)).unwrap()
}

pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn prefix(namespace: &str) -> String {
    format!("rullst:messaging:v1:{}:", digest(namespace.as_bytes()))
}

pub async fn raw() -> redis::aio::MultiplexedConnection {
    use redis::IntoConnectionInfo;
    let info = endpoint()
        .into_connection_info()
        .unwrap()
        .set_redis_settings(
            redis::RedisConnectionInfo::default()
                .set_username("default")
                .set_password(password()),
        );
    redis::Client::open(info)
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap()
}
