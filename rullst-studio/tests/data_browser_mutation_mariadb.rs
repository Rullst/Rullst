#![cfg(all(
    feature = "strict-mysql",
    not(feature = "strict-postgres"),
    not(feature = "strict-sqlite")
))]

mod support;

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mariadb::Mariadb;

#[tokio::test]
async fn data_browser_mutates_mariadb_by_primary_key() {
    let container = match Mariadb::default().start().await {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("MariaDB", error);
            return;
        }
    };
    let host = container.get_host().await.expect("MariaDB host");
    let port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("MariaDB port");
    support::exercise_mutations(
        &format!("mysql://root@{host}:{port}/test"),
        "mysql",
        "studio_mutation_mariadb",
    )
    .await;
}
