//! Nexus CRUD against MariaDB (Docker matrix). MariaDB rejects an offset in
//! a date-time literal and nulls `TIMESTAMP` parameters outside 1970–2038,
//! so its `DATETIME` columns show how Nexus binds UTC instants there.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-sqlite")))]

mod sql_matrix;
mod support;

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mariadb::Mariadb;

#[tokio::test]
async fn nexus_crud_matches_mariadb_collations_and_types() {
    let container = match Mariadb::default().start().await {
        Ok(container) => container,
        Err(error) => {
            sql_matrix::handle_container_start_error("MariaDB", error);
            return;
        }
    };
    let host = container.get_host().await.expect("MariaDB host");
    let port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("MariaDB port");
    sql_matrix::exercise(&format!("mysql://root@{host}:{port}/test"), "mysql").await;
}
