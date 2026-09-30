use super::support::*;
use std::{sync::Arc, time::Duration};

pub async fn run(url: &str) {
    let namespace = unique();
    let clock = Arc::new(Clock::new());
    let service = EmailLoginService::initialize(url, keys(), config(&namespace))
        .await
        .unwrap();
    let (_, email, _) = account(&service, clock.as_ref()).await;
    let browser = BrowserBinding::generate().unwrap();
    let pending = issue(&service, &email, &browser, clock.as_ref()).await;
    let token = token(&pending);
    let raw = sqlx::AnyPool::connect(url).await.unwrap();
    let mut lock = raw.begin().await.unwrap();
    sqlx::query("UPDATE rullst_recovery_control SET attempts = attempts WHERE id = 'write'")
        .execute(&mut *lock)
        .await
        .unwrap();
    let waiting = {
        let service = service.clone();
        let clock = clock.clone();
        let token = token.clone();
        let browser = BrowserBinding::from_cookie(browser.expose_cookie()).unwrap();
        tokio::spawn(async move { service.redeem(&token, &browser, clock.as_ref()).await })
    };
    // This future reaches an actual contended database write. It must re-read
    // trusted time after acquiring the lock instead of trusting the start time.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!waiting.is_finished());
    clock.advance(901);
    lock.commit().await.unwrap();
    assert_eq!(
        waiting.await.unwrap().unwrap_err(),
        RecoveryError::InvalidAction
    );
    let pending = issue(&service, &email, &browser, clock.as_ref()).await;
    let current = token_from(&pending);
    let mut lock = raw.begin().await.unwrap();
    sqlx::query("UPDATE rullst_recovery_control SET attempts = attempts WHERE id = 'write'")
        .execute(&mut *lock)
        .await
        .unwrap();
    let cancelled = {
        let service = service.clone();
        let clock = clock.clone();
        let current = current.clone();
        let browser = BrowserBinding::from_cookie(browser.expose_cookie()).unwrap();
        tokio::spawn(async move { service.redeem(&current, &browser, clock.as_ref()).await })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!cancelled.is_finished());
    cancelled.abort();
    assert!(cancelled.await.unwrap_err().is_cancelled());
    lock.rollback().await.unwrap();
    assert!(
        service
            .redeem(&current, &browser, clock.as_ref())
            .await
            .is_ok()
    );
    raw.close().await;
    service.close().await;
    clock_skew(url).await;
}
/// Two hosts sharing the database cross each whole second at slightly
/// different instants: one trailing the recorded time by up to five seconds
/// adopts it, a larger regression still fails closed.
async fn clock_skew(url: &str) {
    let namespace = unique();
    let ahead = Clock::new();
    let service = EmailLoginService::initialize(url, keys(), config(&namespace))
        .await
        .unwrap();
    let peer = EmailLoginService::connect(url, keys(), config(&namespace))
        .await
        .unwrap();
    let (_, email, _) = account(&service, &ahead).await;
    let browser = BrowserBinding::generate().unwrap();
    ahead.advance(1);
    let pending = issue(&service, &email, &browser, &ahead).await;
    let behind = Clock::new();
    assert!(
        peer.redeem(&token(&pending), &browser, &behind)
            .await
            .is_ok()
    );
    behind.set(1_800_000_000 - 4);
    assert!(peer.purge_expired(&behind).await.is_ok());
    behind.set(1_800_000_000 - 5);
    assert_eq!(
        peer.purge_expired(&behind).await,
        Err(RecoveryError::InvalidAction)
    );
    peer.close().await;
    service.close().await;
}
fn token_from(delivery: &EmailLoginDelivery) -> String {
    super::support::token(delivery)
}
