#![cfg(feature = "sql-memory")]

mod sql_chat_memory_support;

use rullst_ai::SqlChatBackend;
use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;

#[tokio::test]
async fn sql_chat_memory_orders_mysql_exchanges() {
    let container = match Mysql::default()
        .with_tag("8.0")
        .with_env_var("MYSQL_ROOT_PASSWORD", "root")
        .with_env_var("MYSQL_DATABASE", "testdb")
        .start()
        .await
    {
        Ok(container) => container,
        Err(error) => {
            sql_chat_memory_support::handle_container_start_error("MySQL", error);
            return;
        }
    };
    let host = container.get_host().await.expect("MySQL host");
    let port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("MySQL port");
    let database_url = format!("mysql://root:root@{host}:{port}/testdb");
    sql_chat_memory_support::exercise_sql_chat_memory(&database_url, SqlChatBackend::Mysql).await;
    sql_chat_memory_support::exercise_case_sensitive_mysql_keys(&database_url).await;
}
