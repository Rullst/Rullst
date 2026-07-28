#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use futures::StreamExt;
use rullst_orm::{
    FromRow, Orm,
    schema::{Blueprint, Schema},
};
use std::fs;

#[derive(FromRow, rullst_orm::Orm, Debug, Clone, PartialEq)]
#[orm(table = "stream_test_users")]
pub struct StreamUser {
    pub id: i32,
    pub name: String,
}

const DB_FILE: &str = "stream_test.db";

#[tokio::test]
async fn test_stream_methods() {
    let _ = fs::remove_file(DB_FILE);

    // Initialize the ORM with an SQLite connection
    Orm::init(&format!("sqlite:{}?mode=rwc", DB_FILE))
        .await
        .expect("Orm::init should succeed");

    // Create table
    Schema::create("stream_test_users", |t: &mut Blueprint| {
        t.id();
        t.string("name").not_null();
    })
    .await
    .expect("Schema::create should succeed");

    // Insert 10 records
    for i in 1..=10 {
        let mut user = StreamUser {
            id: 0,
            name: format!("User {}", i),
        };
        user.save().await.expect("save should succeed");
    }

    // Use .stream() to fetch them all
    let qb = StreamUser::query();
    let mut stream = std::pin::pin!(qb.stream());
    let mut fetched = 0;
    while let Some(user_res) = stream.next().await {
        let user = user_res.expect("stream item should be Ok");
        assert!(user.id > 0);
        assert!(user.name.starts_with("User "));
        fetched += 1;
    }
    assert_eq!(fetched, 10, "Should have streamed exactly 10 users");

    // Use .stream_with_tx()
    let mut tx = Orm::begin_transaction().await.expect("begin_transaction");
    {
        let qb_tx = StreamUser::query();
        let mut stream_tx = std::pin::pin!(qb_tx.stream_with_tx(&mut tx));
        let mut fetched_tx = 0;
        while let Some(user_res) = stream_tx.next().await {
            let user = user_res.expect("stream item should be Ok");
            assert!(user.id > 0);
            fetched_tx += 1;
        }
        assert_eq!(
            fetched_tx, 10,
            "Should have streamed exactly 10 users in tx"
        );
    }
    tx.commit().await.expect("commit");

    let _ = fs::remove_file(DB_FILE);
}
