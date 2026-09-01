use rullst_messaging::{
    BrokerConfig, ConsumerGroup, ConsumerName, ContentType, DeadLetterQuery, EventKind,
    FailureCode, IdempotencyKey, InMemoryBroker, MessageBroker, MessageHeaders, MessagingError,
    Namespace, PublishRequest, PurgeRequest, ReceiveRequest, StartPosition, SubscriptionRequest,
    TopicName,
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

#[test]
fn public_value_objects_enforce_their_complete_bounded_grammars() {
    let namespace = Namespace::try_new("tenant:one").expect("namespace");
    let topic = TopicName::try_new("orders.created").expect("topic");
    let group = ConsumerGroup::try_new("workers_primary").expect("group");
    let consumer = ConsumerName::try_new("worker-1").expect("consumer");
    let event = EventKind::try_new("order.created").expect("event");
    for (actual, expected) in [
        (namespace.to_string(), "tenant:one"),
        (topic.to_string(), "orders.created"),
        (group.to_string(), "workers_primary"),
        (consumer.to_string(), "worker-1"),
        (event.to_string(), "order.created"),
    ] {
        assert_eq!(actual, expected);
    }
    assert_eq!(namespace.as_str(), "tenant:one");
    assert_eq!(topic.as_str(), "orders.created");
    assert_eq!(group.as_str(), "workers_primary");
    assert_eq!(consumer.as_str(), "worker-1");
    assert_eq!(event.as_str(), "order.created");

    assert!(Namespace::try_new("").is_err());
    assert!(TopicName::try_new("x".repeat(256)).is_err());
    assert!(ConsumerGroup::try_new("workers/slash").is_err());
    assert!(ConsumerName::try_new("worker space").is_err());
    assert!(EventKind::try_new("event\nkind").is_err());

    let key = IdempotencyKey::try_new("tenant/order:1").expect("idempotency key");
    assert_eq!(format!("{key:?}"), "IdempotencyKey([REDACTED])");
    assert!(IdempotencyKey::try_new("").is_err());
    assert!(IdempotencyKey::try_new("x".repeat(256)).is_err());
    assert!(IdempotencyKey::try_new("not allowed").is_err());

    let content_type = ContentType::try_new("application/vnd.rullst+json").expect("content type");
    assert_eq!(content_type.as_str(), "application/vnd.rullst+json");
    for invalid in [
        "",
        "application",
        "/json",
        "application/",
        "application/json/extra",
        "application/json; charset=utf-8",
        "appli(cation/json",
        "aplicação/json",
    ] {
        assert!(
            ContentType::try_new(invalid).is_err(),
            "accepted {invalid:?}"
        );
    }
    assert!(ContentType::try_new(format!("application/{}", "x".repeat(128))).is_err());

    let failure = FailureCode::try_new("handler.transient_2").expect("failure code");
    assert_eq!(failure.as_str(), "handler.transient_2");
    assert!(FailureCode::try_new("").is_err());
    assert!(FailureCode::try_new("x".repeat(129)).is_err());
    assert!(FailureCode::try_new("Handler.Transient").is_err());
}

#[test]
fn headers_are_deterministic_redacted_and_bounded_by_count_and_bytes() {
    let mut headers = MessageHeaders::new();
    assert!(headers.is_empty());
    assert_eq!(headers.len(), 0);
    assert_eq!(headers.get("missing"), None);
    headers
        .try_insert("trace-id", "private-value")
        .expect("header");
    headers.try_insert("attempt", "1").expect("second header");
    assert_eq!(headers.get("trace-id"), Some("private-value"));
    assert_eq!(headers.len(), 2);
    assert_eq!(
        headers.iter().collect::<Vec<_>>(),
        vec![("attempt", "1"), ("trace-id", "private-value")]
    );
    let debug = format!("{headers:?}");
    assert!(debug.contains("trace-id"));
    assert!(!debug.contains("private-value"));

    assert!(headers.try_insert("trace-id", "duplicate").is_err());
    assert!(headers.try_insert("", "value").is_err());
    assert!(headers.try_insert("UPPER", "value").is_err());
    assert!(headers.try_insert("x".repeat(65), "value").is_err());
    assert!(headers.try_insert("control", "line\nbreak").is_err());
    assert!(headers.try_insert("large", "x".repeat(1_025)).is_err());

    let mut count_bounded = MessageHeaders::new();
    for index in 0..32 {
        count_bounded
            .try_insert(format!("header-{index}"), "value")
            .expect("within header count");
    }
    assert_eq!(
        count_bounded.try_insert("header-overflow", "value"),
        Err(MessagingError::CapacityExceeded {
            resource: "message headers",
            limit: 32,
        })
    );

    let mut byte_bounded = MessageHeaders::new();
    for index in 0..7 {
        byte_bounded
            .try_insert(format!("h{index}"), "x".repeat(1_024))
            .expect("within aggregate byte limit");
    }
    assert_eq!(
        byte_bounded.try_insert("overflow", "x".repeat(1_024)),
        Err(MessagingError::CapacityExceeded {
            resource: "message header bytes",
            limit: 8 * 1_024,
        })
    );
}

#[test]
fn request_and_configuration_boundaries_are_observable_through_public_accessors() {
    let config = BrokerConfig::try_new("bounded")
        .expect("config")
        .with_limits(
            BrokerConfig::MAX_RETAINED_MESSAGES,
            BrokerConfig::MAX_SUBSCRIPTIONS,
            BrokerConfig::MAX_ATTEMPTS,
            BrokerConfig::MAX_PAYLOAD_BYTES,
        )
        .expect("maximum supported limits");
    assert_eq!(config.namespace().as_str(), "bounded");
    assert_eq!(
        config.max_retained_messages(),
        BrokerConfig::MAX_RETAINED_MESSAGES
    );
    assert_eq!(config.max_subscriptions(), BrokerConfig::MAX_SUBSCRIPTIONS);
    assert_eq!(config.max_attempts(), BrokerConfig::MAX_ATTEMPTS);
    assert_eq!(config.max_payload_bytes(), BrokerConfig::MAX_PAYLOAD_BYTES);

    let base = BrokerConfig::try_new("bounded").expect("base config");
    assert!(
        base.clone()
            .with_limits(BrokerConfig::MAX_RETAINED_MESSAGES + 1, 1, 1, 1)
            .is_err()
    );
    assert!(
        base.clone()
            .with_limits(1, BrokerConfig::MAX_SUBSCRIPTIONS + 1, 1, 1)
            .is_err()
    );
    assert!(
        base.clone()
            .with_limits(1, 1, BrokerConfig::MAX_ATTEMPTS + 1, 1)
            .is_err()
    );
    assert!(
        base.with_limits(1, 1, 1, BrokerConfig::MAX_PAYLOAD_BYTES + 1)
            .is_err()
    );

    let publish = PublishRequest::try_new("events", "event.created", "event/1", b"body".to_vec())
        .expect("publish")
        .with_content_type("application/json")
        .expect("content type")
        .with_header("trace-id", "trace-1")
        .expect("header");
    assert_eq!(publish.topic().as_str(), "events");
    assert_eq!(publish.event_kind().as_str(), "event.created");
    assert_eq!(publish.content_type().as_str(), "application/json");
    assert_eq!(publish.headers().get("trace-id"), Some("trace-1"));
    assert_eq!(publish.payload(), b"body");

    let subscription = SubscriptionRequest::try_new("events", "workers", StartPosition::Latest)
        .expect("subscription");
    assert_eq!(subscription.topic().as_str(), "events");
    assert_eq!(subscription.group().as_str(), "workers");
    assert_eq!(subscription.start(), StartPosition::Latest);

    let receive = ReceiveRequest::try_new(
        "events",
        "workers",
        "worker-1",
        100,
        Duration::from_secs(3_600),
    )
    .expect("receive");
    assert_eq!(receive.topic().as_str(), "events");
    assert_eq!(receive.group().as_str(), "workers");
    assert_eq!(receive.consumer().as_str(), "worker-1");
    assert_eq!(receive.max_messages(), 100);
    assert!(
        ReceiveRequest::try_new("events", "workers", "worker", 101, Duration::from_secs(1))
            .is_err()
    );
    assert!(
        ReceiveRequest::try_new("events", "workers", "worker", 1, Duration::from_secs(3_601))
            .is_err()
    );
    assert!(ReceiveRequest::try_new("events", "workers", "worker", 1, Duration::MAX).is_err());

    assert!(DeadLetterQuery::try_new("events", "workers", 0).is_err());
    assert!(DeadLetterQuery::try_new("events", "workers", 101).is_err());
    assert!(PurgeRequest::try_new("events", 0).is_err());
    assert!(PurgeRequest::try_new("events", 1_001).is_err());
}
