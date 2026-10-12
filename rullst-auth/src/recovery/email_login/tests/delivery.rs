use super::*;

#[tokio::test]
async fn deliveries_keep_their_id_and_locale_and_retry_after_backoff() {
    let (_directory, _, service) = service().await;
    let clock = Clock::at(T);
    let (email, _) = enabled(&service, &clock, "owner-1", Some(RecoveryLocale::PtBr)).await;
    let browser = BrowserBinding::generate().unwrap();
    service
        .request_login(&email, &browser, &clock)
        .await
        .unwrap();
    let first = service.claim_notice(&clock).await.unwrap().unwrap();
    assert_eq!(first.delivery_id().len(), 43);
    assert_eq!(first.locale(), Some(RecoveryLocale::PtBr));
    assert_eq!(format!("{first:?}"), "EmailLoginDelivery([REDACTED])");
    service
        .fail_notice(&first, RecoveryDeliveryFailure::Transient, &clock)
        .await
        .unwrap();
    // The first retry waits 15 * 2 seconds.
    clock.set(T + 29);
    assert!(service.claim_notice(&clock).await.unwrap().is_none());
    clock.set(T + 30);
    let retry = service.claim_notice(&clock).await.unwrap().unwrap();
    assert_eq!(retry.delivery_id(), first.delivery_id());
    service.complete_notice(&retry, &clock).await.unwrap();
    assert_eq!(count(&service, "rullst_email_login_outbox").await, 0);
    clock.set(T + 400);
    assert!(service.claim_notice(&clock).await.unwrap().is_none());
}

#[tokio::test]
async fn a_transient_failure_on_the_sixth_attempt_drops_the_notice() {
    let (_directory, _, service) = service().await;
    let clock = Clock::at(T);
    let (email, _) = enabled(&service, &clock, "owner-1", None).await;
    let browser = BrowserBinding::generate().unwrap();
    service
        .request_login(&email, &browser, &clock)
        .await
        .unwrap();
    execute(
        &service,
        "UPDATE rullst_email_login_outbox SET attempts = 5",
    )
    .await;
    let last = service.claim_notice(&clock).await.unwrap().unwrap();
    service
        .fail_notice(&last, RecoveryDeliveryFailure::Transient, &clock)
        .await
        .unwrap();
    assert_eq!(count(&service, "rullst_email_login_outbox").await, 0);
}

#[tokio::test]
async fn notices_are_bound_to_their_row_token_and_expiry() {
    let (_directory, _, service) = service().await;
    let clock = Clock::at(T);
    let (email, _) = enabled(&service, &clock, "owner-1", None).await;
    let browser = BrowserBinding::generate().unwrap();
    for tamper in [
        "UPDATE rullst_email_login_outbox SET id = 'moved-row'",
        "UPDATE rullst_email_login_tokens SET token_digest = 'tampered'",
        "UPDATE rullst_email_login_outbox SET expires_at = expires_at - 1",
    ] {
        execute(
            &service,
            "UPDATE rullst_email_login_accounts SET requests = 0",
        )
        .await;
        service
            .request_login(&email, &browser, &clock)
            .await
            .unwrap();
        execute(&service, tamper).await;
        assert_eq!(
            service.claim_notice(&clock).await.err(),
            Some(RecoveryError::Crypto),
            "{tamper}"
        );
    }
}

#[tokio::test]
async fn retention_removes_expired_links_and_notices() {
    let (_directory, _, service) = service().await;
    let clock = Clock::at(T);
    let (email, _) = enabled(&service, &clock, "owner-1", None).await;
    let browser = BrowserBinding::generate().unwrap();
    service
        .request_login(&email, &browser, &clock)
        .await
        .unwrap();
    assert_eq!(count(&service, "rullst_email_login_outbox").await, 1);
    assert_eq!(count(&service, "rullst_email_login_tokens").await, 1);
    clock.set(T + 900);
    service.purge_expired(&clock).await.unwrap();
    assert_eq!(count(&service, "rullst_email_login_outbox").await, 0);
    assert_eq!(count(&service, "rullst_email_login_tokens").await, 0);
}
