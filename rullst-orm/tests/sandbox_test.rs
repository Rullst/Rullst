#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::{
    FromRow,
    schema::{Blueprint, Schema},
};
use std::fs;
use std::sync::Once;

#[derive(FromRow, rullst_orm::Orm, Debug, Clone, PartialEq)]
#[orm(table = "sandbox_users")]
pub struct SandboxUser {
    pub id: i32,
    pub name: String,
}

static INIT: Once = Once::new();
const DB_FILE: &str = "sandbox_test.db";

fn init_db() {
    INIT.call_once(|| {
        let _ = fs::remove_file(DB_FILE);
        unsafe {
            std::env::set_var("DATABASE_URL", format!("sqlite:{}?mode=rwc", DB_FILE));
        }
    });
}

#[rullst_orm::test]
async fn test_sandbox_isolation_part_1() {
    init_db();

    // We expect the schema to be created, but since they run in parallel, one might create it first
    let _ = Schema::create("sandbox_users", |t: &mut Blueprint| {
        t.id();
        t.string("name").not_null();
    })
    .await;

    let mut user = SandboxUser {
        id: 0,
        name: "Alice".to_string(),
    };
    user.save().await.unwrap();

    let users = SandboxUser::query().get().await.unwrap();
    // In this transaction, Alice should be there
    assert!(users.iter().any(|u| u.name == "Alice"));
}

#[rullst_orm::test]
async fn test_sandbox_isolation_part_2() {
    init_db();

    // We expect the schema to be created, but since they run in parallel, one might create it first
    let _ = Schema::create("sandbox_users", |t: &mut Blueprint| {
        t.id();
        t.string("name").not_null();
    })
    .await;

    // Wait slightly to ensure part 1 might have inserted Alice if they run parallel
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let users = SandboxUser::query().get().await.unwrap();

    let mut user = SandboxUser {
        id: 0,
        name: "Bob".to_string(),
    };
    user.save().await.unwrap();

    let users = SandboxUser::query().get().await.unwrap();
    assert!(users.iter().any(|u| u.name == "Bob"));
}
