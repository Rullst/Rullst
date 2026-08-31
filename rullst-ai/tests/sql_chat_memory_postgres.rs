#![cfg(feature = "sql-memory")]

mod sql_chat_memory_support;

use rullst_ai::SqlChatBackend;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn sql_chat_memory_orders_postgres_exchanges() {
    let container = match Postgres::default().start().await {
        Ok(container) => container,
        Err(error) => {
            sql_chat_memory_support::handle_container_start_error("PostgreSQL", error);
            return;
        }
    };
    let host = container.get_host().await.expect("PostgreSQL host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("PostgreSQL port");
    sql_chat_memory_support::exercise_sql_chat_memory(
        &format!("postgres://postgres:postgres@{host}:{port}/postgres"),
        SqlChatBackend::Postgres,
    )
    .await;
}
