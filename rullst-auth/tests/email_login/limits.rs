use super::support::*;

pub async fn run(url: &str) {
    let namespace = unique();
    let clock = Clock::new();
    let config =
        EmailLoginConfig::new(&namespace, "https://app.example/login", "/dashboard", 1).unwrap();
    let service = EmailLoginService::initialize(url, keys(), config)
        .await
        .unwrap();
    let (_, email, password) = account(&service, &clock).await;
    let browser = BrowserBinding::generate().unwrap();
    let proof = service
        .accounts()
        .authenticate(&email, &password)
        .await
        .unwrap()
        .unwrap();
    let second = unique();
    let second_email = format!("{second}@example.com");
    service
        .accounts()
        .register_account(&second, &second_email, &password, clock.now().unwrap())
        .await
        .unwrap();
    service
        .request_login(&second_email, &browser, &clock)
        .await
        .unwrap();
    assert!(service.claim_notice(&clock).await.unwrap().is_none());
    let second_proof = service
        .accounts()
        .authenticate(&second_email, &password)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        service
            .set_account_enabled(&second_proof, true, &clock)
            .await,
        Err(RecoveryError::Limited)
    );
    let claim = issue(&service, &email, &browser, &clock).await;
    let token = token(&claim);
    // The session bound is shared with password-created sessions. Capacity failure
    // rolls back the link consumption; releasing one slot permits one redemption.
    let mut sessions = Vec::new();
    for _ in 0..20 {
        sessions.push(
            service
                .accounts()
                .create_session(&proof, clock.now().unwrap(), 3600)
                .await
                .unwrap(),
        );
    }
    assert_eq!(
        service.redeem(&token, &browser, &clock).await.unwrap_err(),
        RecoveryError::Limited
    );
    service
        .accounts()
        .revoke_session(sessions[0].expose())
        .await
        .unwrap();
    // Failed redemptions are charged to the presenting browser binding only after
    // the token lookup fails. Sixty failures from one unauthenticated client
    // throttle that client alone and never block another client's valid link.
    let attacker = BrowserBinding::generate().unwrap();
    for _ in 0..60 {
        assert_eq!(
            service
                .redeem("invalid", &attacker, &clock)
                .await
                .unwrap_err(),
            RecoveryError::InvalidAction
        );
    }
    assert_eq!(
        service.redeem(&token, &attacker, &clock).await.unwrap_err(),
        RecoveryError::Throttled
    );
    let session = service.redeem(&token, &browser, &clock).await.unwrap();
    assert_eq!(
        service.redeem(&token, &browser, &clock).await.unwrap_err(),
        RecoveryError::InvalidAction
    );
    // A browser's own malformed, wrong or replayed redemptions (the replay above
    // included) throttle it, even for a valid link, until its window passes.
    service
        .accounts()
        .revoke_session(session.token().expose())
        .await
        .unwrap();
    let retry = super::support::token(&issue(&service, &email, &browser, &clock).await);
    for _ in 0..59 {
        assert!(service.redeem("invalid", &browser, &clock).await.is_err());
    }
    assert_eq!(
        service.redeem(&retry, &browser, &clock).await.unwrap_err(),
        RecoveryError::Throttled
    );
    clock.advance(60);
    assert_eq!(
        service
            .redeem("invalid", &attacker, &clock)
            .await
            .unwrap_err(),
        RecoveryError::InvalidAction
    );
    assert!(service.redeem(&retry, &browser, &clock).await.is_ok());
    // Retention remains operable without another incoming authentication request.
    clock.advance(3600);
    service.purge_expired(&clock).await.unwrap();
    service.close().await;
}
