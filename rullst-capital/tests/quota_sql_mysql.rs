#![cfg(feature = "quota-sql")]

mod quota_sql_live_support;

use rullst_capital::SqlQuotaBackend;
#[cfg(feature = "webhook-sql")]
use rullst_capital::SqlWebhookBackend;
use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;

#[tokio::test]
async fn shared_quota_is_atomic_on_mysql() {
    let container = match Mysql::default()
        .with_tag("8.0")
        .with_env_var("MYSQL_ROOT_PASSWORD", "root")
        .with_env_var("MYSQL_DATABASE", "testdb")
        .start()
        .await
    {
        Ok(container) => container,
        Err(error) => {
            quota_sql_live_support::handle_container_start_error("MySQL", error);
            return;
        }
    };
    let host = container.get_host().await.expect("MySQL host");
    let port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("MySQL port");
    quota_sql_live_support::exercise_sql_quota(
        &format!("mysql://root:root@{host}:{port}/testdb"),
        SqlQuotaBackend::Mysql,
    )
    .await;
    #[cfg(feature = "webhook-sql")]
    quota_sql_live_support::exercise_sql_webhook_replay(
        &format!("mysql://root:root@{host}:{port}/testdb"),
        SqlWebhookBackend::Mysql,
    )
    .await;
}
