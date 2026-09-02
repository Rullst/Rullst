#![cfg(not(any(feature = "strict-sqlite", feature = "strict-postgres")))]

mod support;

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mariadb::Mariadb;

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "mariadb_users")]
struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(rullst_orm::Enum, Debug, Clone, Copy, PartialEq, Eq)]
#[rullst_enum(type_name = "mariadb_account_status", rename_all = "snake_case")]
enum AccountStatus {
    AwaitingReview,
    Active,
}

#[tokio::test]
async fn test_matrix_mariadb_crud() {
    let container = match Mariadb::default().start().await {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("MariaDB", error);
            return;
        }
    };
    let host = container
        .get_host()
        .await
        .expect("MariaDB container host should be available");
    let port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("MariaDB container port should be available");
    let connection_string = format!("mysql://root@{host}:{port}/test");

    Orm::init(&connection_string)
        .await
        .expect("ORM should connect to MariaDB through the MySQL protocol");
    Schema::create("mariadb_users", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
        table.string("email").not_null();
    })
    .await
    .expect("MariaDB schema should be created");

    let mut user = User {
        id: 0,
        name: "Alice MariaDB".into(),
        email: "alice@mariadb.test".into(),
    };
    user.save().await.expect("MariaDB insert should succeed");
    assert!(user.id > 0);

    let found = User::find(user.id)
        .await
        .expect("MariaDB select should execute")
        .expect("inserted MariaDB row should exist");
    assert_eq!(found.email, "alice@mariadb.test");

    user.name = "Alice MariaDB Updated".into();
    user.save().await.expect("MariaDB update should succeed");
    assert_eq!(
        User::find(user.id)
            .await
            .expect("MariaDB select after update should execute")
            .expect("updated MariaDB row should exist")
            .name,
        "Alice MariaDB Updated"
    );

    user.delete().await.expect("MariaDB delete should succeed");
    assert!(
        User::find(user.id)
            .await
            .expect("MariaDB select after delete should execute")
            .is_none()
    );

    support::exercise_outbox().await;
    exercise_native_enum().await;
}

async fn exercise_native_enum() {
    Schema::create("mariadb_enum_accounts", |table: &mut Blueprint| {
        table.id();
        table.native_enum::<AccountStatus>("status").not_null();
    })
    .await
    .expect("MariaDB inline enum schema should be created");

    let pool = Orm::pool().expect("MariaDB pool");
    let mut connection = pool.acquire().await.expect("MariaDB connection");
    sqlx::query("SET SESSION sql_mode = 'STRICT_ALL_TABLES'")
        .execute(&mut *connection)
        .await
        .expect("strict MariaDB enum validation should be enabled");
    sqlx::query("INSERT INTO mariadb_enum_accounts (status) VALUES (?)")
        .bind(AccountStatus::Active)
        .execute(&mut *connection)
        .await
        .expect("MariaDB enum should encode");
    let stored = sqlx::query_scalar::<_, AccountStatus>(
        "SELECT status FROM mariadb_enum_accounts WHERE id = 1",
    )
    .fetch_one(&mut *connection)
    .await
    .expect("MariaDB enum should decode");
    assert_eq!(stored, AccountStatus::Active);

    let invalid = sqlx::query("INSERT INTO mariadb_enum_accounts (status) VALUES (?)")
        .bind("retired")
        .execute(&mut *connection)
        .await;
    assert!(invalid.is_err(), "MariaDB ENUM must reject unknown labels");
    drop(connection);

    Schema::drop_if_exists("mariadb_enum_accounts")
        .await
        .expect("MariaDB enum table should be dropped");
}
