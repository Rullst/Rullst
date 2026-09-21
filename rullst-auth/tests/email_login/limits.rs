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
    // Durable attempt limit includes malformed and wrong-browser redemptions.
    for _ in 0..60 {
        assert!(service.redeem("invalid", &browser, &clock).await.is_err());
    }
    assert!(service.redeem(&token, &browser, &clock).await.is_err());
    clock.advance(60);
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
    assert!(service.redeem(&token, &browser, &clock).await.is_ok());
    // Retention remains operable without another incoming authentication request.
    clock.advance(3600);
    service.purge_expired(&clock).await.unwrap();
    service.close().await;
}
