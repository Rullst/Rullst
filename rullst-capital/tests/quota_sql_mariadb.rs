#![cfg(feature = "quota-sql")]

mod quota_sql_live_support;

use rullst_capital::SqlQuotaBackend;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mariadb::Mariadb;

#[tokio::test]
async fn shared_quota_is_atomic_on_mariadb() {
    let container = match Mariadb::default().start().await {
        Ok(container) => container,
        Err(error) => {
            quota_sql_live_support::handle_container_start_error("MariaDB", error);
            return;
        }
    };
    let host = container.get_host().await.expect("MariaDB host");
    let port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("MariaDB port");
    quota_sql_live_support::exercise_sql_quota(
        &format!("mysql://root@{host}:{port}/test"),
        SqlQuotaBackend::Mysql,
    )
    .await;
}
