#![allow(clippy::expect_used, clippy::unwrap_used)]
//! Debug output names every non-secret field and redacts keys, tokens and payloads.

use crate::{
    BrokerConfig, Clock, DeadLetterQuery, FailureCode, InMemoryBroker, MessageAdmin, MessageBroker,
    PublishRequest, ReceiveRequest, Result, StartPosition, SubscriptionRequest,
};
use std::time::Duration;

#[derive(Clone)]
struct FixedClock;

impl Clock for FixedClock {
    fn now_millis(&self) -> Result<i64> {
        Ok(1_800_000_000_000)
    }
}

fn request() -> PublishRequest {
    PublishRequest::try_new("orders", "order.ready", "private-key", b"secret".to_vec())
        .unwrap()
        .with_header("trace-id", "private-value")
        .unwrap()
}

#[test]
fn publish_request_debug_lists_metadata_only() {
    assert_eq!(
        format!("{:?}", request()),
        "PublishRequest { topic: TopicName(\"orders\"), event_kind: EventKind(\"order.ready\"), \
         idempotency_key: IdempotencyKey([REDACTED]), \
         content_type: ContentType(\"application/octet-stream\"), \
         headers: MessageHeaders { names: [\"trace-id\"], count: 1 }, payload_bytes: 6 }"
    );
}

#[tokio::test]
async fn envelope_delivery_and_dead_letter_debug_list_metadata_only() {
    let broker = InMemoryBroker::with_clock(BrokerConfig::try_new("debug").unwrap(), FixedClock);
    broker
        .subscribe(
            SubscriptionRequest::try_new("orders", "workers", StartPosition::Earliest).unwrap(),
        )
        .await
        .unwrap();
    broker.publish(request()).await.unwrap();
    let receive =
        ReceiveRequest::try_new("orders", "workers", "worker", 1, Duration::from_secs(30)).unwrap();
    let delivery = broker.receive(receive).await.unwrap().pop().unwrap();
    let id = delivery.envelope().id().as_str().to_owned();
    let envelope = format!(
        "MessageEnvelope {{ schema: {:?}, id: MessageId({id:?}), namespace: Namespace(\"debug\"), \
         topic: TopicName(\"orders\"), event_kind: EventKind(\"order.ready\"), \
         content_type: ContentType(\"application/octet-stream\"), \
         headers: MessageHeaders {{ names: [\"trace-id\"], count: 1 }}, payload_bytes: 6, \
         published_at_ms: 1800000000000 }}",
        delivery.envelope().schema()
    );
    assert_eq!(format!("{:?}", delivery.envelope()), envelope);
    assert_eq!(
        format!("{delivery:?}"),
        format!(
            "Delivery {{ envelope: {envelope}, group: ConsumerGroup(\"workers\"), \
             consumer: ConsumerName(\"worker\"), attempt: 1, \
             lease_expires_at_ms: 1800000030000, ack_token: AckToken([REDACTED]) }}"
        )
    );
    broker
        .dead_letter(
            delivery.ack_token(),
            FailureCode::try_new("handler.failed").unwrap(),
        )
        .await
        .unwrap();
    let dead = broker
        .dead_letters(DeadLetterQuery::try_new("orders", "workers", 1).unwrap())
        .await
        .unwrap();
    assert_eq!(
        format!("{:?}", dead[0]),
        format!(
            "DeadLetter {{ message_id: MessageId({id:?}), group: ConsumerGroup(\"workers\"), \
             attempts: 1, failure_code: FailureCode(\"handler.failed\"), \
             dead_lettered_at_ms: 1800000000000 }}"
        )
    );
}
