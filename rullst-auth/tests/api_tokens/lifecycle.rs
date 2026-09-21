use super::support::*;

pub async fn run(url: &str) {
    let namespace = unique();
    let clock = Clock::new();
    let service = ApiTokenService::initialize(url, keys(), config(&namespace))
        .await
        .unwrap();
    let (owner, email, password) = account(&service, &clock).await;
    let (foreign, _, _) = account(&service, &clock).await;
    let token = service
        .issue(&owner, read(), label(), 600, &clock)
        .await
        .unwrap();
    let id = token.metadata().id().clone();
    assert!(token.expose_bearer().starts_with("rlt1_"));
    assert_eq!(token.expose_bearer().len(), 92);
    let bearer = token.expose_bearer();
    let changed = if bearer.as_bytes()[49] == b'A' {
        'B'
    } else {
        'A'
    };
    let forged = format!("{}{changed}{}", &bearer[..49], &bearer[50..]);
    for malformed in ["", "rlt1_invalid", &forged, &format!("{bearer}=")] {
        assert!(service.verify(malformed, &read(), &clock).await.is_err());
    }
    assert!(!format!("{token:?}").contains(token.expose_bearer()));
    assert_eq!(token.metadata().created_at(), clock.now().unwrap());
    assert_eq!(token.metadata().issued_at(), clock.now().unwrap());
    assert_eq!(token.metadata().expires_at(), clock.now().unwrap() + 600);
    assert_eq!(token.metadata().revision(), 1);
    let principal = service
        .verify(token.expose_bearer(), &read(), &clock)
        .await
        .unwrap();
    assert_eq!(principal.subject(), owner.subject());
    assert_eq!(principal.namespace(), namespace);
    assert!(!format!("{principal:?}").contains(owner.subject()));
    assert!(
        service
            .verify(token.expose_bearer(), &scopes(), &clock)
            .await
            .is_err()
    );
    assert!(
        service
            .issue(
                &owner,
                ApiScopes::new(["admin:all"]).unwrap(),
                label(),
                600,
                &clock
            )
            .await
            .is_err()
    );
    assert!(
        service
            .issue(&owner, read(), label(), 3601, &clock)
            .await
            .is_err()
    );
    assert!(service.verify(id.as_str(), &read(), &clock).await.is_err());
    let session = service
        .accounts()
        .create_session(&owner, clock.now().unwrap(), 3600)
        .await
        .unwrap();
    assert!(
        service
            .verify(session.expose(), &read(), &clock)
            .await
            .is_err()
    );
    assert!(
        service
            .accounts()
            .verify_session(token.expose_bearer(), clock.now().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    let other = ApiTokenService::initialize(url, keys(), config(&unique()))
        .await
        .unwrap();
    assert!(
        other
            .verify(token.expose_bearer(), &read(), &clock)
            .await
            .is_err()
    );
    other.close().await;
    assert!(
        service
            .inventory(&foreign, &clock)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!service.revoke(&foreign, &id, &clock).await.unwrap());
    assert!(service.rotate(&foreign, &id, 1, 600, &clock).await.is_err());
    assert!(
        service
            .verify(token.expose_bearer(), &read(), &clock)
            .await
            .is_ok()
    );
    let peer = ApiTokenService::connect(url, keys(), config(&namespace))
        .await
        .unwrap();
    assert!(peer.inventory(&owner, &clock).await.is_err());
    let peer_owner = peer
        .accounts()
        .authenticate(&email, &password)
        .await
        .unwrap()
        .unwrap();
    clock.advance(1);
    let (first, second) = tokio::join!(
        service.rotate(&owner, &id, 1, 600, &clock),
        peer.rotate(&peer_owner, &id, 1, 600, &clock)
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let rotated = first.or(second).unwrap();
    assert_eq!(rotated.metadata().id(), &id);
    assert_eq!(rotated.metadata().revision(), 2);
    assert_eq!(
        rotated.metadata().created_at(),
        token.metadata().created_at()
    );
    assert_eq!(rotated.metadata().issued_at(), clock.now().unwrap());
    assert_ne!(rotated.expose_bearer(), token.expose_bearer());
    assert!(
        peer.verify(token.expose_bearer(), &read(), &clock)
            .await
            .is_err()
    );
    assert!(
        service
            .verify(rotated.expose_bearer(), &read(), &clock)
            .await
            .is_ok()
    );
    assert_eq!(peer.inventory(&peer_owner, &clock).await.unwrap().len(), 1);
    let (racing_rotation, revocation) = tokio::join!(
        service.rotate(&owner, &id, 2, 600, &clock),
        peer.revoke(&peer_owner, &id, &clock)
    );
    assert!(revocation.unwrap());
    match racing_rotation {
        Ok(token) => {
            assert!(
                peer.verify(token.expose_bearer(), &read(), &clock)
                    .await
                    .is_err()
            );
        }
        Err(error) => assert_eq!(error, RecoveryError::InvalidAction),
    }
    assert!(!peer.revoke(&peer_owner, &id, &clock).await.unwrap());
    assert!(
        service
            .verify(rotated.expose_bearer(), &read(), &clock)
            .await
            .is_err()
    );
    assert!(service.rotate(&owner, &id, 2, 600, &clock).await.is_err());
    let obsolete = service
        .issue(&owner, read(), label(), 600, &clock)
        .await
        .unwrap();
    service
        .accounts()
        .revoke_other_sessions(session.expose(), clock.now().unwrap())
        .await
        .unwrap();
    assert!(
        peer.verify(obsolete.expose_bearer(), &read(), &clock)
            .await
            .is_err()
    );
    assert!(
        service
            .issue(&owner, read(), label(), 600, &clock)
            .await
            .is_err()
    );
    let owner = service
        .accounts()
        .authenticate(&email, &password)
        .await
        .unwrap()
        .unwrap();
    let expiring = service
        .issue(&owner, read(), label(), 60, &clock)
        .await
        .unwrap();
    clock.advance(60);
    assert!(
        service
            .verify(expiring.expose_bearer(), &read(), &clock)
            .await
            .is_err()
    );
    let active = service
        .issue(&owner, read(), label(), 600, &clock)
        .await
        .unwrap();
    service.close().await;
    peer.close().await;
    let reopened = ApiTokenService::connect(url, keys(), config(&namespace))
        .await
        .unwrap();
    assert!(
        reopened
            .verify(active.expose_bearer(), &read(), &clock)
            .await
            .is_ok()
    );
    assert!(
        reopened
            .verify(rotated.expose_bearer(), &read(), &clock)
            .await
            .is_err()
    );
    clock.set(1_800_000_000);
    assert!(
        reopened
            .verify(active.expose_bearer(), &read(), &clock)
            .await
            .is_err()
    );
    reopened.close().await;
    let changed = ApiTokenConfig::new(&namespace, read(), 100, 3600).unwrap();
    assert!(
        ApiTokenService::connect(url, keys(), changed)
            .await
            .is_err()
    );
    assert!(
        ApiTokenService::connect(
            url,
            RecoverySecrets::new([7; 32], [8; 32]).unwrap(),
            config(&namespace)
        )
        .await
        .is_err()
    );
}
