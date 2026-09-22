use super::support::*;

pub async fn run(url: &str) {
    let raw = sqlx::PgPool::connect(url).await.unwrap();
    let namespace = unique();
    let (first, second) = tokio::join!(
        ApiTokenService::initialize(url, keys(), config(&namespace)),
        ApiTokenService::initialize(url, keys(), config(&namespace))
    );
    first.unwrap().close().await;
    second.unwrap().close().await;
    sqlx::query("CREATE ROLE rullst_api_token_runtime LOGIN")
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query("GRANT USAGE ON SCHEMA public TO rullst_api_token_runtime")
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query("GRANT SELECT,INSERT,UPDATE,DELETE ON rullst_recovery_control,rullst_recovery_accounts,rullst_recovery_tokens,rullst_recovery_sessions,rullst_recovery_session_details,rullst_recovery_outbox,rullst_api_token_control,rullst_api_tokens TO rullst_api_token_runtime")
        .execute(&raw).await.unwrap();
    let mut runtime = url::Url::parse(url).unwrap();
    runtime.set_username("rullst_api_token_runtime").unwrap();
    let service = ApiTokenService::connect(runtime.as_str(), keys(), config(&namespace))
        .await
        .unwrap();
    let clock = Clock::new();
    let (owner, _, _) = account(&service, &clock).await;
    let token = service
        .issue(&owner, read(), label(), 600, &clock)
        .await
        .unwrap();
    let role = sqlx::PgPool::connect(runtime.as_str()).await.unwrap();
    assert!(
        sqlx::query("ALTER TABLE rullst_api_tokens ADD COLUMN forbidden TEXT")
            .execute(&role)
            .await
            .is_err()
    );
    role.close().await;
    // Every request rechecks durable authoritative storage, including pools
    // that connected before an operator changed the underlying table.
    sqlx::query("ALTER TABLE rullst_api_tokens SET UNLOGGED")
        .execute(&raw)
        .await
        .unwrap();
    assert_eq!(
        service
            .verify(token.expose_bearer(), &read(), &clock)
            .await
            .unwrap_err(),
        RecoveryError::Configuration
    );
    assert!(
        ApiTokenService::connect(url, keys(), config(&namespace))
            .await
            .is_err()
    );
    sqlx::query("ALTER TABLE rullst_api_tokens SET LOGGED")
        .execute(&raw)
        .await
        .unwrap();
    assert!(
        service
            .verify(token.expose_bearer(), &read(), &clock)
            .await
            .is_ok()
    );
    for (setting, expected) in [
        ("ALTER SYSTEM SET full_page_writes = off", "off"),
        ("ALTER SYSTEM RESET full_page_writes", "on"),
    ] {
        sqlx::query(setting).execute(&raw).await.unwrap();
        sqlx::query("SELECT pg_reload_conf()")
            .execute(&raw)
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                let actual: String =
                    sqlx::query_scalar("SELECT current_setting('full_page_writes')")
                        .fetch_one(&raw)
                        .await
                        .unwrap();
                if actual == expected {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        let result = service.verify(token.expose_bearer(), &read(), &clock).await;
        if expected == "off" {
            assert_eq!(result.unwrap_err(), RecoveryError::Configuration);
        } else {
            assert!(result.is_ok());
        }
    }
    let rotated = service
        .rotate(&owner, token.metadata().id(), 1, 600, &clock)
        .await
        .unwrap();
    assert_eq!(service.inventory(&owner, &clock).await.unwrap().len(), 1);
    assert!(
        service
            .revoke(&owner, rotated.metadata().id(), &clock)
            .await
            .unwrap()
    );
    service.close().await;
    sqlx::query("DROP OWNED BY rullst_api_token_runtime")
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query("DROP ROLE rullst_api_token_runtime")
        .execute(&raw)
        .await
        .unwrap();
    raw.close().await;
}
