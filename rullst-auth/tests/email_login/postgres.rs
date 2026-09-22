use super::support::*;
use sqlx::Row;

pub async fn run(url: &str) {
    let raw = sqlx::PgPool::connect(url).await.unwrap();
    let namespace = unique();
    // Deployment bootstraps serialize even when independent processes/pools
    // observe the same namespace before it has a metadata row.
    let (first, second) = tokio::join!(
        EmailLoginService::initialize(url, keys(), config(&namespace)),
        EmailLoginService::initialize(url, keys(), config(&namespace))
    );
    let first = first.unwrap();
    let second = second.unwrap();
    first.close().await;
    second.close().await;
    sqlx::query("CREATE ROLE rullst_email_login_runtime LOGIN")
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query("GRANT USAGE ON SCHEMA public TO rullst_email_login_runtime")
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query("GRANT SELECT,INSERT,UPDATE,DELETE ON rullst_recovery_control,rullst_recovery_accounts,rullst_recovery_tokens,rullst_recovery_sessions,rullst_recovery_session_details,rullst_recovery_outbox,rullst_email_login_control,rullst_email_login_accounts,rullst_email_login_tokens,rullst_email_login_outbox TO rullst_email_login_runtime").execute(&raw).await.unwrap();
    let mut runtime = url::Url::parse(url).unwrap();
    runtime.set_username("rullst_email_login_runtime").unwrap();
    let service = EmailLoginService::connect(runtime.as_str(), keys(), config(&namespace))
        .await
        .unwrap();
    let clock = Clock::new();
    let (_, email, _) = account(&service, &clock).await;
    let browser = BrowserBinding::generate().unwrap();
    let notice = issue(&service, &email, &browser, &clock).await;
    assert!(
        service
            .redeem(&token(&notice), &browser, &clock)
            .await
            .is_ok()
    );
    let role = sqlx::PgPool::connect(runtime.as_str()).await.unwrap();
    assert!(
        sqlx::query("ALTER TABLE rullst_email_login_tokens ADD COLUMN forbidden TEXT")
            .execute(&role)
            .await
            .is_err()
    );
    role.close().await;
    // Unlogged authoritative state cannot support one-use guarantees after a
    // database crash. Existing services must reject it, not only new startup.
    sqlx::query("ALTER TABLE rullst_email_login_tokens SET UNLOGGED")
        .execute(&raw)
        .await
        .unwrap();
    assert_eq!(
        service.request_login(&email, &browser, &clock).await,
        Err(RecoveryError::Configuration)
    );
    assert!(
        EmailLoginService::connect(url, keys(), config(&namespace))
            .await
            .is_err()
    );
    sqlx::query("ALTER TABLE rullst_email_login_tokens SET LOGGED")
        .execute(&raw)
        .await
        .unwrap();
    let notice = issue(&service, &email, &browser, &clock).await;
    assert!(
        service
            .redeem(&token(&notice), &browser, &clock)
            .await
            .is_ok()
    );
    // PostgreSQL durability settings are read on every operation. Scope this
    // temporary configuration change to the disposable owned cluster only.
    sqlx::query("ALTER SYSTEM SET full_page_writes = off")
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query("SELECT pg_reload_conf()")
        .execute(&raw)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let row = sqlx::query("SELECT current_setting('full_page_writes') AS enabled")
                .fetch_one(&raw)
                .await
                .unwrap();
            if row.get::<String, _>("enabled") == "off" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        service.request_login(&email, &browser, &clock).await,
        Err(RecoveryError::Configuration)
    );
    sqlx::query("ALTER SYSTEM RESET full_page_writes")
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query("SELECT pg_reload_conf()")
        .execute(&raw)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let row = sqlx::query("SELECT current_setting('full_page_writes') AS enabled")
                .fetch_one(&raw)
                .await
                .unwrap();
            if row.get::<String, _>("enabled") == "on" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    service.close().await;
    sqlx::query("DROP OWNED BY rullst_email_login_runtime")
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query("DROP ROLE rullst_email_login_runtime")
        .execute(&raw)
        .await
        .unwrap();
    raw.close().await;
}
