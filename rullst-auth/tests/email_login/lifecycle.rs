use super::support::*;

pub async fn run(url: &str) {
    let namespace = unique();
    let clock = Clock::new();
    let service = EmailLoginService::initialize(url, keys(), config(&namespace))
        .await
        .unwrap();
    let (subject, email, password) = account(&service, &clock).await;
    let browser = BrowserBinding::generate().unwrap();
    assert_eq!(
        service
            .request_login("absent@example.com", &browser, &clock)
            .await
            .unwrap(),
        LoginRequestAccepted
    );
    assert_eq!(
        service
            .request_login("malformed", &browser, &clock)
            .await
            .unwrap(),
        LoginRequestAccepted
    );
    assert!(service.claim_notice(&clock).await.unwrap().is_none());
    let first = issue(&service, &email, &browser, &clock).await;
    let first_token = token(&first);
    assert_eq!(first.recipient(), email);
    assert_eq!(first.expires_at(), clock.now().unwrap() as i64 + 900);
    assert!(!first.expose_link().contains(browser.expose_cookie()));
    assert!(!format!("{first:?}").contains(&email));
    assert!(matches!(
        service
            .redeem(&first_token, &BrowserBinding::generate().unwrap(), &clock)
            .await,
        Err(RecoveryError::InvalidAction)
    ));
    let other_namespace = EmailLoginService::initialize(url, keys(), config(&unique()))
        .await
        .unwrap();
    assert!(
        other_namespace
            .redeem(&first_token, &browser, &clock)
            .await
            .is_err()
    );
    assert!(
        other_namespace
            .complete_notice(&first, &clock)
            .await
            .is_err()
    );
    other_namespace.close().await;
    // Replacement removes leased mail and rejects a stale worker's acknowledgement.
    let second = issue(&service, &email, &browser, &clock).await;
    assert!(service.complete_notice(&first, &clock).await.is_err());
    assert!(
        service
            .redeem(&first_token, &browser, &clock)
            .await
            .is_err()
    );
    let valid = token(&second);
    service.complete_notice(&second, &clock).await.unwrap();
    let peer = EmailLoginService::connect(url, keys(), config(&namespace))
        .await
        .unwrap();
    // One winner across independent connection pools, including SQLite file locks.
    let (one, two) = tokio::join!(
        service.redeem(&valid, &browser, &clock),
        peer.redeem(&valid, &browser, &clock)
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    let session = one.or(two).unwrap();
    assert_eq!(session.subject(), subject);
    assert_eq!(session.destination(), "/dashboard");
    assert_eq!(
        peer.accounts()
            .verify_session(session.token().expose(), clock.now().unwrap())
            .await
            .unwrap(),
        Some(subject.clone())
    );
    assert_eq!(
        peer.accounts()
            .active_sessions(session.token().expose(), clock.now().unwrap())
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(peer.redeem(&valid, &browser, &clock).await.is_err());
    assert!(!format!("{session:?}").contains(session.token().expose()));
    peer.accounts()
        .revoke_session(session.token().expose())
        .await
        .unwrap();
    assert!(
        service
            .accounts()
            .verify_session(session.token().expose(), clock.now().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    let expired = issue(&service, &email, &browser, &clock).await;
    clock.advance(900);
    assert!(
        peer.redeem(&token(&expired), &browser, &clock)
            .await
            .is_err()
    );
    assert!(peer.claim_notice(&clock).await.unwrap().is_none());
    // Account opt-out cancels pending mail and invalidates a previously delivered link.
    let pending = issue(&service, &email, &browser, &clock).await;
    let proof = service
        .accounts()
        .authenticate(&email, &password)
        .await
        .unwrap()
        .unwrap();
    service
        .set_account_enabled(&proof, false, &clock)
        .await
        .unwrap();
    assert!(
        peer.redeem(&token(&pending), &browser, &clock)
            .await
            .is_err()
    );
    assert_eq!(
        peer.request_login(&email, &browser, &clock).await.unwrap(),
        LoginRequestAccepted
    );
    assert!(peer.claim_notice(&clock).await.unwrap().is_none());
    // A proof from another pool cannot opt in this service's account policy.
    assert_eq!(
        peer.set_account_enabled(&proof, true, &clock).await,
        Err(RecoveryError::InvalidAction)
    );
    // Restart retains policy, clock observations and consumed-link state.
    service.close().await;
    peer.close().await;
    let reopened = EmailLoginService::connect(url, keys(), config(&namespace))
        .await
        .unwrap();
    assert!(reopened.redeem(&valid, &browser, &clock).await.is_err());
    clock.set(1_800_000_000);
    assert!(
        reopened
            .request_login(&email, &browser, &clock)
            .await
            .is_err()
    );
    reopened.close().await;
    assert!(
        EmailLoginService::connect(
            url,
            RecoverySecrets::new([7; 32], [8; 32]).unwrap(),
            config(&namespace)
        )
        .await
        .is_err()
    );
    let mismatch =
        EmailLoginConfig::new(&namespace, "https://app.example/changed", "/dashboard", 20).unwrap();
    assert!(
        EmailLoginService::connect(url, keys(), mismatch)
            .await
            .is_err()
    );
}
