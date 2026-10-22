//! The generated sign-in with Google, GitHub, Microsoft and Apple in one
//! project, through the real generated router, migrations, account model and
//! session cookies, with Rullst Connect's offline providers (`mock_*`
//! credentials, `oauth-mock` feature).
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

/// The providers of this project and the subject their offline mock signs in.
const PROVIDERS: [(&str, &str); 4] = [
    ("google", "mock-user"),
    ("github", "1"),
    ("microsoft", "00000000-0000-0000-0000-00000000000a:00000000-0000-0000-0000-000000000001"),
    ("apple", "mock-user"),
];

/// Starts a sign-in: the challenge cookie and the callback the offline
/// provider sends the browser to.
async fn begin_with(app: &App, provider: &str, next: Option<&str>) -> (String, String) {
    let uri = next.map_or(format!("/auth/{provider}"), |next| format!("/auth/{provider}?next={next}"));
    let started = get(app, &uri, &[]).await;
    assert_eq!(started.status(), StatusCode::SEE_OTHER, "{provider}");
    let challenge = set_cookie(&started, "rullst_oauth_challenge").unwrap();
    assert!(challenge.ends_with(&format!("; Path=/auth/{provider}; Max-Age=600; HttpOnly; SameSite=Lax")), "{challenge}");
    let id = pair(&challenge);
    // 256 random bits as 43 base64url characters, and nothing else.
    assert_eq!(id.len(), "rullst_oauth_challenge=".len() + 43);
    let callback = location(&started)
        .strip_prefix("http://localhost:3000")
        .unwrap()
        .to_owned();
    assert!(callback.starts_with(&format!("/auth/{provider}/callback?code=mock_authorization_code&")));
    assert!(callback.contains("&state="));
    // GitHub is OAuth 2.0 only; the others are OpenID Connect with a nonce.
    assert_eq!(callback.contains("&nonce="), provider != "github", "{provider}");
    assert!(!callback.contains(id.split('=').nth(1).unwrap()));
    (id, callback)
}

async fn begin(app: &App, next: Option<&str>) -> (String, String) {
    begin_with(app, "google", next).await
}

/// The session token of a successful callback, after checking its redirect.
fn signed_in(response: &Response, target: &str) -> String {
    signed_in_with(response, "google", target)
}

fn signed_in_with(response: &Response, provider: &str, target: &str) -> String {
    assert_eq!(response.status(), StatusCode::SEE_OTHER, "{provider}");
    assert_eq!(location(response), target);
    let cleared = set_cookie(response, "rullst_oauth_challenge").unwrap();
    assert!(cleared.starts_with(&format!("rullst_oauth_challenge=; Path=/auth/{provider}; Max-Age=0;")));
    let session = set_cookie(response, "rullst_session").unwrap();
    pair(&session).trim_start_matches("rullst_session=").to_owned()
}

/// A form POST from another site: no CSRF token, only the challenge cookie.
async fn post_form(app: &App, uri: &str, cookies: &[&str], form: &str) -> Response {
    let n = NEXT_PEER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let peer = std::net::SocketAddr::from(([10, 1, (n >> 8) as u8, n as u8], 40_000));
    let mut request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("origin", "https://appleid.apple.com")
        .header("cookie", cookies.join("; "))
        .body(Body::from(form.to_owned()))
        .unwrap();
    request
        .extensions_mut()
        .insert(rullst::web::axum::extract::ConnectInfo(peer));
    app.clone().oneshot(request).await.unwrap()
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
    // `Server` loads Rullst.toml; this contract calls the router directly.
    let config = std::fs::read_to_string("Rullst.toml").unwrap();
    let config = rullst::config::RullstConfig::from_toml(&config).unwrap();
    assert!(config.security.csrf_signed_webhook_paths.contains(&"/auth/apple/callback".to_owned()));
    assert!(rullst::config::RullstConfig::set_global(config).is_ok());
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
    for (provider, _) in PROVIDERS {
        assert!(page.contains(&format!("href=\"/auth/{provider}\"")), "{provider}");
    }

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

    // Every other provider: the same challenge, refusals and account
    // policy, in one oauth_identities table keyed by provider and subject.
    for (provider, subject) in &PROVIDERS[1..] {
        let (provider, subject) = (*provider, *subject);
        // The mock's verified email belongs to the Google account: refused
        // by default, and the challenge is consumed anyway.
        let (challenge, callback) = begin_with(app, provider, None).await;
        refused(&get(app, &callback, &[&challenge]).await, StatusCode::CONFLICT);
        refused(&get(app, &callback, &[&challenge]).await, StatusCode::BAD_REQUEST);
        assert_eq!(OAuthIdentity::user_id(provider, subject).await.unwrap(), None);
        // A changed state is refused.
        let (challenge, callback) = begin_with(app, provider, None).await;
        let state = callback.split("&state=").nth(1).unwrap().split('&').next().unwrap();
        let forged = callback.replace(state, &"x".repeat(64));
        refused(&get(app, &forged, &[&challenge]).await, StatusCode::BAD_REQUEST);
        // Never trust an email the provider did not verify.
        for verified in [Some(false), None] {
            let unverified = profile(subject, "new@example.test", verified);
            assert!(matches!(
                resolve_account(provider, &unverified, ExistingAccountPolicy::Refuse).await,
                Err(AccountError::UnverifiedEmail)
            ));
        }
        // Linked under the explicit policy, the identity signs in to the
        // same account.
        let verified = profile(subject, "mock@example.invalid", Some(true));
        let linked = resolve_account(provider, &verified, ExistingAccountPolicy::LinkAndResetPassword)
            .await
            .unwrap();
        assert_eq!(linked.id, account.id);
        let (challenge, callback) = begin_with(app, provider, Some("/password/change")).await;
        let response = if provider == "apple" {
            // Apple POSTs the callback from its own site, without this
            // site's CSRF token; the first one also carries the user's name.
            let form = format!(
                "{}&user=%7B%22name%22%3A%7B%22firstName%22%3A%22Ana%22%7D%7D",
                callback.split_once('?').unwrap().1
            );
            post_form(app, "/auth/apple/callback", &[&challenge], &form).await
        } else {
            get(app, &callback, &[&challenge]).await
        };
        let token = signed_in_with(&response, provider, "/password/change");
        let claims = rullst::auth::decrypt_session_claims(&token, &key).unwrap();
        assert_eq!(claims.user_id, account.id);
        assert_eq!(OAuthIdentity::user_id(provider, subject).await.unwrap(), Some(account.id));
    }
    // A form POST without the challenge's state is refused like any other.
    let (challenge, _) = begin_with(app, "apple", None).await;
    let response = post_form(app, "/auth/apple/callback", &[&challenge], "code=mock_authorization_code").await;
    refused(&response, StatusCode::BAD_REQUEST);
    let identities: Vec<(String, String, i32)> =
        rullst::db::sqlx::query_as("SELECT provider, subject, user_id FROM oauth_identities ORDER BY provider")
            .fetch_all(rullst::orm::Orm::pool().unwrap())
            .await
            .unwrap();
    let mut expected: Vec<(String, String, i32)> = PROVIDERS
        .iter()
        .map(|(provider, subject)| ((*provider).to_owned(), (*subject).to_owned(), account.id))
        .collect();
    expected.sort();
    assert_eq!(identities, expected);
    assert_eq!(users().await, 1);

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

    // GitHub: both credentials or neither.
    assert!(github::configured(None, None, None, None, Environment::Production).unwrap().is_none());
    let error = github::configured(Some("client".to_owned()), None, None, None, Environment::Development)
        .err()
        .unwrap()
        .to_string();
    assert_eq!(error, "set both GITHUB_CLIENT_ID and GITHUB_CLIENT_SECRET, or neither");

    // Microsoft: the tenant is an explicit, valid choice.
    let microsoft = |tenant: Option<&str>, id: &str| {
        microsoft::configured(
            tenant.map(str::to_owned),
            Some(id.to_owned()),
            Some(id.to_owned()),
            None,
            None,
            Environment::Development,
        )
    };
    let tenant_required = "set MICROSOFT_TENANT to common, organizations, consumers or your directory (tenant) ID";
    for tenant in [None, Some("contoso.example")] {
        assert_eq!(microsoft(tenant, "client").err().unwrap().to_string(), tenant_required);
    }
    assert!(microsoft::configured(Some("common".to_owned()), None, None, None, None, Environment::Production).unwrap().is_none());
    let provider = microsoft(Some("72f988bf-86f1-41af-91ab-2d7cd011db47"), "client").unwrap().unwrap();
    assert_eq!(provider.tenant().as_str(), "72f988bf-86f1-41af-91ab-2d7cd011db47");

    // Apple: all four values or none, and a key that can sign.
    let apple = |values: [Option<&str>; 4], environment| {
        apple::configured(values.map(|value| value.map(str::to_owned)), None, Some("https://app.example".to_owned()), environment)
    };
    assert!(apple([None; 4], Environment::Production).unwrap().is_none());
    assert_eq!(
        apple([Some("com.example.web"), None, None, None], Environment::Development).err().unwrap().to_string(),
        "set all of APPLE_CLIENT_ID, APPLE_TEAM_ID, APPLE_KEY_ID and APPLE_PRIVATE_KEY, or none"
    );
    let error = apple([Some("com.example.web"), Some("TEAM123456"), Some("KEY1234567"), Some("not a key")], Environment::Production)
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("private_key_pem"), "{error}");
    let mock = apple([Some("mock_apple"); 4], Environment::Development);
    let mock_microsoft = microsoft(Some("common"), "mock_microsoft");
    let mock_github = github::configured(Some("mock_github".to_owned()), Some("mock_github".to_owned()), None, None, Environment::Development);
    if cfg!(feature = "oauth-mock") {
        assert!(mock.unwrap().is_some());
        assert!(mock_microsoft.unwrap().is_some());
        assert!(mock_github.unwrap().is_some());
        // Mock credentials never run in staging or production.
        let error = apple([Some("mock_apple"); 4], Environment::Staging).err().unwrap().to_string();
        assert!(error.contains("only in development and tests"), "{error}");
    } else {
        for error in [
            mock.err().unwrap().to_string(),
            mock_microsoft.err().unwrap().to_string(),
            mock_github.err().unwrap().to_string(),
        ] {
            assert!(error.contains("oauth-mock"), "{error}");
        }
    }
}
