#![cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]

use rullst_orm::schema::migration::Migration;
use rullst_orm::{Error, Orm, Outbox, OutboxMigration};
use serde_json::json;

#[tokio::test]
async fn outbox_is_atomic_idempotent_and_safely_claimed() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-outbox-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    let database_url = format!("sqlite:{}?mode=rwc", database_path.to_string_lossy());
    Orm::init(&database_url)
        .await
        .expect("initialize isolated SQLite ORM");
    Outbox::install().await.expect("install outbox table");
    assert_eq!(
        OutboxMigration.name(),
        "m20260830_000001_create_rullst_outbox"
    );

    let outside_transaction = Outbox::enqueue(
        "tenant-a",
        "order-outside",
        "order.created",
        &json!({"order_id": 1}),
    )
    .await
    .expect_err("managed enqueue must refuse an independent commit");
    assert!(matches!(outside_transaction, Error::Validation(_)));

    let rollback = Orm::transaction(|_| {
        Box::pin(async {
            Outbox::enqueue(
                "tenant-a",
                "order-rollback",
                "order.created",
                &json!({"order_id": 2}),
            )
            .await?;
            Err::<(), Error>(Error::Validation("force rollback".to_string()))
        })
    })
    .await;
    assert!(rollback.is_err());

    let rolled_back_rows: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM rullst_outbox WHERE event_key = ?")
            .bind("order-rollback")
            .fetch_one(Orm::pool().expect("ORM pool"))
            .await
            .expect("count rolled-back outbox rows");
    assert_eq!(rolled_back_rows.0, 0);

    let (first, duplicate, other_stream) = Orm::transaction(|_| {
        Box::pin(async {
            let first = Outbox::enqueue(
                "tenant-a",
                "order-1",
                "order.created",
                &json!({"order_id": 11}),
            )
            .await?;
            let duplicate = Outbox::enqueue(
                "tenant-a",
                "order-1",
                "order.created",
                &json!({"order_id": 11}),
            )
            .await?;
            let other_stream = Outbox::enqueue(
                "tenant-b",
                "order-1",
                "order.created",
                &json!({"order_id": 12}),
            )
            .await?;
            Ok::<_, Error>((first, duplicate, other_stream))
        })
    })
    .await
    .expect("commit idempotent outbox events");
    assert!(first.inserted);
    assert!(!duplicate.inserted);
    assert_eq!(first.id, duplicate.id);
    assert!(other_stream.inserted);
    assert_ne!(first.id, other_stream.id);

    let mut collision_transaction = Orm::begin_transaction()
        .await
        .expect("begin collision transaction");
    let collision = Outbox::enqueue_with_tx(
        &mut collision_transaction,
        "tenant-a",
        "order-1",
        "order.created",
        &json!({"order_id": 999}),
    )
    .await
    .expect_err("reject idempotency-key reuse with different content");
    assert!(matches!(collision, Error::Validation(_)));
    collision_transaction
        .rollback()
        .await
        .expect("rollback collision transaction");

    let tenant_b = Outbox::claim_next("tenant-b", "worker-b", 30, 3)
        .await
        .expect("claim tenant-b event")
        .expect("tenant-b event should exist");
    assert_eq!(tenant_b.payload().expect("parse payload")["order_id"], 12);
    assert!(
        Outbox::acknowledge(tenant_b.id, &tenant_b.claim_key)
            .await
            .expect("acknowledge tenant-b event")
    );

    let now = unix_now();
    let first_claim = Outbox::claim_next_at("tenant-a", "worker-a", now, 10, 2)
        .await
        .expect("claim tenant-a event")
        .expect("tenant-a event should exist");
    assert_eq!(first_claim.attempts, 1);
    assert!(
        !Outbox::acknowledge(first_claim.id, "wrong-token")
            .await
            .expect("reject stale acknowledgement")
    );
    assert!(
        Outbox::fail(first_claim.id, &first_claim.claim_key, "temporary", 2, 5)
            .await
            .expect("release first attempt")
    );
    assert!(
        Outbox::claim_next_at("tenant-a", "worker-a", now, 10, 2)
            .await
            .expect("query delayed retry")
            .is_none()
    );
    let second_claim = Outbox::claim_next_at("tenant-a", "worker-a", now + 6, 10, 2)
        .await
        .expect("claim retry")
        .expect("retry should become available");
    assert_eq!(second_claim.attempts, 2);
    assert_ne!(first_claim.claim_key, second_claim.claim_key);
    assert!(
        !Outbox::acknowledge(first_claim.id, &first_claim.claim_key)
            .await
            .expect("reject superseded claim token")
    );
    assert!(
        Outbox::fail(second_claim.id, &second_claim.claim_key, "permanent", 2, 0)
            .await
            .expect("dead-letter final attempt")
    );
    assert!(
        Outbox::claim_next_at("tenant-a", "worker-a", now + 30, 10, 2)
            .await
            .expect("query dead-lettered stream")
            .is_none()
    );
    let dead_letter: (String, i32, String) =
        sqlx::query_as("SELECT status, attempts, last_error FROM rullst_outbox WHERE id = ?")
            .bind(second_claim.id)
            .fetch_one(Orm::pool().expect("ORM pool"))
            .await
            .expect("inspect dead letter");
    assert_eq!(
        dead_letter,
        ("dead_letter".to_string(), 2, "permanent".to_string())
    );

    enqueue("lease-stream", "lease-1").await;
    let lease_claim = Outbox::claim_next_at("lease-stream", "worker-one", now, 5, 3)
        .await
        .expect("claim leased event")
        .expect("leased event should exist");
    assert!(
        Outbox::claim_next_at("lease-stream", "worker-two", now + 4, 5, 3)
            .await
            .expect("query active lease")
            .is_none()
    );
    let reclaimed = Outbox::claim_next_at("lease-stream", "worker-two", now + 5, 5, 3)
        .await
        .expect("reclaim expired lease")
        .expect("expired event should be reclaimable");
    assert_eq!(reclaimed.attempts, 2);
    assert!(
        !Outbox::acknowledge(lease_claim.id, &lease_claim.claim_key)
            .await
            .expect("reject expired lease token")
    );
    assert!(
        Outbox::acknowledge(reclaimed.id, &reclaimed.claim_key)
            .await
            .expect("acknowledge reclaimed event")
    );

    enqueue("expired-token-stream", "expired-token-1").await;
    let expired = Outbox::claim_next_at("expired-token-stream", "slow-worker", now, 5, 2)
        .await
        .expect("claim expiration fixture")
        .expect("expiration fixture should exist");
    sqlx::query("UPDATE rullst_outbox SET claim_expires_at_epoch = ? WHERE id = ?")
        .bind(1_i64)
        .bind(expired.id)
        .execute(Orm::pool().expect("ORM pool"))
        .await
        .expect("expire fixture claim");
    assert!(
        !Outbox::acknowledge(expired.id, &expired.claim_key)
            .await
            .expect("reject expired token without requiring a newer claimant")
    );

    enqueue("crash-stream", "crash-1").await;
    let crashed = Outbox::claim_next_at("crash-stream", "crashing-worker", now, 5, 1)
        .await
        .expect("claim crash fixture")
        .expect("crash fixture should exist");
    assert_eq!(crashed.attempts, 1);
    assert!(
        Outbox::claim_next_at("crash-stream", "replacement-worker", now + 5, 5, 1)
            .await
            .expect("expire final crashed claim")
            .is_none()
    );
    let crashed_status: (String, String) =
        sqlx::query_as("SELECT status, last_error FROM rullst_outbox WHERE id = ?")
            .bind(crashed.id)
            .fetch_one(Orm::pool().expect("ORM pool"))
            .await
            .expect("inspect exhausted crashed claim");
    assert_eq!(
        crashed_status,
        (
            "dead_letter".to_string(),
            "claim attempt limit reached".to_string()
        )
    );

    enqueue("race-stream", "race-1").await;
    let (left, right) = tokio::join!(
        Outbox::claim_next_at("race-stream", "worker-left", now, 30, 3),
        Outbox::claim_next_at("race-stream", "worker-right", now, 30, 3)
    );
    let claims = [
        left.expect("left concurrent claim should not fail"),
        right.expect("right concurrent claim should not fail"),
    ];
    assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);

    let mut transaction = Orm::begin_transaction()
        .await
        .expect("begin validation transaction");
    let invalid_key = Outbox::enqueue_with_tx(
        &mut transaction,
        "invalid stream",
        "event-1",
        "event.created",
        &json!({}),
    )
    .await
    .expect_err("reject unsafe stream key");
    assert!(matches!(invalid_key, Error::Validation(_)));
    let huge_payload = json!("x".repeat(1_048_577));
    let oversized = Outbox::enqueue_with_tx(
        &mut transaction,
        "valid-stream",
        "event-1",
        "event.created",
        &huge_payload,
    )
    .await
    .expect_err("reject oversized payload");
    assert!(matches!(oversized, Error::Validation(_)));
    transaction
        .rollback()
        .await
        .expect("rollback validation transaction");
    assert!(
        Outbox::claim_next("bad stream", "worker", 30, 3)
            .await
            .is_err()
    );
    assert!(
        Outbox::claim_next("valid-stream", "worker", 0, 3)
            .await
            .is_err()
    );
    assert!(
        Outbox::claim_next("valid-stream", "worker", 30, 0)
            .await
            .is_err()
    );
    assert!(Outbox::acknowledge(0, "valid-token").await.is_err());
    assert!(Outbox::fail(1, "valid-token", "", 2, 0).await.is_err());

    let _ = std::fs::remove_file(database_path);
}

async fn enqueue(stream: &'static str, event_key: &'static str) {
    Orm::transaction(move |_| {
        Box::pin(async move {
            Outbox::enqueue(
                stream,
                event_key,
                "test.created",
                &json!({"event_key": event_key}),
            )
            .await?;
            Ok::<(), Error>(())
        })
    })
    .await
    .expect("enqueue fixture event");
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("test clock must follow Unix epoch")
        .as_secs() as i64
}
