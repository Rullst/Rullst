//! Nexus CRUD against PostgreSQL (Docker matrix).
#![cfg(not(any(feature = "strict-mysql", feature = "strict-sqlite")))]

mod sql_matrix;
mod support;

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn nexus_crud_matches_postgres_types_and_keys() {
    let container = match Postgres::default().start().await {
        Ok(container) => container,
        Err(error) => {
            sql_matrix::handle_container_start_error("PostgreSQL", error);
            return;
        }
    };
    let host = container.get_host().await.expect("PostgreSQL host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("PostgreSQL port");
    sql_matrix::exercise(
        &format!("postgres://postgres:postgres@{host}:{port}/postgres"),
        "postgres",
    )
    .await;
}
