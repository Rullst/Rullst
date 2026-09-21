use super::redis_support::*;
use rullst_messaging::*;
use rullst_orm::{Error, Orm, Outbox};
use serde_json::json;

#[tokio::test]
#[ignore = "requires an owned Redis service and the backend-neutral ORM profile"]
async fn remote_outbox_replay_converges_after_publish_before_ack() {
    let profile = std::any::type_name::<rullst_orm::RullstDatabase>();
    assert!(
        profile.contains("Any") || profile.contains("Sqlite"),
        "run the dedicated redis-streams,orm-outbox profile"
    );
    let directory = std::env::temp_dir().join(unique("rullst-remote-outbox"));
    std::fs::create_dir(&directory).unwrap();
    let database = directory.join("outbox.db");
    Orm::init(&format!("sqlite:{}?mode=rwc", database.to_string_lossy()))
        .await
        .unwrap();
    Outbox::install().await.unwrap();
    Orm::transaction(|_| {
        Box::pin(async {
            Outbox::enqueue(
                "school-a",
                "invoice:42:v1",
                "invoice.created",
                &json!({"invoice":42}),
            )
            .await?;
            Ok::<(), Error>(())
        })
    })
    .await
    .unwrap();
    let config = configuration(&unique("outbox"));
    let broker = RedisBroker::provision(config.clone()).await.unwrap();
    broker
        .subscribe(subscription("events", "billing", StartPosition::Earliest))
        .await
        .unwrap();
    let first_relay = OrmOutboxRelay::try_new("school-a", "events", broker).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let first = Outbox::claim_next_at("school-a", "relay-a", now, 1, 3)
        .await
        .unwrap()
        .unwrap();
    let published = first_relay.publish_claim(&first).await.unwrap();
    assert!(!published.is_duplicate());
    drop(first_relay);
    drop(first);
    let second_relay = OrmOutboxRelay::try_new(
        "school-a",
        "events",
        RedisBroker::connect(config).await.unwrap(),
    )
    .unwrap();
    let next = Outbox::claim_next_at("school-a", "relay-b", now + 1, 30, 3)
        .await
        .unwrap()
        .unwrap();
    let receipt = second_relay.relay_and_ack(next).await.unwrap();
    assert!(receipt.publication().is_duplicate());
    assert_eq!(receipt.publication().id(), published.id());
    assert!(receipt.outbox_acknowledged());
    let deliveries = second_relay
        .broker()
        .receive(receive("events", "billing", "worker", 10))
        .await
        .unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].envelope().payload(), br#"{"invoice":42}"#);
    second_relay
        .broker()
        .ack(deliveries[0].ack_token())
        .await
        .unwrap();
    assert!(
        Outbox::claim_next_at("school-a", "relay-c", now + 40, 30, 3)
            .await
            .unwrap()
            .is_none()
    );
    Orm::pool().unwrap().close().await;
    std::fs::remove_dir_all(directory).unwrap();
}
