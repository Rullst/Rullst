use super::support::*;
use std::{sync::Arc, time::Duration};

pub async fn run(url: &str) {
    let namespace = unique();
    let clock = Arc::new(Clock::new());
    let service = ApiTokenService::initialize(
        url,
        keys(),
        ApiTokenConfig::new(&namespace, scopes(), 2, 3600).unwrap(),
    )
    .await
    .unwrap();
    let (owner, _, _) = account(&service, &clock).await;
    let first = service
        .issue(&owner, read(), label(), 60, clock.as_ref())
        .await
        .unwrap();
    let second = service
        .issue(&owner, read(), label(), 600, clock.as_ref())
        .await
        .unwrap();
    assert_eq!(
        service
            .issue(&owner, read(), label(), 600, clock.as_ref())
            .await
            .unwrap_err(),
        RecoveryError::Limited
    );
    let raw = sqlx::AnyPool::connect(url).await.unwrap();
    let digest: String = sqlx::query_scalar(
        "SELECT token_digest FROM rullst_api_tokens WHERE namespace = $1 AND id = $2",
    )
    .bind(&namespace)
    .bind(second.metadata().id().as_str())
    .fetch_one(&raw)
    .await
    .unwrap();
    assert_ne!(digest, second.expose_bearer());
    assert_eq!(digest.len(), 43);
    let ddl = if url.starts_with("postgres") {
        "ALTER TABLE rullst_api_tokens ADD CONSTRAINT api_rotation_failure CHECK(revision = 1) NOT VALID"
    } else {
        "CREATE TRIGGER api_rotation_failure BEFORE UPDATE ON rullst_api_tokens BEGIN SELECT RAISE(ABORT,'fixture'); END"
    };
    sqlx::query(ddl).execute(&raw).await.unwrap();
    assert_eq!(
        service
            .rotate(&owner, second.metadata().id(), 1, 600, clock.as_ref())
            .await
            .unwrap_err(),
        RecoveryError::Storage
    );
    assert!(
        service
            .verify(second.expose_bearer(), &read(), clock.as_ref())
            .await
            .is_ok()
    );
    let cleanup = if url.starts_with("postgres") {
        "ALTER TABLE rullst_api_tokens DROP CONSTRAINT api_rotation_failure"
    } else {
        "DROP TRIGGER api_rotation_failure"
    };
    sqlx::query(cleanup).execute(&raw).await.unwrap();
    // Expiry and cancellation are observed after an actual database lock wait.
    let mut lock = raw.begin().await.unwrap();
    sqlx::query("UPDATE rullst_recovery_control SET attempts = attempts WHERE id = 'write'")
        .execute(&mut *lock)
        .await
        .unwrap();
    let waiting = {
        let service = service.clone();
        let clock = clock.clone();
        let bearer = first.expose_bearer().to_owned();
        tokio::spawn(async move { service.verify(&bearer, &read(), clock.as_ref()).await })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!waiting.is_finished());
    clock.advance(61);
    lock.commit().await.unwrap();
    assert_eq!(
        waiting.await.unwrap().unwrap_err(),
        RecoveryError::InvalidAction
    );
    let mut lock = raw.begin().await.unwrap();
    sqlx::query("UPDATE rullst_recovery_control SET attempts = attempts WHERE id = 'write'")
        .execute(&mut *lock)
        .await
        .unwrap();
    let cancelled = {
        let service = service.clone();
        let clock = clock.clone();
        let bearer = second.expose_bearer().to_owned();
        tokio::spawn(async move { service.verify(&bearer, &read(), clock.as_ref()).await })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!cancelled.is_finished());
    cancelled.abort();
    assert!(cancelled.await.unwrap_err().is_cancelled());
    lock.rollback().await.unwrap();
    assert!(
        service
            .verify(second.expose_bearer(), &read(), clock.as_ref())
            .await
            .is_ok()
    );
    service.purge_expired(clock.as_ref()).await.unwrap();
    let third = service
        .issue(&owner, read(), label(), 600, clock.as_ref())
        .await
        .unwrap();
    assert_eq!(service.revoke_all(&owner, clock.as_ref()).await.unwrap(), 2);
    assert!(
        service
            .verify(third.expose_bearer(), &read(), clock.as_ref())
            .await
            .is_err()
    );
    assert_eq!(service.revoke_all(&owner, clock.as_ref()).await.unwrap(), 0);
    raw.close().await;
    service.close().await;
    // The per-account bound remains independent from the deployment row quota.
    let other = ApiTokenService::initialize(url, keys(), config(&unique()))
        .await
        .unwrap();
    let (owner, _, _) = account(&other, &clock).await;
    for _ in 0..20 {
        other
            .issue(&owner, read(), label(), 600, clock.as_ref())
            .await
            .unwrap();
    }
    assert_eq!(
        other
            .issue(&owner, read(), label(), 600, clock.as_ref())
            .await
            .unwrap_err(),
        RecoveryError::Limited
    );
    other.close().await;
    assert_eq!(other_verify_closed(url).await, RecoveryError::Storage);
}
async fn other_verify_closed(url: &str) -> RecoveryError {
    let clock = Clock::new();
    let service = ApiTokenService::initialize(url, keys(), config(&unique()))
        .await
        .unwrap();
    let (owner, _, _) = account(&service, &clock).await;
    let token = service
        .issue(&owner, read(), label(), 600, &clock)
        .await
        .unwrap();
    service.clone().close().await;
    service
        .verify(token.expose_bearer(), &read(), &clock)
        .await
        .unwrap_err()
}
