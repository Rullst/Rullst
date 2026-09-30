//! Nexus CRUD against MySQL (Docker matrix).
#![cfg(not(any(feature = "strict-postgres", feature = "strict-sqlite")))]

mod sql_matrix;
mod support;

use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;

#[tokio::test]
async fn nexus_crud_matches_mysql_collations_and_types() {
    // Pin to MySQL 8.0: the module's authentication flag was removed in 8.4+.
    let container = match Mysql::default()
        .with_tag("8.0")
        .with_env_var("MYSQL_ROOT_PASSWORD", "root")
        .with_env_var("MYSQL_DATABASE", "testdb")
        .start()
        .await
    {
        Ok(container) => container,
        Err(error) => {
            sql_matrix::handle_container_start_error("MySQL", error);
            return;
        }
    };
    let host = container.get_host().await.expect("MySQL host");
    let port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("MySQL port");
    sql_matrix::exercise(&format!("mysql://root:root@{host}:{port}/testdb"), "mysql").await;
}
