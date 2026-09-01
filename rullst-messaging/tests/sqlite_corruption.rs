#![cfg(feature = "sqlite")]

mod sqlite_adversarial_support;
mod sqlite_support;

use rullst_messaging::{
    DeadLetterQuery, FailureCode, MessageAdmin, MessageBroker, MessagingError, SqliteBroker,
};
use sqlite_adversarial_support::{ManualClock, receive, subscribe_and_publish};
use sqlite_support::{cleanup, config, fixture};

async fn assert_receive_corruption(
    broker: &SqliteBroker<ManualClock>,
    topic: &str,
    group: &str,
    context: &'static str,
) {
    assert_eq!(
        broker.receive(receive(topic, group)).await,
        Err(MessagingError::CorruptStorage { context })
    );
}

#[tokio::test]
async fn malformed_envelope_fields_and_replay_metadata_are_rejected_individually() {
    let (path, url) = fixture("malformed-envelope");
    let broker = SqliteBroker::connect_with_clock(
        url.clone(),
        config("malformed-envelope"),
        ManualClock::new(300_000),
    )
    .await
    .expect("open broker");
    let request = subscribe_and_publish(&broker, "events", "workers", "event/1").await;
    let repair_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("open repair pool");
    let original: (String, String, String, String, Vec<u8>, i64, Vec<u8>) = sqlx::query_as(
        "SELECT message_id, event_kind, content_type, headers_json, payload, published_at_ms, fingerprint FROM rullst_messaging_messages WHERE namespace = ?",
    )
    .bind("malformed-envelope")
    .fetch_one(&repair_pool)
    .await
    .expect("load original row");

    sqlx::query(
        "UPDATE rullst_messaging_messages SET event_kind = 'bad event' WHERE namespace = ?",
    )
    .bind("malformed-envelope")
    .execute(&repair_pool)
    .await
    .expect("corrupt event kind");
    assert_receive_corruption(&broker, "events", "workers", "message event kind").await;
    sqlx::query("UPDATE rullst_messaging_messages SET event_kind = ? WHERE namespace = ?")
        .bind(&original.1)
        .bind("malformed-envelope")
        .execute(&repair_pool)
        .await
        .expect("restore event kind");

    sqlx::query(
        "UPDATE rullst_messaging_messages SET content_type = 'invalid' WHERE namespace = ?",
    )
    .bind("malformed-envelope")
    .execute(&repair_pool)
    .await
    .expect("corrupt content type");
    assert_receive_corruption(&broker, "events", "workers", "message content type").await;
    sqlx::query("UPDATE rullst_messaging_messages SET content_type = ? WHERE namespace = ?")
        .bind(&original.2)
        .bind("malformed-envelope")
        .execute(&repair_pool)
        .await
        .expect("restore content type");

    sqlx::query(
        "UPDATE rullst_messaging_messages SET headers_json = '{\"UPPER\":\"value\"}' WHERE namespace = ?",
    )
    .bind("malformed-envelope")
    .execute(&repair_pool)
    .await
    .expect("corrupt headers");
    assert_receive_corruption(&broker, "events", "workers", "message headers").await;
    sqlx::query("UPDATE rullst_messaging_messages SET headers_json = ? WHERE namespace = ?")
        .bind(&original.3)
        .bind("malformed-envelope")
        .execute(&repair_pool)
        .await
        .expect("restore headers");

    sqlx::query("UPDATE rullst_messaging_messages SET message_id = 'bad' WHERE namespace = ?")
        .bind("malformed-envelope")
        .execute(&repair_pool)
        .await
        .expect("corrupt message id");
    assert_receive_corruption(&broker, "events", "workers", "message identifier").await;
    sqlx::query("UPDATE rullst_messaging_messages SET message_id = ? WHERE namespace = ?")
        .bind(&original.0)
        .bind("malformed-envelope")
        .execute(&repair_pool)
        .await
        .expect("restore message id");

    sqlx::query(
        "UPDATE rullst_messaging_messages SET payload = zeroblob(4097) WHERE namespace = ?",
    )
    .bind("malformed-envelope")
    .execute(&repair_pool)
    .await
    .expect("corrupt payload bounds");
    assert_receive_corruption(&broker, "events", "workers", "message bounds").await;
    sqlx::query("UPDATE rullst_messaging_messages SET payload = ? WHERE namespace = ?")
        .bind(&original.4)
        .bind("malformed-envelope")
        .execute(&repair_pool)
        .await
        .expect("restore payload");

    let mut repair = repair_pool.acquire().await.expect("repair connection");
    sqlx::query("PRAGMA ignore_check_constraints = ON")
        .execute(&mut *repair)
        .await
        .expect("enable corruption fixture");
    sqlx::query(
        "UPDATE rullst_messaging_messages SET fingerprint = zeroblob(31) WHERE namespace = ?",
    )
    .bind("malformed-envelope")
    .execute(&mut *repair)
    .await
    .expect("corrupt fingerprint");
    assert_eq!(
        broker.publish(request.clone()).await,
        Err(MessagingError::CorruptStorage {
            context: "publication fingerprint"
        })
    );
    sqlx::query("UPDATE rullst_messaging_messages SET fingerprint = ? WHERE namespace = ?")
        .bind(&original.6)
        .bind("malformed-envelope")
        .execute(&mut *repair)
        .await
        .expect("restore fingerprint");
    sqlx::query("UPDATE rullst_messaging_messages SET published_at_ms = -1 WHERE namespace = ?")
        .bind("malformed-envelope")
        .execute(&mut *repair)
        .await
        .expect("corrupt publication timestamp");
    assert_eq!(
        broker.publish(request).await,
        Err(MessagingError::CorruptStorage {
            context: "publication timestamp"
        })
    );
    sqlx::query("UPDATE rullst_messaging_messages SET published_at_ms = ? WHERE namespace = ?")
        .bind(original.5)
        .bind("malformed-envelope")
        .execute(&mut *repair)
        .await
        .expect("restore publication timestamp");
    drop(repair);
    repair_pool.close().await;
    drop(broker);
    cleanup(&path);
}

#[tokio::test]
async fn malformed_dead_letter_rows_and_schema_versions_fail_closed() {
    let (path, url) = fixture("malformed-dead-letter");
    let broker = SqliteBroker::connect_with_clock(
        url.clone(),
        config("malformed-dead-letter"),
        ManualClock::new(400_000),
    )
    .await
    .expect("open broker");
    subscribe_and_publish(&broker, "events", "workers", "event/dead").await;
    let delivery = broker
        .receive(receive("events", "workers"))
        .await
        .expect("claim")
        .pop()
        .expect("delivery");
    broker
        .dead_letter(
            delivery.ack_token(),
            FailureCode::try_new("handler.rejected").expect("failure code"),
        )
        .await
        .expect("dead letter");
    let query = || DeadLetterQuery::try_new("events", "workers", 10).expect("query");
    let repair_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("open repair pool");

    sqlx::query(
        "UPDATE rullst_messaging_deliveries SET failure_code = 'INVALID' WHERE namespace = ?",
    )
    .bind("malformed-dead-letter")
    .execute(&repair_pool)
    .await
    .expect("corrupt failure code");
    assert_eq!(
        broker.dead_letters(query()).await,
        Err(MessagingError::CorruptStorage {
            context: "dead-letter failure code"
        })
    );
    sqlx::query("UPDATE rullst_messaging_deliveries SET failure_code = 'handler.rejected', dead_lettered_at_ms = -1 WHERE namespace = ?")
        .bind("malformed-dead-letter")
        .execute(&repair_pool)
        .await
        .expect("corrupt dead-letter timestamp");
    assert_eq!(
        broker.dead_letters(query()).await,
        Err(MessagingError::CorruptStorage {
            context: "dead-letter timestamp"
        })
    );

    let mut repair = repair_pool.acquire().await.expect("repair connection");
    sqlx::query("PRAGMA ignore_check_constraints = ON")
        .execute(&mut *repair)
        .await
        .expect("enable corruption fixture");
    sqlx::query("UPDATE rullst_messaging_deliveries SET dead_lettered_at_ms = 400000, attempt = -1 WHERE namespace = ?")
        .bind("malformed-dead-letter")
        .execute(&mut *repair)
        .await
        .expect("corrupt delivery attempt");
    assert_eq!(
        broker.dead_letters(query()).await,
        Err(MessagingError::CorruptStorage {
            context: "delivery attempt"
        })
    );
    sqlx::query("UPDATE rullst_messaging_deliveries SET attempt = 1 WHERE namespace = ?")
        .bind("malformed-dead-letter")
        .execute(&mut *repair)
        .await
        .expect("restore attempt");
    drop(repair);

    let dead = broker
        .dead_letters(query())
        .await
        .expect("valid dead letter");
    assert_eq!(dead[0].group().as_str(), "workers");
    assert_eq!(dead[0].dead_lettered_at_ms(), 400_000);
    assert_eq!(dead[0].envelope().payload(), b"payload");
    assert!(!format!("{:?}", dead[0]).contains("payload"));

    sqlx::query("UPDATE rullst_messaging_brokers SET schema_version = 2 WHERE namespace = ?")
        .bind("malformed-dead-letter")
        .execute(&repair_pool)
        .await
        .expect("corrupt schema version");
    drop(broker);
    assert!(matches!(
        SqliteBroker::connect(url, config("malformed-dead-letter")).await,
        Err(MessagingError::CorruptStorage {
            context: "schema version"
        })
    ));
    repair_pool.close().await;
    cleanup(&path);
}

#[tokio::test]
async fn unavailable_database_paths_return_redacted_storage_errors() {
    let missing = format!(
        "target/rullst-messaging-tests/missing-{}/broker.sqlite",
        uuid::Uuid::new_v4().simple()
    );
    let result = SqliteBroker::connect(
        format!("sqlite://{missing}"),
        config("unavailable-database"),
    )
    .await;
    assert!(matches!(
        result,
        Err(MessagingError::StorageUnavailable {
            operation: "connect"
        })
    ));
}
