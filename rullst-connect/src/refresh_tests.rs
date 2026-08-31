use super::*;
use crate::provider::ExchangeParams;
use async_trait::async_trait;
use secrecy::ExposeSecret;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Default)]
struct CountingProvider {
    refreshes: AtomicUsize,
    fail: AtomicBool,
    rotate: AtomicBool,
    wrong_user: AtomicBool,
    omit_expiry: AtomicBool,
}

#[async_trait]
impl Provider for CountingProvider {
    fn redirect_url(&self) -> String {
        "https://identity.example/authorize".to_string()
    }

    async fn get_user(&self, _params: ExchangeParams<'_>) -> Result<ConnectUser, ConnectError> {
        Err(ConnectError::Provider("unused test operation".to_string()))
    }

    async fn get_user_from_token(&self, _access_token: &str) -> Result<ConnectUser, ConnectError> {
        Err(ConnectError::Provider("unused test operation".to_string()))
    }

    fn token_url(&self) -> String {
        "https://identity.example/token".to_string()
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<ConnectUser, ConnectError> {
        assert!(matches!(refresh_token, "refresh-0" | "refresh-1"));
        if self.fail.load(Ordering::SeqCst) {
            return Err(ConnectError::Token("fixture refresh failed".to_string()));
        }
        let generation = self.refreshes.fetch_add(1, Ordering::SeqCst) + 1;
        let rotated = self
            .rotate
            .load(Ordering::SeqCst)
            .then(|| SecretString::from(format!("refresh-{generation}")));
        let mut refreshed = user(
            &format!("access-{generation}"),
            rotated,
            (!self.omit_expiry.load(Ordering::SeqCst)).then_some(3_600),
        );
        if self.wrong_user.load(Ordering::SeqCst) {
            refreshed.id = "user-other".to_string();
        }
        Ok(refreshed)
    }
}

#[test]
fn token_state_is_bounded_and_redacted() {
    let state = RefreshableTokenState::try_new(
        "user-1",
        SecretString::from("access-0".to_string()),
        SecretString::from("refresh-0".to_string()),
        1_000,
        3_600,
    )
    .expect("valid state");
    assert_eq!(state.issued_at(), 1_000);
    assert_eq!(state.expires_at(), 4_600);
    assert_eq!(state.generation(), 0);
    let debug = format!("{state:?}");
    assert!(!debug.contains("access-0"));
    assert!(!debug.contains("refresh-0"));

    assert!(
        RefreshableTokenState::try_new(
            "user-1",
            SecretString::from(String::new()),
            SecretString::from("refresh".to_string()),
            0,
            1,
        )
        .is_err()
    );
    assert!(
        RefreshableTokenState::try_new(
            "user-1",
            SecretString::from("access".to_string()),
            SecretString::from("refresh".to_string()),
            0,
            0,
        )
        .is_err()
    );
}

#[tokio::test]
async fn fresh_token_is_returned_without_provider_call() {
    let provider = CountingProvider::default();
    let session = AutoRefreshingSession::from_user_at(
        &provider,
        &user(
            "access-0",
            Some(SecretString::from("refresh-0".to_string())),
            Some(3_600),
        ),
        1_000,
    )
    .expect("session");

    let lease = session.access_token_at(2_000).await.expect("fresh lease");
    assert_eq!(lease.access_token().expose_secret(), "access-0");
    assert_eq!(lease.generation(), 0);
    assert!(!lease.was_refreshed());
    assert_eq!(provider.refreshes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn concurrent_expiry_performs_one_refresh_and_rotates_state() {
    let provider = CountingProvider::default();
    provider.rotate.store(true, Ordering::SeqCst);
    let session = AutoRefreshingSession::from_user_at(
        &provider,
        &user(
            "access-0",
            Some(SecretString::from("refresh-0".to_string())),
            Some(100),
        ),
        1_000,
    )
    .expect("session")
    .with_refresh_leeway(10)
    .expect("leeway");

    let (first, second, third) = tokio::join!(
        session.access_token_at(1_095),
        session.access_token_at(1_095),
        session.access_token_at(1_095),
    );
    let leases = [first, second, third].map(|result| result.expect("lease"));
    assert_eq!(provider.refreshes.load(Ordering::SeqCst), 1);
    assert_eq!(
        leases.iter().filter(|lease| lease.was_refreshed()).count(),
        1
    );
    assert!(leases.iter().all(|lease| lease.generation() == 1));
    assert!(
        leases
            .iter()
            .all(|lease| lease.access_token().expose_secret() == "access-1")
    );

    let state = session.state_snapshot().await;
    assert_eq!(state.refresh_token().expose_secret(), "refresh-1");
    assert_eq!(state.generation(), 1);
}

#[tokio::test]
async fn failure_and_invalid_input_leave_prior_state_unchanged() {
    let provider = CountingProvider::default();
    provider.fail.store(true, Ordering::SeqCst);
    let session = AutoRefreshingSession::from_user_at(
        &provider,
        &user(
            "access-0",
            Some(SecretString::from("refresh-0".to_string())),
            Some(10),
        ),
        1_000,
    )
    .expect("session")
    .with_refresh_leeway(0)
    .expect("leeway");

    assert!(session.access_token_at(1_010).await.is_err());
    let state = session.state_snapshot().await;
    assert_eq!(state.access_token().expose_secret(), "access-0");
    assert_eq!(state.refresh_token().expose_secret(), "refresh-0");
    assert_eq!(state.generation(), 0);

    let missing_refresh = user("access", None, Some(60));
    assert!(AutoRefreshingSession::from_user_at(&provider, &missing_refresh, 0).is_err());
    let missing_expiry = user(
        "access",
        Some(SecretString::from("refresh".to_string())),
        None,
    );
    assert!(
        AutoRefreshingSession::from_user_at(&provider, &missing_expiry, 0)
            .expect_err("missing expiry")
            .to_string()
            .contains("lifetime")
    );
}

#[tokio::test]
async fn invalid_refresh_response_cannot_replace_bound_identity_or_state() {
    let provider = CountingProvider::default();
    provider.wrong_user.store(true, Ordering::SeqCst);
    let session = AutoRefreshingSession::from_user_at(
        &provider,
        &user(
            "access-0",
            Some(SecretString::from("refresh-0".to_string())),
            Some(10),
        ),
        1_000,
    )
    .expect("session")
    .with_refresh_leeway(0)
    .expect("leeway");

    assert!(session.access_token_at(1_010).await.is_err());
    let state = session.state_snapshot().await;
    assert_eq!(state.provider_user_id(), "user-1");
    assert_eq!(state.access_token().expose_secret(), "access-0");
    assert_eq!(state.generation(), 0);

    provider.wrong_user.store(false, Ordering::SeqCst);
    provider.omit_expiry.store(true, Ordering::SeqCst);
    assert!(session.access_token_at(1_010).await.is_err());
    let state = session.state_snapshot().await;
    assert_eq!(state.access_token().expose_secret(), "access-0");
    assert_eq!(state.generation(), 0);
}

fn user(
    access_token: &str,
    refresh_token: Option<SecretString>,
    expires_in: Option<u64>,
) -> ConnectUser {
    ConnectUser {
        id: "user-1".to_string(),
        name: "Test User".to_string(),
        email: Some("user@example.com".to_string()),
        email_verified: Some(true),
        avatar_url: None,
        raw_data: json!({}),
        access_token: SecretString::from(access_token.to_string()),
        refresh_token,
        expires_in,
    }
}
