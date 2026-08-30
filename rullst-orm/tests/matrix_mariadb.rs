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
}
