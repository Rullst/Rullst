use rullst_messaging::{
    BrokerConfig, InMemoryBroker, MessageBroker, MessagingError, PublishRequest, ReceiveRequest,
};
use std::time::Duration;

#[test]
fn public_builders_reject_unbounded_or_ambiguous_values() {
    assert!(BrokerConfig::try_new("bad namespace").is_err());
    assert!(
        BrokerConfig::try_new("safe")
            .and_then(|config| config.with_limits(0, 1, 1, 1))
            .is_err()
    );
    assert!(PublishRequest::try_new("bad/topic", "event", "key", Vec::new()).is_err());
    assert!(
        PublishRequest::try_new("topic", "event", "key", Vec::new())
            .expect("request")
            .with_content_type("application/json; charset=utf-8")
            .is_err()
    );
    let request = PublishRequest::try_new("topic", "event", "private-key", b"secret".to_vec())
        .expect("request")
        .with_header("trace-id", "private-value")
        .expect("header");
    assert!(
        request
            .clone()
            .with_header("trace-id", "duplicate")
            .is_err()
    );
    assert!(
        request
            .clone()
            .with_header("Trace-Id", "uppercase")
            .is_err()
    );
    assert!(
        request
            .clone()
            .with_header("trace-id-2", "line\nbreak")
            .is_err()
    );
    let debug = format!("{request:?}");
    assert!(!debug.contains("private-key"));
    assert!(!debug.contains("private-value"));
    assert!(!debug.contains("secret"));

    assert!(
        ReceiveRequest::try_new("topic", "group", "worker", 0, Duration::from_secs(1)).is_err()
    );
    assert!(
        ReceiveRequest::try_new("topic", "group", "worker", 1, Duration::from_millis(999),)
            .is_err()
    );
}

#[tokio::test]
async fn configured_payload_and_retention_limits_fail_before_state_growth() {
    let config = BrokerConfig::try_new("bounded")
        .expect("config")
        .with_limits(1, 1, 1, 4)
        .expect("limits");
    let broker = InMemoryBroker::new(config);
    let oversized =
        PublishRequest::try_new("topic", "event", "large", b"12345".to_vec()).expect("request");
    assert_eq!(
        broker.publish(oversized).await,
        Err(MessagingError::CapacityExceeded {
            resource: "message payload bytes",
            limit: 4,
        })
    );
    let bounded =
        PublishRequest::try_new("topic", "event", "small", b"1234".to_vec()).expect("request");
    broker.publish(bounded).await.expect("bounded publish");
    let another =
        PublishRequest::try_new("topic", "event", "other", b"1".to_vec()).expect("request");
    assert_eq!(
        broker.publish(another).await,
        Err(MessagingError::CapacityExceeded {
            resource: "retained messages",
            limit: 1,
        })
    );
}
