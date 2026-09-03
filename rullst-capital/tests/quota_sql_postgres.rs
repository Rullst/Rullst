#![cfg(feature = "quota-sql")]

mod quota_sql_live_support;

use rullst_capital::SqlQuotaBackend;
#[cfg(feature = "webhook-sql")]
use rullst_capital::SqlWebhookBackend;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn shared_quota_is_atomic_on_postgres() {
    let container = match Postgres::default().start().await {
        Ok(container) => container,
        Err(error) => {
            quota_sql_live_support::handle_container_start_error("PostgreSQL", error);
            return;
        }
    };
    let host = container.get_host().await.expect("PostgreSQL host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("PostgreSQL port");
    quota_sql_live_support::exercise_sql_quota(
        &format!("postgres://postgres:postgres@{host}:{port}/postgres"),
        SqlQuotaBackend::Postgres,
    )
    .await;
    #[cfg(feature = "webhook-sql")]
    quota_sql_live_support::exercise_sql_webhook_replay(
        &format!("postgres://postgres:postgres@{host}:{port}/postgres"),
        SqlWebhookBackend::Postgres,
    )
    .await;
}
