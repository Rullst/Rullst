//! Outbox payload and failure-policy bounds and its migration, on SQLite.
#![cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::schema::migration::Migration;
use rullst_orm::{Error, Orm, Outbox, OutboxMigration};
use serde_json::json;

async fn outbox_table_exists() -> bool {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'rullst_outbox'",
    )
    .fetch_one(Orm::pool().unwrap())
    .await
    .expect("read the schema");
    count == 1
}

async fn enqueue(payload: serde_json::Value) -> Result<(), Error> {
    Orm::transaction(move |_| {
        Box::pin(async move {
            Outbox::enqueue("bounds", "event-1", "bounds.checked", &payload)
                .await
                .map(|_| ())
        })
    })
    .await
}

#[tokio::test]
async fn outbox_bounds_and_migration_on_sqlite() {
    let database_path = std::env::temp_dir().join(format!(
        "rullst-outbox-bounds-{}-{}.db",
        std::process::id(),
        rand::random::<u64>()
    ));
    Orm::init(&format!("sqlite:{}?mode=rwc", database_path.display()))
        .await
        .expect("initialize isolated SQLite ORM");

    OutboxMigration.up().await.expect("migration up");
    assert!(outbox_table_exists().await);

    // A JSON string serializes with its two quotes; 1 MiB is the limit.
    let limit = 1_048_576;
    enqueue(json!("a".repeat(limit - 2)))
        .await
        .expect("a payload of exactly the limit is stored");
    let too_large = enqueue(json!("a".repeat(limit - 1)))
        .await
        .expect_err("one byte over the limit");
    assert!(
        too_large
            .to_string()
            .contains("outbox payload exceeds 1048576 bytes"),
        "{too_large}"
    );

    let policy = "outbox failure policy is outside its bound";
    for (error, max_attempts, delay) in [
        (String::new(), 3, 10),
        ("x".repeat(513), 3, 10),
        ("line\nbreak".to_string(), 3, 10),
        ("failed".to_string(), 0, 10),
        ("failed".to_string(), 3, 86_401),
    ] {
        let refused = Outbox::fail(1, "claim-1", error, max_attempts, delay)
            .await
            .expect_err("outside the failure policy");
        assert!(
            matches!(&refused, Error::Validation(message) if message == policy),
            "{refused}"
        );
    }
    // Within every bound the statement runs and finds no claimed row.
    assert!(
        !Outbox::fail(1, "claim-1", "x".repeat(512), 100, 86_400)
            .await
            .expect("a valid failure policy")
    );

    OutboxMigration.down().await.expect("migration down");
    assert!(!outbox_table_exists().await);

    Orm::pool().unwrap().close().await;
    let _ = std::fs::remove_file(database_path);
}
