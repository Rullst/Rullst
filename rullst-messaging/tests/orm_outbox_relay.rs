#![cfg(feature = "orm-outbox")]

use rullst_messaging::{
    InMemoryBroker, MessageBroker, OrmOutboxRelay, OrmOutboxRelayError, ReceiveRequest,
    StartPosition, SubscriptionRequest,
};
use rullst_orm::{Error, Orm, Outbox};
use serde_json::json;
use std::time::Duration;

#[tokio::test]
async fn committed_claim_replays_exactly_after_publish_before_ack_crash() {
    // TM-MESSAGING-06: a crash-window replay cannot create a second broker message.
    if !sqlite_compatible_orm_profile() {
        // `cargo test --workspace --all-features` unifies the ORM's mutually
        // exclusive strict profiles and intentionally selects PostgreSQL.
        // The dedicated no-default-features CI/release command executes this
        // SQLite contract against the backend-neutral ORM profile.
        return;
    }
    let database_path = std::env::temp_dir().join(format!(
        "rullst-messaging-outbox-{}-{}.db",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let database_url = format!("sqlite:{}?mode=rwc", database_path.to_string_lossy());
    Orm::init(&database_url)
        .await
        .expect("initialize isolated ORM");
    Outbox::install().await.expect("install outbox");
    Orm::transaction(|_| {
        Box::pin(async {
            Outbox::enqueue(
                "tenant-a",
                "invoice:42:issued:v1",
                "invoice.issued",
                &json!({"invoice_id": 42}),
            )
            .await?;
            Ok::<(), Error>(())
        })
    })
    .await
    .expect("commit domain outbox event");

    let broker = InMemoryBroker::new(
        rullst_messaging::BrokerConfig::try_new("relay-contract").expect("broker config"),
    );
    broker
        .subscribe(
            SubscriptionRequest::try_new("events", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    let relay = OrmOutboxRelay::try_new("tenant-a", "events", broker).expect("relay");
    let now = unix_now();
    let first = Outbox::claim_next_at("tenant-a", "worker-a", now, 5, 3)
        .await
        .expect("claim first attempt")
        .expect("committed event");
    let debug = format!("{first:?}");
    assert!(!debug.contains("invoice:42"));
    assert!(!debug.contains(&first.claim_key));
    assert!(!debug.contains("tenant-a"));
    let first_publication = relay
        .publish_claim(&first)
        .await
        .expect("publish before simulated crash");
    assert!(!first_publication.is_duplicate());
    drop(first);

    let reclaimed = Outbox::claim_next_at("tenant-a", "worker-b", now + 5, 30, 3)
        .await
        .expect("reclaim after crash window")
        .expect("expired claim becomes available");
    let result = relay
        .relay_and_ack(reclaimed)
        .await
        .expect("replay and acknowledge");
    assert!(result.publication().is_duplicate());
    assert_eq!(result.publication().id(), first_publication.id());
    assert!(result.outbox_acknowledged());

    let deliveries = relay
        .broker()
        .receive(
            ReceiveRequest::try_new(
                "events",
                "workers",
                "consumer-a",
                10,
                Duration::from_secs(30),
            )
            .expect("receive request"),
        )
        .await
        .expect("receive relayed message");
    assert_eq!(deliveries.len(), 1);
    assert_eq!(
        deliveries[0].envelope().event_kind().as_str(),
        "invoice.issued"
    );
    assert_eq!(deliveries[0].envelope().payload(), br#"{"invoice_id":42}"#);
    assert!(
        Outbox::claim_next_at("tenant-a", "worker-c", now + 40, 30, 3)
            .await
            .expect("query acknowledged outbox row")
            .is_none()
    );

    let fabricated = rullst_orm::ClaimedOutboxEvent {
        id: 7,
        stream: "other-tenant".to_string(),
        event_key: "event:7".to_string(),
        event_kind: "event.ready".to_string(),
        payload_json: "not-json".to_string(),
        attempts: 1,
        claim_key: "claim-key".to_string(),
        claim_expires_at_epoch: now + 60,
    };
    assert!(matches!(
        relay.publish_claim(&fabricated).await,
        Err(OrmOutboxRelayError::InvalidClaim { .. })
    ));

    let _ = std::fs::remove_file(database_path);
}

#[tokio::test]
async fn valid_claim_json_is_normalized_without_requiring_field_order() {
    let broker = InMemoryBroker::new(
        rullst_messaging::BrokerConfig::try_new("relay-canonical-json").expect("broker config"),
    );
    broker
        .subscribe(
            SubscriptionRequest::try_new("events", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    let relay = OrmOutboxRelay::try_new("tenant-a", "events", broker).expect("relay");
    let claim = rullst_orm::ClaimedOutboxEvent {
        id: 9,
        stream: "tenant-a".to_string(),
        event_key: "event:9".to_string(),
        event_kind: "event.ready".to_string(),
        payload_json: r#"{"z_status":"issued","invoice_id":42}"#.to_string(),
        attempts: 1,
        claim_key: "claim-key".to_string(),
        claim_expires_at_epoch: unix_now() + 60,
    };
    relay.publish_claim(&claim).await.expect("publish claim");
    let delivery = relay
        .broker()
        .receive(
            ReceiveRequest::try_new(
                "events",
                "workers",
                "consumer-a",
                1,
                Duration::from_secs(30),
            )
            .expect("receive request"),
        )
        .await
        .expect("receive canonical payload")
        .pop()
        .expect("delivery");
    let delivered: serde_json::Value =
        serde_json::from_slice(delivery.envelope().payload()).expect("valid delivered JSON");
    assert_eq!(delivered, json!({"invoice_id": 42, "z_status": "issued"}));
}

fn claim(stream: &str, id: i64, payload_json: &str) -> rullst_orm::ClaimedOutboxEvent {
    rullst_orm::ClaimedOutboxEvent {
        id,
        stream: stream.to_string(),
        event_key: "order-42".to_string(),
        event_kind: "order.created".to_string(),
        payload_json: payload_json.to_string(),
        attempts: 1,
        claim_key: "claim-key".to_string(),
        claim_expires_at_epoch: unix_now() + 60,
    }
}

#[tokio::test]
async fn streams_sharing_a_topic_keep_their_own_idempotency_scope() {
    // Outbox event keys are unique per stream, while the broker deduplicates
    // per topic: equal keys from two streams must stay two messages.
    let broker = InMemoryBroker::new(
        rullst_messaging::BrokerConfig::try_new("relay-stream-scope").expect("broker config"),
    );
    broker
        .subscribe(
            SubscriptionRequest::try_new("domain-events", "workers", StartPosition::Earliest)
                .expect("subscription"),
        )
        .await
        .expect("subscribe");
    let tenant_a =
        OrmOutboxRelay::try_new("tenant-a", "domain-events", broker.clone()).expect("relay a");
    let tenant_b =
        OrmOutboxRelay::try_new("tenant-b", "domain-events", broker.clone()).expect("relay b");
    let created = r#"{"status":"created"}"#;
    let first = tenant_a
        .publish_claim(&claim("tenant-a", 1, created))
        .await
        .expect("tenant A publication");
    let identical = tenant_b
        .publish_claim(&claim("tenant-b", 2, created))
        .await
        .expect("tenant B publication with identical content");
    assert!(!identical.is_duplicate());
    assert_ne!(identical.id(), first.id());
    // A different payload under the same key is not an idempotency conflict.
    let tenant_c =
        OrmOutboxRelay::try_new("tenant-c", "domain-events", broker.clone()).expect("relay c");
    let different = tenant_c
        .publish_claim(&claim("tenant-c", 3, r#"{"status":"paid"}"#))
        .await
        .expect("tenant C publication with different content");
    assert!(!different.is_duplicate());
    // The crash-window replay of one stream still deduplicates.
    let replay = tenant_a
        .publish_claim(&claim("tenant-a", 1, created))
        .await
        .expect("tenant A replay");
    assert!(replay.is_duplicate());
    assert_eq!(replay.id(), first.id());

    let deliveries = broker
        .receive(
            ReceiveRequest::try_new(
                "domain-events",
                "workers",
                "consumer-a",
                10,
                Duration::from_secs(30),
            )
            .expect("receive request"),
        )
        .await
        .expect("receive relayed messages");
    assert_eq!(deliveries.len(), 3);
}

fn sqlite_compatible_orm_profile() -> bool {
    let database = std::any::type_name::<rullst_orm::RullstDatabase>();
    database.contains("Any") || database.contains("Sqlite")
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("test clock follows Unix epoch")
        .as_secs() as i64
}
