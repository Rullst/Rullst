#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::{
    FromRow, Orm,
    schema::{Blueprint, Schema},
};
use std::fs;

#[derive(FromRow, rullst_orm::Orm, Debug, Clone, PartialEq)]
#[orm(table = "raw_test_users")]
pub struct RawUser {
    pub id: i32,
    pub name: String,
}

const DB_FILE: &str = "raw_test.db";

#[tokio::test]
async fn test_raw_query_fallback() {
    let _ = fs::remove_file(DB_FILE);

    Orm::init(&format!("sqlite:{}?mode=rwc", DB_FILE))
        .await
        .expect("Orm::init should succeed");

    Schema::create("raw_test_users", |t: &mut Blueprint| {
        t.id();
        t.string("name").not_null();
    })
    .await
    .expect("Schema::create should succeed");

    // Insert manually using Raw execute
    let rows_affected = Orm::raw("INSERT INTO raw_test_users (name) VALUES (?), (?)")
        .bind("Alice")
        .bind("Bob")
        .execute()
        .await
        .expect("Raw execute should succeed");

    assert_eq!(rows_affected, 2);

    // Fetch using map_to
    let users = Orm::raw("SELECT * FROM raw_test_users WHERE name = ?")
        .bind("Alice")
        .map_to::<RawUser>()
        .await
        .expect("Raw map_to should succeed");

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Alice");

    let _ = fs::remove_file(DB_FILE);
}
