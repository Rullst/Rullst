//! The generated sign-in with Google through the real generated router,
//! migrations, account model and session cookies, with Rullst Connect's
//! offline provider (`mock_*` credentials, `oauth-mock` feature).
use super::*;
use crate::models::user::User;
use rullst::server::{Body, Request};
use tower::ServiceExt;

type App = rullst::web::axum::Router;

/// Each request comes from its own client, so the per-client sign-in budget
/// (10 a minute) applies only where a test asks for it.
static NEXT_PEER: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(1);

async fn get_from(app: &App, peer: std::net::SocketAddr, uri: &str, cookies: &[&str]) -> Response {
    let mut request = Request::builder().uri(uri);
    if !cookies.is_empty() {
        request = request.header("cookie", cookies.join("; "));
    }
    let mut request = request.body(Body::empty()).unwrap();
    request
        .extensions_mut()
        .insert(rullst::web::axum::extract::ConnectInfo(peer));
    app.clone().oneshot(request).await.unwrap()
}

async fn get(app: &App, uri: &str, cookies: &[&str]) -> Response {
    let n = NEXT_PEER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let peer = std::net::SocketAddr::from(([10, 0, (n >> 8) as u8, n as u8], 40_000));
    get_from(app, peer, uri, cookies).await
}

fn set_cookie(response: &Response, name: &str) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().to_owned())
        .find(|cookie| cookie.starts_with(&format!("{name}=")))
}

fn pair(cookie: &str) -> String {
    cookie.split(';').next().unwrap().to_owned()
}

fn location(response: &Response) -> String {
    response.headers()["location"].to_str().unwrap().to_owned()
}

/// Starts a sign-in: the challenge cookie and the callback the offline
/// provider sends the browser to.
async fn begin(app: &App, next: Option<&str>) -> (String, String) {
    let uri = next.map_or("/auth/google".to_owned(), |next| format!("/auth/google?next={next}"));
    let started = get(app, &uri, &[]).await;
    assert_eq!(started.status(), StatusCode::SEE_OTHER);
    let challenge = set_cookie(&started, "rullst_oauth_challenge").unwrap();
    assert!(challenge.ends_with("; Path=/auth/google; Max-Age=600; HttpOnly; SameSite=Lax"));
    let id = pair(&challenge);
    // 256 random bits as 43 base64url characters, and nothing else.
    assert_eq!(id.len(), "rullst_oauth_challenge=".len() + 43);
    let callback = location(&started)
        .strip_prefix("http://localhost:3000")
        .unwrap()
        .to_owned();
    assert!(callback.starts_with("/auth/google/callback?code=mock_authorization_code&"));
    assert!(callback.contains("&state=") && callback.contains("&nonce="));
    assert!(!callback.contains(id.split('=').nth(1).unwrap()));
    (id, callback)
}

/// The session token of a successful callback, after checking its redirect.
fn signed_in(response: &Response, target: &str) -> String {
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(response), target);
    let cleared = set_cookie(response, "rullst_oauth_challenge").unwrap();
    assert!(cleared.starts_with("rullst_oauth_challenge=; Path=/auth/google; Max-Age=0;"));
    let session = set_cookie(response, "rullst_session").unwrap();
    pair(&session).trim_start_matches("rullst_session=").to_owned()
}

fn refused(response: &Response, status: StatusCode) {
    assert_eq!(response.status(), status);
    assert!(set_cookie(response, "rullst_session").is_none());
}

async fn user(email: &str) -> User {
    User::find_by_email(email).await.unwrap().unwrap()
}

async fn users() -> i64 {
    let (count,): (i64,) = rullst::db::sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(rullst::orm::Orm::pool().unwrap())
        .await
        .unwrap();
    count
}

fn profile(subject: &str, email: &str, verified: Option<bool>) -> ConnectUser {
    ConnectUser {
        id: subject.to_owned(),
        name: "Test Person".to_owned(),
        email: Some(email.to_owned()),
        email_verified: verified,
        avatar_url: None,
        raw_data: serde_json::Value::Null,
        access_token: rullst::connect::prelude::SecretString::from(String::new()),
        refresh_token: None,
        expires_in: None,
    }
}

#[tokio::test]
async fn generated_oauth_flow_contract() {
    rullst::orm::Orm::init("sqlite::memory:").await.unwrap();
    for migration in crate::migrations::get_migrations() {
        migration.up().await.unwrap();
    }
    let key = rullst::auth::get_app_key().unwrap();
    let router = crate::router().unwrap();
    let app = router.as_axum();
    let login = get(app, "/login", &[]).await;
    let page = String::from_utf8(
        rullst::web::axum::body::to_bytes(login.into_body(), 1 << 20).await.unwrap().to_vec(),
    )
    .unwrap();
    assert!(page.contains("href=\"/auth/google\""));

    // A first sign-in creates a password-less account linked to the subject,
    // and its session cookie carries the account's current epoch.
    let (challenge, callback) = begin(app, None).await;
    let response = get(app, &callback, &[&challenge]).await;
    let token = signed_in(&response, "/dashboard");
    let account = user("mock@example.invalid").await;
    assert_eq!(account.password_hash, None);
    assert_eq!(account.oauth_provider.as_deref(), Some("google"));
    assert_eq!(account.session_epoch, 0);
    let claims = rullst::auth::decrypt_session_claims(&token, &key).unwrap();
    assert_eq!(claims.user_id, account.id);
    assert_eq!(claims.epoch, 0);
    assert!(account.accepts_session(&claims));
    assert_eq!(
        OAuthIdentity::user_id("google", "mock-user").await.unwrap(),
        Some(account.id)
    );
    let session = format!("rullst_session={token}");
    assert_eq!(get(app, "/dashboard", &[&session]).await.status(), StatusCode::OK);

    // The same callback again finds no challenge.
    refused(&get(app, &callback, &[&challenge]).await, StatusCode::BAD_REQUEST);

    // A linked identity signs in to the same account, at a requested page.
    User::revoke_sessions(account.id).await.unwrap();
    let (challenge, callback) = begin(app, Some("/password/change")).await;
    let token = signed_in(&get(app, &callback, &[&challenge]).await, "/password/change");
    let claims = rullst::auth::decrypt_session_claims(&token, &key).unwrap();
    assert_eq!((claims.user_id, claims.epoch), (account.id, 1));
    assert_eq!(users().await, 1);
    // The cookie issued before the revocation no longer counts.
    assert_eq!(get(app, "/dashboard", &[&session]).await.status(), StatusCode::SEE_OTHER);
    // Another site is never a return page.
    let (challenge, callback) = begin(app, Some("//evil.example/")).await;
    signed_in(&get(app, &callback, &[&challenge]).await, "/dashboard");

    // A changed state is refused, and the challenge is consumed anyway.
    let (challenge, callback) = begin(app, None).await;
    let state = callback.split("&state=").nth(1).unwrap().split('&').next().unwrap();
    let forged = callback.replace(state, &"x".repeat(64));
    refused(&get(app, &forged, &[&challenge]).await, StatusCode::BAD_REQUEST);
    refused(&get(app, &callback, &[&challenge]).await, StatusCode::BAD_REQUEST);

    // Without the cookie, or with another id, nothing is consumed.
    let (challenge, callback) = begin(app, None).await;
    refused(&get(app, &callback, &[]).await, StatusCode::BAD_REQUEST);
    let other = format!("rullst_oauth_challenge={}", "A".repeat(43));
    refused(&get(app, &callback, &[&other]).await, StatusCode::BAD_REQUEST);
    signed_in(&get(app, &callback, &[&challenge]).await, "/dashboard");

    // Starting and finishing sign-in share the password login's budget.
    let peer = std::net::SocketAddr::from(([10, 9, 9, 9], 40_000));
    for _ in 0..10 {
        let started = get_from(app, peer, "/auth/google", &[]).await;
        assert_eq!(started.status(), StatusCode::SEE_OTHER);
    }
    let limited = get_from(app, peer, "/auth/google", &[]).await;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);

    // An account the password login refuses is refused here too.
    rullst::db::sqlx::query("UPDATE users SET session_epoch = -1 WHERE id = ?")
        .bind(account.id)
        .execute(rullst::orm::Orm::pool().unwrap())
        .await
        .unwrap();
    let (challenge, callback) = begin(app, None).await;
    refused(&get(app, &callback, &[&challenge]).await, StatusCode::FORBIDDEN);

    // Never trust an email the provider did not verify.
    for verified in [Some(false), None] {
        let unverified = profile("sub-unverified", "new@example.test", verified);
        assert!(matches!(
            resolve_account("google", &unverified, ExistingAccountPolicy::Refuse).await,
            Err(AccountError::UnverifiedEmail)
        ));
    }
    assert!(User::find_by_email("new@example.test").await.unwrap().is_none());

    // A verified email of a password account: refused by default; linking
    // removes the password and signs out every device.
    let mut owner = User {
        name: "Owner".to_owned(),
        email: "owner@example.test".to_owned(),
        password_hash: Some("$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ$aGFzaGhhc2g".to_owned()),
        created_at: crate::controllers::auth_controller::utc_timestamp(),
        updated_at: crate::controllers::auth_controller::utc_timestamp(),
        ..User::default()
    };
    owner.save().await.unwrap();
    let verified = profile("sub-owner", "Owner@Example.test", Some(true));
    assert!(matches!(
        resolve_account("google", &verified, ExistingAccountPolicy::Refuse).await,
        Err(AccountError::ExistingAccount)
    ));
    assert_eq!(OAuthIdentity::user_id("google", "sub-owner").await.unwrap(), None);
    let linked = resolve_account("google", &verified, ExistingAccountPolicy::LinkAndResetPassword)
        .await
        .unwrap();
    assert_eq!(linked.id, owner.id);
    assert_eq!(linked.password_hash, None);
    assert_eq!(linked.session_epoch, 1);
    assert_eq!(
        OAuthIdentity::user_id("google", "sub-owner").await.unwrap(),
        Some(owner.id)
    );
}

#[test]
fn generated_oauth_configuration_contract() {
    let mock = || {
        google::configured(
            Some("mock_google_client".to_owned()),
            Some("mock_google_secret".to_owned()),
            None,
            None,
            Environment::Development,
        )
    };
    if cfg!(feature = "oauth-mock") {
        assert!(mock().unwrap().is_some());
    } else {
        // Without the feature, mock credentials stop the application at startup.
        let error = mock().err().unwrap().to_string();
        assert!(error.contains("oauth-mock"), "{error}");
    }
    assert!(google::configured(None, None, None, None, Environment::Production).unwrap().is_none());
    for (id, secret) in [(Some("client"), None), (None, Some("secret"))] {
        let error = google::configured(id.map(str::to_owned), secret.map(str::to_owned), None, None, Environment::Development)
            .err()
            .unwrap()
            .to_string();
        assert_eq!(error, "set both GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET, or neither");
    }
    // Staging and production never guess the callback origin.
    let error = google::configured(Some("client".to_owned()), Some("secret".to_owned()), None, None, Environment::Production)
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("RULLST_PUBLIC_ORIGIN"), "{error}");
    assert_eq!(
        redirect_url("google", None, Some("https://app.example/".to_owned()), Environment::Production).unwrap(),
        "https://app.example/auth/google/callback"
    );
    assert_eq!(
        redirect_url("google", Some("https://app.example/cb".to_owned()), None, Environment::Production).unwrap(),
        "https://app.example/cb"
    );
    assert_eq!(
        redirect_url("google", None, None, Environment::Test).unwrap(),
        "http://localhost:3000/auth/google/callback"
    );
    if cfg!(feature = "oauth-mock") {
        let error = google::configured(
            Some("mock_google_client".to_owned()),
            Some("mock_google_secret".to_owned()),
            None,
            Some("https://app.example".to_owned()),
            Environment::Staging,
        )
        .err()
        .unwrap()
        .to_string();
        assert!(error.contains("only in development and tests"), "{error}");
    }
}
