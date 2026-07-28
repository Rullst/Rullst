#![cfg(not(any(feature = "strict-sqlite", feature = "strict-mysql")))]

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "pg_users")]
struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[tokio::test]
async fn test_matrix_postgres_crud() {
    // 1. Inicia o container do PostgreSQL
    let container = Postgres::default()
        .start()
        .await
        .expect("Failed to start Postgres container");

    let host_ip = container.get_host().await.expect("Failed to get host IP");
    let host_port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get port");

    let connection_string = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        host_ip, host_port
    );

    // 2. Inicializa o ORM com o Postgres real
    Orm::init(&connection_string)
        .await
        .expect("Orm::init should succeed with Postgres");

    // 3. Cria a tabela (Schema Builder deve funcionar no PG)
    Schema::create("pg_users", |t: &mut Blueprint| {
        t.id();
        t.string("name").not_null();
        t.string("email").not_null();
    })
    .await
    .expect("create pg_users");

    // 4. Executa um CRUD básico para provar que a gramática gerada estaticamente funciona
    let mut user = User {
        id: 0,
        name: "Alice PG".into(),
        email: "alice@pg.com".into(),
    };

    // INSERT
    user.save().await.expect("save new user to postgres");
    assert!(user.id > 0, "id must be assigned after save (RETURNING id)");

    // SELECT
    let found = User::find(user.id)
        .await
        .expect("find query executed")
        .expect("user exists");
    assert_eq!(found.name, "Alice PG");

    // UPDATE
    user.name = "Alice PG Updated".into();
    user.save().await.expect("update user in postgres");

    let updated = User::find(user.id).await.unwrap().unwrap();
    assert_eq!(updated.name, "Alice PG Updated");

    // DELETE
    user.delete().await.expect("delete executed");

    let not_found = User::find(user.id).await.unwrap();
    assert!(not_found.is_none());
}
