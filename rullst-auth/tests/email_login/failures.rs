use super::support::*;
use sqlx::Row;

pub async fn run(url: &str) {
    let namespace = unique();
    let clock = Clock::new();
    let service = EmailLoginService::initialize(url, keys(), config(&namespace))
        .await
        .unwrap();
    let (subject, email, password) = account(&service, &clock).await;
    let browser = BrowserBinding::generate().unwrap();
    let first = issue(&service, &email, &browser, &clock).await;
    let first_id = first.delivery_id().to_owned();
    let first_token = token(&first);
    assert!(service.claim_notice(&clock).await.unwrap().is_none());
    clock.advance(60);
    let retry = service.claim_notice(&clock).await.unwrap().unwrap();
    assert_eq!(retry.delivery_id(), first_id);
    assert_eq!(token(&retry), first_token);
    assert!(service.complete_notice(&first, &clock).await.is_err());
    service
        .fail_notice(&retry, RecoveryDeliveryFailure::Transient, &clock)
        .await
        .unwrap();
    assert!(service.claim_notice(&clock).await.unwrap().is_none());
    clock.advance(60);
    let retry = service.claim_notice(&clock).await.unwrap().unwrap();
    assert_eq!(retry.delivery_id(), first_id);
    service.complete_notice(&retry, &clock).await.unwrap();
    // The owned database contains no raw emailed or browser secret.
    let db = sqlx::AnyPool::connect(url).await.unwrap();
    let row = sqlx::query("SELECT token_digest,browser_digest FROM rullst_email_login_tokens WHERE namespace = $1 AND subject = $2")
        .bind(&namespace).bind(&subject).fetch_one(&db).await.unwrap();
    assert_ne!(row.get::<String, _>("token_digest"), first_token);
    assert_ne!(
        row.get::<String, _>("browser_digest"),
        browser.expose_cookie()
    );
    // Account epoch changes invalidate pending links and prevent queued delivery.
    sqlx::query("UPDATE rullst_recovery_accounts SET session_version = session_version + 1 WHERE subject = $1")
        .bind(&subject).execute(&db).await.unwrap();
    assert!(
        service
            .redeem(&first_token, &browser, &clock)
            .await
            .is_err()
    );
    let _new = issue(&service, &email, &browser, &clock).await;
    let newest = issue(&service, &email, &browser, &clock).await;
    assert_eq!(
        service
            .request_login(&email, &browser, &clock)
            .await
            .unwrap(),
        LoginRequestAccepted
    );
    // Fourth request in the same account window must not replace the third.
    let session = service
        .redeem(&token(&newest), &browser, &clock)
        .await
        .unwrap();
    service
        .accounts()
        .revoke_session(session.token().expose())
        .await
        .unwrap();
    clock.advance(900);
    let claim = issue(&service, &email, &browser, &clock).await;
    // Fail the second half of redemption. The first half must not consume the link.
    let ddl = if url.starts_with("postgres") {
        "ALTER TABLE rullst_recovery_session_details ADD CONSTRAINT email_login_failure CHECK (created_at < 0) NOT VALID"
    } else {
        "CREATE TRIGGER email_login_failure BEFORE INSERT ON rullst_recovery_session_details BEGIN SELECT RAISE(ABORT,'fixture'); END"
    };
    sqlx::query(ddl).execute(&db).await.unwrap();
    assert_eq!(
        service
            .redeem(&token(&claim), &browser, &clock)
            .await
            .unwrap_err(),
        RecoveryError::Storage
    );
    let cleanup = if url.starts_with("postgres") {
        "ALTER TABLE rullst_recovery_session_details DROP CONSTRAINT email_login_failure"
    } else {
        "DROP TRIGGER email_login_failure"
    };
    sqlx::query(cleanup).execute(&db).await.unwrap();
    let session = service
        .redeem(&token(&claim), &browser, &clock)
        .await
        .unwrap();
    assert_eq!(
        service
            .accounts()
            .verify_session(session.token().expose(), clock.now().unwrap())
            .await
            .unwrap(),
        Some(subject.clone())
    );
    // A permanent bounce suppresses future login mail and invalidates pending links.
    let bounced = issue(&service, &email, &browser, &clock).await;
    service
        .fail_notice(&bounced, RecoveryDeliveryFailure::PermanentBounce, &clock)
        .await
        .unwrap();
    assert!(
        service
            .redeem(&token(&bounced), &browser, &clock)
            .await
            .is_err()
    );
    service
        .request_login(&email, &browser, &clock)
        .await
        .unwrap();
    assert!(service.claim_notice(&clock).await.unwrap().is_none());
    // Password authentication still works; the mail failure is not account deletion.
    assert!(
        service
            .accounts()
            .authenticate(&email, &password)
            .await
            .unwrap()
            .is_some()
    );
    db.close().await;
    service.close().await;
}
