//! Every reply field is checked even though the bundled scripts never return
//! an out-of-range value: a corrupted or foreign server must fail closed.

use super::fake::{FakeRedis, Resp, bulk, ok, tagged};
use super::*;
use crate::model::StoredEnvelopeParts;
use crate::{
    ContentType, DeadLetterQuery, EventKind, FailureCode, MessageAdmin, MessageBroker,
    MessageEnvelope, MessageHeaders, MessageId, Namespace, PublishRequest, PurgeRequest,
    ReceiveRequest, RetryDisposition, StartPosition, SubscriptionRequest, TopicName,
    WireEnvelopeCodec,
};
use std::time::Duration;

const ID: &str = "msg_0123456789abcdef0123456789abcdef";
/// Index of the first acknowledgement token in a receive `EVAL`: command,
/// script, key count, key, four profile values, group, count and lease.
const FIRST_TOKEN: usize = 11;

fn broker_config() -> BrokerConfig {
    BrokerConfig::try_new("fake")
        .and_then(|config| config.with_limits(4, 2, 3, 1_024))
        .unwrap()
}

async fn connected(fake: &FakeRedis) -> RedisBroker {
    fake.reply(|_| ok(vec![]));
    let config = RedisBrokerConfig::try_new(
        broker_config(),
        "generation-1",
        &fake.url,
        "default",
        "fixture-password",
    )
    .unwrap()
    .allow_loopback_for_tests()
    .unwrap()
    .with_timeout(Duration::from_secs(5))
    .unwrap();
    let broker = RedisBroker::connect(config).await.unwrap();
    assert!(!broker.is_mock());
    broker
}

fn corrupt() -> Option<MessagingError> {
    Some(MessagingError::CorruptStorage {
        context: "Redis namespace",
    })
}

fn wire(topic: &str) -> Vec<u8> {
    let envelope = MessageEnvelope::from_stored(StoredEnvelopeParts {
        id: MessageId::from_stored(ID.to_owned()).unwrap(),
        namespace: Namespace::try_new("fake").unwrap(),
        topic: TopicName::try_new(topic).unwrap(),
        event_kind: EventKind::try_new("job.ready").unwrap(),
        content_type: ContentType::try_new("application/json").unwrap(),
        headers: MessageHeaders::new(),
        payload: b"{}".to_vec(),
        published_at_ms: 1,
    });
    WireEnvelopeCodec::encode(&envelope, &broker_config()).unwrap()
}

fn request() -> PublishRequest {
    PublishRequest::try_new("jobs", "job.ready", "job-1", b"{}".to_vec()).unwrap()
}

fn receive_request() -> ReceiveRequest {
    ReceiveRequest::try_new("jobs", "workers", "worker", 1, Duration::from_secs(1)).unwrap()
}

/// Answers the next receive with one delivery row, echoing the client's token
/// unless `token` is given.
fn delivery_row(
    fake: &FakeRedis,
    topic: &'static str,
    attempt: i64,
    expires: i64,
    token: Option<&'static str>,
) {
    fake.reply(move |command| {
        let token = token.map_or_else(|| command[FIRST_TOKEN].clone(), |t| t.as_bytes().to_vec());
        ok(vec![Resp::Array(vec![
            Resp::Bulk(wire(topic)),
            Resp::Int(attempt),
            Resp::Int(expires),
            Resp::Bulk(token),
        ])])
    });
}

fn dead_row(topic: &str, attempts: i64, timestamp: i64) -> Resp {
    Resp::Array(vec![
        Resp::Bulk(wire(topic)),
        Resp::Int(attempts),
        bulk("handler.failed"),
        Resp::Int(timestamp),
    ])
}

#[tokio::test]
async fn publication_replies_need_a_boolean_flag_and_a_valid_time() {
    let fake = FakeRedis::start().await;
    let broker = connected(&fake).await;
    fake.reply(|_| ok(vec![bulk(ID), Resp::Int(0), Resp::Int(0)]));
    let receipt = broker.publish(request()).await.unwrap();
    assert_eq!(receipt.published_at_ms(), 0);
    assert!(!receipt.is_duplicate());
    for (duplicate, published) in [(2, 5), (0, -1)] {
        fake.reply(move |_| ok(vec![bulk(ID), Resp::Int(duplicate), Resp::Int(published)]));
        assert_eq!(broker.publish(request()).await.err(), corrupt());
    }
    fake.reply(|_| tagged("sequence", vec![]));
    assert_eq!(
        broker.publish(request()).await.err(),
        Some(MessagingError::CapacityExceeded {
            resource: "Redis publication sequence",
            limit: 9_007_199_254_740_000,
        })
    );
}

#[tokio::test]
async fn subscription_replies_need_a_boolean_flag_and_a_bounded_backlog() {
    let fake = FakeRedis::start().await;
    let broker = connected(&fake).await;
    let subscription =
        || SubscriptionRequest::try_new("jobs", "workers", StartPosition::Earliest).unwrap();
    // The backlog may hold every retained message.
    fake.reply(|_| ok(vec![Resp::Int(0), Resp::Int(4)]));
    let receipt = broker.subscribe(subscription()).await.unwrap();
    assert!(!receipt.was_created());
    assert_eq!(receipt.pending_messages(), 4);
    for (created, pending) in [(2, 0), (0, 5)] {
        fake.reply(move |_| ok(vec![Resp::Int(created), Resp::Int(pending)]));
        assert_eq!(broker.subscribe(subscription()).await.err(), corrupt());
    }
}

#[tokio::test]
async fn delivery_rows_are_checked_field_by_field() {
    let fake = FakeRedis::start().await;
    let broker = connected(&fake).await;
    delivery_row(&fake, "jobs", 3, 0, None);
    let deliveries = broker.receive(receive_request()).await.unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].attempt(), 3);
    assert_eq!(deliveries[0].lease_expires_at_ms(), 0);
    for (topic, attempt, expires, token) in [
        ("other", 1, 5, None),
        ("jobs", 0, 5, None),
        ("jobs", 4, 5, None),
        ("jobs", 1, -1, None),
        ("jobs", 1, 5, Some("ack_00000000000000000000000000000000")),
    ] {
        delivery_row(&fake, topic, attempt, expires, token);
        assert_eq!(
            broker.receive(receive_request()).await.err(),
            corrupt(),
            "{topic} {attempt} {expires} {token:?}"
        );
    }
}

#[tokio::test]
async fn retry_replies_pair_each_state_with_a_valid_time() {
    let fake = FakeRedis::start().await;
    let broker = connected(&fake).await;
    delivery_row(&fake, "jobs", 1, 5, None);
    let delivery = broker
        .receive(receive_request())
        .await
        .unwrap()
        .pop()
        .unwrap();
    let retry = |delay: u64| {
        let code = FailureCode::try_new("handler.transient").unwrap();
        broker.retry(delivery.ack_token(), Duration::from_millis(delay), code)
    };
    // Seven days is the longest delay and still reaches the server.
    fake.reply(|_| ok(vec![bulk("retry"), Resp::Int(123)]));
    assert_eq!(
        retry(604_800_000).await,
        Ok(RetryDisposition::Scheduled {
            available_at_ms: 123
        })
    );
    assert_eq!(
        retry(604_800_001).await,
        Err(MessagingError::Invalid {
            field: "retry delay",
            reason: "must not exceed seven days",
        })
    );
    fake.reply(|_| ok(vec![bulk("dead"), Resp::Int(0)]));
    assert_eq!(retry(1_000).await, Ok(RetryDisposition::DeadLettered));
    for (state, timestamp) in [("retry", -1), ("dead", 5)] {
        fake.reply(move |_| ok(vec![bulk(state), Resp::Int(timestamp)]));
        assert_eq!(retry(1_000).await.err(), corrupt(), "{state} {timestamp}");
    }
}

#[tokio::test]
async fn dead_letter_rows_are_bounded_and_checked_field_by_field() {
    let fake = FakeRedis::start().await;
    let broker = connected(&fake).await;
    let query = || DeadLetterQuery::try_new("jobs", "workers", 2).unwrap();
    fake.reply(|_| ok(vec![dead_row("jobs", 1, 0), dead_row("jobs", 3, 7)]));
    let dead = broker.dead_letters(query()).await.unwrap();
    assert_eq!(dead.len(), 2);
    assert_eq!(dead[0].dead_lettered_at_ms(), 0);
    fake.reply(|_| {
        ok(vec![
            dead_row("jobs", 1, 0),
            dead_row("jobs", 1, 0),
            dead_row("jobs", 1, 0),
        ])
    });
    assert_eq!(broker.dead_letters(query()).await.err(), corrupt());
    for (topic, attempts, timestamp) in [
        ("other", 1, 0),
        ("jobs", 0, 0),
        ("jobs", 4, 0),
        ("jobs", 1, -1),
    ] {
        fake.reply(move |_| ok(vec![dead_row(topic, attempts, timestamp)]));
        assert_eq!(
            broker.dead_letters(query()).await.err(),
            corrupt(),
            "{topic} {attempts} {timestamp}"
        );
    }
}

#[tokio::test]
async fn purge_replies_never_exceed_the_requested_limit() {
    let fake = FakeRedis::start().await;
    let broker = connected(&fake).await;
    let purge = || PurgeRequest::try_new("jobs", 3).unwrap();
    fake.reply(|_| ok(vec![Resp::Int(3)]));
    assert_eq!(broker.purge_terminal(purge()).await.unwrap().removed(), 3);
    fake.reply(|_| ok(vec![Resp::Int(4)]));
    assert_eq!(broker.purge_terminal(purge()).await.err(), corrupt());
}
