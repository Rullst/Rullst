#![cfg(feature = "sql-memory")]

mod sql_chat_memory_support;

use rullst_ai::SqlChatBackend;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mariadb::Mariadb;

#[tokio::test]
async fn sql_chat_memory_orders_mariadb_exchanges() {
    let container = match Mariadb::default().start().await {
        Ok(container) => container,
        Err(error) => {
            sql_chat_memory_support::handle_container_start_error("MariaDB", error);
            return;
        }
    };
    let host = container.get_host().await.expect("MariaDB host");
    let port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("MariaDB port");
    let database_url = format!("mysql://root@{host}:{port}/test");
    sql_chat_memory_support::exercise_sql_chat_memory(&database_url, SqlChatBackend::Mysql).await;
    sql_chat_memory_support::exercise_case_sensitive_mysql_keys(&database_url).await;
}
