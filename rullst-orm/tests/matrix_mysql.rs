#![cfg(not(any(feature = "strict-sqlite", feature = "strict-postgres")))]

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mysql::Mysql;

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "mysql_users")]
struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[tokio::test]
async fn test_matrix_mysql_crud() {
    // 1. Inicia o container do MySQL
    let container = Mysql::default()
        .with_env_var("MYSQL_ROOT_PASSWORD", "root")
        .with_env_var("MYSQL_DATABASE", "testdb")
        .with_cmd(["--default-authentication-plugin=mysql_native_password"])
        .start()
        .await
        .expect("Failed to start MySQL container");

    let host_ip = container.get_host().await.expect("Failed to get host IP");
    let host_port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("Failed to get port");

    let connection_string = format!("mysql://root:root@{}:{}/testdb", host_ip, host_port);

    // 2. Inicializa o ORM com o MySQL real
    Orm::init(&connection_string)
        .await
        .expect("Orm::init should succeed with MySQL");

    // 3. Cria a tabela (Schema Builder deve funcionar no MySQL)
    Schema::create("mysql_users", |t: &mut Blueprint| {
        t.id();
        t.string("name").not_null();
        t.string("email").not_null();
    })
    .await
    .expect("create mysql_users");

    // 4. Executa um CRUD básico
    let mut user = User {
        id: 0,
        name: "Alice MySQL".into(),
        email: "alice@mysql.com".into(),
    };

    // INSERT
    user.save().await.expect("save new user to mysql");
    assert!(
        user.id > 0,
        "id must be assigned after save (LAST_INSERT_ID)"
    );

    // SELECT
    let found = User::find(user.id)
        .await
        .expect("find query executed")
        .expect("user exists");
    assert_eq!(found.name, "Alice MySQL");

    // UPDATE
    user.name = "Alice MySQL Updated".into();
    user.save().await.expect("update user in mysql");

    let updated = User::find(user.id).await.unwrap().unwrap();
    assert_eq!(updated.name, "Alice MySQL Updated");

    // DELETE
    user.delete().await.expect("delete executed");

    let not_found = User::find(user.id).await.unwrap();
    assert!(not_found.is_none());
}
