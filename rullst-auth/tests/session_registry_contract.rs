//! The registry contract on SQLite, and the request helpers that read
//! `APP_KEY` and the clock themselves. This binary owns its process
//! environment: it sets `APP_KEY` and `RULLST_ENV` once.
#![cfg(feature = "session-registry-sqlite")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "session_registry/contract.rs"]
mod contract;

use axum::http::{HeaderMap, HeaderValue, header};
use rullst_auth::session_registry::{
    DeviceSummary, LAST_SEEN_GRANULARITY_SECONDS, MAX_PURGE_BATCH, MAX_SESSIONS_PER_USER,
    NewSession, SessionRef, SessionRegistry, SessionRegistryError, SessionRegistryFor, unix_now,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Once;

async fn registry() -> SessionRegistryFor<sqlx::SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    SessionRegistry::open(pool).await.unwrap()
}

static ENVIRONMENT: Once = Once::new();

fn environment() {
    ENVIRONMENT.call_once(|| {
        // SAFETY: set once, before any test of this binary reads them.
        unsafe {
            std::env::set_var("APP_KEY", "0123456789abcdefghijklmnopqrstuv");
            std::env::set_var("RULLST_ENV", "production");
        }
    });
}

fn request(set_cookie: &str, user_agent: Option<&str>) -> HeaderMap {
    let cookie = set_cookie.split(';').next().unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        HeaderValue::from_str(&format!("theme=dark; {cookie}")).unwrap(),
    );
    if let Some(user_agent) = user_agent {
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_str(user_agent).unwrap(),
        );
    }
    headers
}

#[tokio::test]
async fn sqlite_passes_the_registry_contract() {
    contract::run(&registry().await).await;
}

#[tokio::test]
async fn requests_sign_in_authenticate_and_sign_out_with_app_key() {
    environment();
    let registry = registry().await;
    let device = DeviceSummary::from_user_agent(
        "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0",
    );
    let set_cookie = registry
        .sign_in(&NewSession::new(9, 1).with_device(device))
        .await
        .unwrap();
    assert!(set_cookie.starts_with("rullst_session=v3."));
    assert!(set_cookie.ends_with("; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000; Secure"));
    let headers = request(&set_cookie, None);
    let session = registry.authenticate_request(&headers).await.unwrap();
    assert_eq!(session.user_id(), 9);
    let before = unix_now().unwrap();
    assert!(session.current().created_at() <= before);
    assert!(before > 1_794_000_000);
    let listed = registry.list_sessions(9, before).await.unwrap();
    assert_eq!(listed[0].device().to_string(), "Firefox on Linux");
    assert!(listed[0].last_seen_at() % LAST_SEEN_GRANULARITY_SECONDS == 0);

    // Sign-out: the copied cookie stops working.
    let copied = headers.clone();
    assert_eq!(registry.sign_out(&headers).await, Ok(true));
    assert_eq!(registry.sign_out(&headers).await, Ok(false));
    assert_eq!(
        registry.authenticate_request(&copied).await.err(),
        Some(SessionRegistryError::Revoked)
    );
    // Without the cookie.
    assert_eq!(registry.sign_out(&HeaderMap::new()).await, Ok(false));
    assert_eq!(
        registry.authenticate_request(&HeaderMap::new()).await.err(),
        Some(SessionRegistryError::InvalidToken)
    );
    // A stateless epoch cookie is not a registered session.
    let epoch =
        rullst_auth::make_login_cookie_with_epoch(9, 1, rullst_auth::DEFAULT_SESSION_MAX_AGE)
            .unwrap();
    assert_eq!(
        registry
            .authenticate_request(&request(&epoch, None))
            .await
            .err(),
        Some(SessionRegistryError::InvalidToken)
    );
    assert_eq!(
        registry.sign_out(&request(&epoch, None)).await,
        Err(SessionRegistryError::InvalidToken)
    );
}

#[tokio::test]
async fn issued_cookies_follow_the_secure_flag() {
    let registry = registry().await;
    let issued = registry
        .create_session(&NewSession::new(3, 0), contract::KEY, contract::NOW)
        .await
        .unwrap();
    let token = issued.token();
    assert_eq!(
        issued.set_cookie(false),
        format!("rullst_session={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000")
    );
    assert_eq!(
        issued.set_cookie(true),
        format!("rullst_session={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000; Secure")
    );
}

#[test]
fn references_parse_only_their_own_shape() {
    for valid in [
        "MqZPJoS6uzx38-Ajzc2Qag",
        "abcdefghijklmnopqrst_-",
        "0123456789ABCDEFGHIJKL",
    ] {
        assert_eq!(SessionRef::parse(valid).unwrap().as_str(), valid);
    }
    for invalid in [
        "",
        "MqZPJoS6uzx38-Ajzc2Qa",
        "MqZPJoS6uzx38-Ajzc2Qagg",
        "MqZPJoS6uzx38+Ajzc2Qag",
        "MqZPJoS6uzx38=Ajzc2Qag",
        "MqZPJoS6uzx38/Ajzc2Qag",
    ] {
        assert_eq!(
            SessionRef::parse(invalid).err(),
            Some(SessionRegistryError::InvalidInput("session reference")),
            "{invalid}"
        );
    }
    let first = SessionRef::parse("MqZPJoS6uzx38-Ajzc2Qag").unwrap();
    assert_eq!(first, SessionRef::parse("MqZPJoS6uzx38-Ajzc2Qag").unwrap());
    assert_ne!(first, SessionRef::parse("MqZPJoS6uzx38-Ajzc2Qah").unwrap());
    assert_eq!(
        format!("{first:?}"),
        "SessionRef(\"MqZPJoS6uzx38-Ajzc2Qag\")"
    );
}

#[test]
fn errors_say_which_ones_mean_signed_out() {
    use SessionRegistryError::*;
    for error in [InvalidToken, Expired, Revoked] {
        assert!(error.is_signed_out(), "{error}");
    }
    for error in [
        InvalidInput("limit"),
        Configuration,
        Crypto,
        Unavailable,
        Corrupt,
    ] {
        assert!(!error.is_signed_out(), "{error}");
    }
    assert_eq!(Unavailable.to_string(), "session storage is unavailable");
    assert_eq!(
        InvalidInput("limit").to_string(),
        "invalid session registry input: limit"
    );
}

#[test]
fn the_documented_limits_hold() {
    assert_eq!(MAX_SESSIONS_PER_USER, 50);
    assert_eq!(LAST_SEEN_GRANULARITY_SECONDS, 3_600);
    assert_eq!(MAX_PURGE_BATCH, 1_000);
}
