//! The registry contract every database must pass, through the public API
//! only: sign-in, a copied cookie after sign-out, revoking one session, the
//! other sessions and all of them, expiry and the session list.

use rullst_auth::session_registry::{
    Browser, DeviceSummary, IssuedSession, NewSession, Platform, SessionDatabase, SessionRef,
    SessionRegistry, SessionRegistryError,
};
use std::time::Duration;

pub const KEY: &[u8; 32] = b"0123456789abcdefghijklmnopqrstuv";
/// 2026-11-15T10:00:00Z, the start of an hour.
pub const HOUR: u64 = 1_794_736_800;
pub const NOW: u64 = HOUR + 600;

async fn login<DB: SessionDatabase>(
    registry: &SessionRegistry<DB>,
    user_id: i32,
    device: DeviceSummary,
    now: u64,
) -> IssuedSession {
    registry
        .create_session(&NewSession::new(user_id, 4).with_device(device), KEY, now)
        .await
        .unwrap()
}

/// Runs the contract on an empty registry. Accounts 101 to 104 are used.
pub async fn run<DB: SessionDatabase>(registry: &SessionRegistry<DB>) {
    let laptop = DeviceSummary::new(Browser::Firefox, Platform::Linux);
    let phone = DeviceSummary::new(Browser::Safari, Platform::Ios);
    let tablet = DeviceSummary::new(Browser::Chrome, Platform::Android);

    // Sign-in and authentication.
    let first = login(registry, 101, laptop, NOW).await;
    let session = registry
        .authenticate(first.token(), KEY, NOW)
        .await
        .unwrap();
    assert_eq!(session.user_id(), 101);
    assert_eq!(session.claims().epoch, 4);
    assert_eq!(session.claims().expires_at, NOW + 2_592_000);
    assert_eq!(session.current().user_id(), 101);
    assert_eq!(session.current().reference(), first.reference());
    assert_eq!(session.current().created_at(), NOW);
    assert_eq!(session.current().expires_at(), NOW + 2_592_000);

    // Sign-out revokes every copy of the cookie.
    let copied = first.token().to_owned();
    assert_eq!(registry.revoke(session.current()).await, Ok(true));
    assert_eq!(registry.revoke(session.current()).await, Ok(false));
    for cookie in [first.token(), copied.as_str()] {
        let refused = registry
            .authenticate(cookie, KEY, NOW + 1)
            .await
            .unwrap_err();
        assert_eq!(refused, SessionRegistryError::Revoked);
        assert!(refused.is_signed_out());
    }

    // Signing out by the cookie value, expired or not.
    let short = NewSession::new(101, 4).with_max_age(Duration::from_secs(5));
    let expiring = registry.create_session(&short, KEY, NOW).await.unwrap();
    assert!(
        registry
            .authenticate(expiring.token(), KEY, NOW + 5)
            .await
            .is_ok()
    );
    assert_eq!(
        registry
            .authenticate(expiring.token(), KEY, NOW + 6)
            .await
            .err(),
        Some(SessionRegistryError::Expired)
    );
    assert_eq!(registry.revoke_token(expiring.token(), KEY).await, Ok(true));
    assert_eq!(
        registry.revoke_token(expiring.token(), KEY).await,
        Ok(false)
    );
    assert_eq!(
        registry.revoke_token("not a cookie", KEY).await,
        Err(SessionRegistryError::InvalidToken)
    );

    // Three devices: the list, newest use first, and revoking one.
    let on_laptop = login(registry, 102, laptop, NOW).await;
    let on_phone = login(registry, 102, phone, NOW + 1).await;
    let on_tablet = login(registry, 102, tablet, NOW + 2).await;
    let elsewhere = login(registry, 103, laptop, NOW).await;
    let current = registry
        .authenticate(on_phone.token(), KEY, HOUR + 3_600)
        .await
        .unwrap()
        .into_current();
    let listed = registry.list_sessions(102, HOUR + 3_600).await.unwrap();
    let references: Vec<&SessionRef> = listed.iter().map(|info| info.reference()).collect();
    assert_eq!(
        references,
        vec![
            on_phone.reference(),
            on_tablet.reference(),
            on_laptop.reference()
        ]
    );
    assert_eq!(
        listed.iter().map(|info| info.device()).collect::<Vec<_>>(),
        vec![phone, tablet, laptop]
    );
    assert_eq!(listed[0].last_seen_at(), HOUR + 3_600);
    assert_eq!(listed[1].last_seen_at(), HOUR);
    assert_eq!(listed[1].created_at(), NOW + 2);
    assert_eq!(listed[1].expires_at(), NOW + 2 + 2_592_000);
    assert_eq!(
        listed
            .iter()
            .map(|info| info.is_current(&current))
            .collect::<Vec<_>>(),
        vec![true, false, false]
    );

    // Another account's reference revokes nothing.
    assert_eq!(
        registry.revoke_session(103, on_tablet.reference()).await,
        Ok(false)
    );
    assert_eq!(
        registry.revoke_session(102, elsewhere.reference()).await,
        Ok(false)
    );
    assert_eq!(
        registry.revoke_session(102, on_tablet.reference()).await,
        Ok(true)
    );
    assert_eq!(
        registry.revoke_session(102, on_tablet.reference()).await,
        Ok(false)
    );
    assert_eq!(
        registry
            .authenticate(on_tablet.token(), KEY, NOW)
            .await
            .err(),
        Some(SessionRegistryError::Revoked)
    );
    assert!(
        registry
            .authenticate(elsewhere.token(), KEY, NOW)
            .await
            .is_ok()
    );

    // Sign out of every other device keeps this one.
    let again = login(registry, 102, tablet, NOW + 3).await;
    assert_eq!(registry.revoke_other_sessions(&current).await, Ok(2));
    assert_eq!(registry.revoke_other_sessions(&current).await, Ok(0));
    for revoked in [&on_laptop, &again] {
        assert_eq!(
            registry.authenticate(revoked.token(), KEY, NOW).await.err(),
            Some(SessionRegistryError::Revoked)
        );
    }
    assert!(
        registry
            .authenticate(on_phone.token(), KEY, NOW)
            .await
            .is_ok()
    );
    assert!(
        registry
            .authenticate(elsewhere.token(), KEY, NOW)
            .await
            .is_ok()
    );
    assert_eq!(registry.list_sessions(102, NOW).await.unwrap().len(), 1);

    // A password reset signs out every session of the account only.
    login(registry, 102, laptop, NOW + 4).await;
    assert_eq!(registry.revoke_all_sessions(102).await, Ok(2));
    assert_eq!(registry.revoke_all_sessions(102).await, Ok(0));
    assert_eq!(
        registry
            .authenticate(on_phone.token(), KEY, NOW)
            .await
            .err(),
        Some(SessionRegistryError::Revoked)
    );
    assert!(registry.list_sessions(102, NOW).await.unwrap().is_empty());
    assert!(
        registry
            .authenticate(elsewhere.token(), KEY, NOW)
            .await
            .is_ok()
    );

    // Expired sessions leave the list at once and the table when purged.
    let brief = NewSession::new(104, 0).with_max_age(Duration::from_secs(60));
    registry.create_session(&brief, KEY, NOW).await.unwrap();
    assert_eq!(
        registry.list_sessions(104, NOW + 60).await.unwrap().len(),
        1
    );
    assert!(
        registry
            .list_sessions(104, NOW + 61)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(registry.purge_expired(NOW + 60, 10).await, Ok(0));
    assert_eq!(registry.purge_expired(NOW + 61, 10).await, Ok(1));
    assert!(
        registry
            .authenticate(elsewhere.token(), KEY, NOW + 61)
            .await
            .is_ok()
    );

    // A cookie sealed with another key is not a session.
    assert_eq!(
        registry
            .authenticate(elsewhere.token(), b"vutsrqponmlkjihgfedcba9876543210", NOW)
            .await
            .err(),
        Some(SessionRegistryError::InvalidToken)
    );
    assert_eq!(registry.revoke_all_sessions(103).await, Ok(1));
}
