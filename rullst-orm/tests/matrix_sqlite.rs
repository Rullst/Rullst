#![cfg(all(
    feature = "strict-sqlite",
    not(feature = "strict-postgres"),
    not(feature = "strict-mysql")
))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "matrix_sqlite_users")]
struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[tokio::test]
async fn strict_sqlite_crud_uses_the_sqlite_pool_and_dialect() {
    Orm::init("sqlite::memory:")
        .await
        .expect("strict SQLite pool should initialize");
    assert_eq!(Orm::driver().expect("driver should be available"), "sqlite");

    Schema::create("matrix_sqlite_users", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
        table.string("email").not_null();
    })
    .await
    .expect("strict SQLite schema should be created");

    let mut user = User {
        id: 0,
        name: "Alice SQLite".to_string(),
        email: "alice@sqlite.test".to_string(),
    };
    user.save().await.expect("strict SQLite insert should work");
    assert!(user.id > 0);

    let found = User::find(user.id)
        .await
        .expect("strict SQLite select should work")
        .expect("inserted user should exist");
    assert_eq!(found.name, "Alice SQLite");

    user.delete()
        .await
        .expect("strict SQLite delete should work");
    assert!(
        User::find(user.id)
            .await
            .expect("strict SQLite final select should work")
            .is_none()
    );
}
