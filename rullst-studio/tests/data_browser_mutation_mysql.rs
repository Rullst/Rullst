#![cfg(all(
    feature = "strict-mysql",
    not(feature = "strict-postgres"),
    not(feature = "strict-sqlite")
))]

mod support;

use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;

#[tokio::test]
async fn data_browser_mutates_mysql_by_primary_key() {
    let container = match Mysql::default()
        .with_tag("8.0")
        .with_env_var("MYSQL_ROOT_PASSWORD", "root")
        .with_env_var("MYSQL_DATABASE", "testdb")
        .start()
        .await
    {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("MySQL", error);
            return;
        }
    };
    let host = container.get_host().await.expect("MySQL host");
    let port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("MySQL port");
    support::exercise_mutations(
        &format!("mysql://root:root@{host}:{port}/testdb"),
        "mysql",
        "studio_mutation_mysql",
    )
    .await;
}
